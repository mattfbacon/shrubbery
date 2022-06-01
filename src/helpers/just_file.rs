use std::io::Seek as _;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::extract::{FromRequest, RequestParts};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use headers::HeaderMapExt as _;
use http::StatusCode;
use http_body::Body;
use tokio::io::ReadBuf;

use crate::timestamp::Timestamp;

#[derive(Debug, thiserror::Error)]
pub enum ParamsRejection {
	#[error("invalid {0} header")]
	Invalid(&'static str),
}
crate::error::impl_response!(ParamsRejection, BAD_REQUEST);

pub struct Params {
	range: Option<headers::Range>,
	if_modified_since: Option<headers::IfModifiedSince>,
	if_range: Option<headers::IfRange>,
}

#[async_trait::async_trait]
impl<B: Send> FromRequest<B> for Params {
	type Rejection = ParamsRejection;

	async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
		let headers = req.headers();
		Ok(Self {
			range: headers
				.typed_try_get()
				.map_err(|_| ParamsRejection::Invalid("Range"))?,
			if_modified_since: headers
				.typed_try_get()
				.map_err(|_| ParamsRejection::Invalid("If-Modified-Since"))?,
			if_range: headers
				.typed_try_get()
				.map_err(|_| ParamsRejection::Invalid("If-Range"))?,
		})
	}
}

impl Params {
	fn is_not_modified(&self, current_mtime: &DateTime<Utc>) -> Option<bool> {
		use chrono::{Duration, DurationRound as _};

		let req_mtime: DateTime<Utc> = std::time::SystemTime::from(self.if_modified_since?).into();
		Some(
			current_mtime.duration_trunc(Duration::seconds(1)).ok()?
				<= req_mtime.duration_trunc(Duration::seconds(1)).ok()?,
		)
	}

	fn parse_range(&self, file_size: u64) -> Option<(u64, u64)> {
		use std::ops::Bound;

		let (from, to) = itertools::Itertools::exactly_one(self.range.as_ref()?.iter()).ok()?;
		let from = match from {
			Bound::Excluded(x) => x.checked_add(1)?,
			Bound::Included(x) => x,
			Bound::Unbounded => 0,
		};
		let to = match to {
			Bound::Excluded(x) => x.checked_sub(1)?,
			Bound::Included(x) => x,
			Bound::Unbounded => file_size,
		};
		Some((from, to))
	}
}

pub struct JustFile {
	path: PathBuf,
	req: Params,
}

impl JustFile {
	#[inline]
	pub fn new(path: PathBuf, req: Params) -> Self {
		Self { path, req }
	}
}

#[derive(Debug)]
struct FileStream {
	file: tokio::fs::File,
	bytes_left: usize,
}

impl FileStream {
	fn new(file: std::fs::File, bytes_left: usize) -> Self {
		Self {
			file: tokio::fs::File::from_std(file),
			bytes_left,
		}
	}
}

impl Body for FileStream {
	type Data = Bytes;
	type Error = std::io::Error;

	fn poll_data(
		mut self: Pin<&mut Self>,
		context: &mut Context<'_>,
	) -> Poll<Option<Result<Self::Data, Self::Error>>> {
		use tokio::io::AsyncRead as _;

		if self.is_end_stream() {
			return Poll::Ready(None);
		}

		// 1024 is arbitrary
		let mut buf = [0u8; 1024];
		let mut read_buf = ReadBuf::new(&mut buf);
		match Pin::new(&mut self.file).poll_read(context, &mut read_buf) {
			Poll::Ready(maybe_err) => Poll::Ready(match maybe_err {
				Ok(()) => match read_buf.filled() {
					[] => None,
					filled => {
						let bytes_left_before_this_read = self.bytes_left;
						self.bytes_left = self.bytes_left.saturating_sub(filled.len());
						Some(Ok(
							filled[0..std::cmp::min(filled.len(), bytes_left_before_this_read)]
								.to_owned()
								.into(),
						))
					}
				},
				Err(err) => Some(Err(err)),
			}),
			Poll::Pending => Poll::Pending,
		}
	}

	fn poll_trailers(
		self: Pin<&mut Self>,
		_cx: &mut Context<'_>,
	) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
		Poll::Ready(Ok(None))
	}

	fn is_end_stream(&self) -> bool {
		self.bytes_left == 0
	}

	fn size_hint(&self) -> http_body::SizeHint {
		http_body::SizeHint::with_exact(self.bytes_left.try_into().unwrap()) // usize fits into u64
	}
}

impl JustFile {
	fn respond_to_impl(self) -> Result<Response, (&'static str, std::io::Error)> {
		let mut file = std::fs::File::open(self.path).map_err(|err| ("opening file", err))?;
		let metadata = file
			.metadata()
			.map_err(|err| ("getting file metadata", err))?;
		let len = metadata.len();
		let range = self.req.parse_range(len);
		let current_mtime_sys = metadata
			.modified()
			.map_err(|err| ("getting file mtime", err))?;
		let current_mtime = <DateTime<Utc>>::from(current_mtime_sys);
		let mut response = if self.req.is_not_modified(&current_mtime) == Some(true) {
			StatusCode::NOT_MODIFIED.into_response()
		} else {
			let max: usize = if let Some((start, end)) = range {
				// `parse_range` already verified that the range is valid and within bounds
				file
					.seek(std::io::SeekFrom::Start(start))
					.map_err(|err| ("seeking file", err))?;
				end - start
			} else {
				len
			}
			.try_into()
			.unwrap();

			let mut response = Response::new(
				FileStream::new(file, max)
					.map_err(axum::Error::new)
					.boxed_unsync(),
			);
			if let Some((start, end)) = range {
				*response.status_mut() = StatusCode::PARTIAL_CONTENT;
				response
					.headers_mut()
					.typed_insert(headers::ContentRange::bytes(start..end, Some(len)).unwrap());
			} else {
				*response.status_mut() = StatusCode::OK;
			};
			response
				.headers_mut()
				.typed_insert(headers::AcceptRanges::bytes());
			response
		};
		response
			.headers_mut()
			.typed_insert(headers::LastModified::from(current_mtime_sys));
		Ok(response)
	}
}

impl IntoResponse for JustFile {
	fn into_response(self) -> Response {
		match self.respond_to_impl() {
			Ok(response) => response,
			Err((reason, err)) => crate::error::Io(reason, err).into_response(),
		}
	}
}
