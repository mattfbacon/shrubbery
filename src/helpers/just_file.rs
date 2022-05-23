use actix_web::body::{BoxBody, SizedStream};
use actix_web::web::Bytes;
use actix_web::{HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use futures::Stream;
use std::io::Seek as _;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

pub struct JustFile(PathBuf);

impl JustFile {
	#[inline]
	pub fn new(path: PathBuf) -> Self {
		Self(path)
	}
}

struct FileStream {
	file: tokio::fs::File,
	max: Option<usize>,
}

impl FileStream {
	fn new(file: std::fs::File, max: Option<usize>) -> Self {
		Self {
			file: tokio::fs::File::from_std(file),
			max,
		}
	}
}

impl Stream for FileStream {
	type Item = Result<Bytes, std::io::Error>;

	fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		use tokio::io::AsyncRead as _;

		if let Some(0) = self.max {
			return Poll::Ready(None);
		}

		// 1024 is arbitrary
		let mut buf = [0u8; 1024];
		let mut read_buf = ReadBuf::new(&mut buf);
		match Pin::new(&mut self.file).poll_read(context, &mut read_buf) {
			Poll::Ready(maybe_err) => Poll::Ready(match maybe_err {
				Ok(()) => match read_buf.filled() {
					[] => None,
					filled => Some(Ok(
						match self.max {
							Some(max) => {
								self.max = Some(max.saturating_sub(filled.len()));
								&filled[0..std::cmp::min(filled.len(), max)]
							}
							None => filled,
						}
						.to_owned()
						.into(),
					)),
				},
				Err(err) => Some(Err(err)),
			}),
			Poll::Pending => Poll::Pending,
		}
	}
}

impl JustFile {
	fn is_not_modified(req: &HttpRequest, current_mtime: &DateTime<Utc>) -> Option<bool> {
		use chrono::{Duration, DurationRound as _};

		let req_mtime = req.headers().get("If-Modified-Since")?.to_str().ok()?;
		let req_mtime: DateTime<Utc> = DateTime::parse_from_rfc2822(req_mtime).ok()?.into();
		Some(
			current_mtime.duration_trunc(Duration::seconds(1)).ok()?
				<= req_mtime.duration_trunc(Duration::seconds(1)).ok()?,
		)
	}

	fn parse_range(req: &HttpRequest, file_size: u64) -> Result<Option<(u64, u64)>, HttpResponse> {
		use actix_web::http::header::Range;
		use std::str::FromStr as _;

		let range = match req.headers().get("Range") {
			Some(range) => range,
			None => return Ok(None),
		};
		match range
			.to_str()
			.ok()
			.and_then(|range| actix_web::http::header::Range::from_str(range).ok())
		{
			Some(Range::Bytes(range)) if range.len() == 1 => Ok(range[0].to_satisfiable_range(file_size)),
			Some(..) => Ok(None),
			None => Err(HttpResponse::BadRequest().body("invalid Range header")),
		}
	}

	fn respond_to_impl(self, req: &HttpRequest) -> Result<HttpResponse, std::io::Error> {
		let mut file = std::fs::File::open(self.0)?;
		let metadata = file.metadata()?;
		let range = match Self::parse_range(req, metadata.len()) {
			Ok(ret) => ret,
			Err(resp) => return Ok(resp),
		};
		let current_mtime: DateTime<Utc> = metadata.modified()?.into();
		Ok(
			if Self::is_not_modified(req, &current_mtime) == Some(true) {
				HttpResponse::NotModified().body(())
			} else {
				let max: Option<usize> = if let Some((start, end)) = range {
					// `parse_range` already verified that the range is valid and within bounds
					file.seek(std::io::SeekFrom::Start(start))?;
					(end - start).try_into().ok() // gracefully degrade to sending the full file if it's too big
				} else {
					None
				};

				let mut response;
				if range.is_some() {
					response = HttpResponse::PartialContent();
					response.insert_header(actix_web::http::header::ContentRange(
						actix_web::http::header::ContentRangeSpec::Bytes {
							range,
							instance_length: Some(metadata.len()),
						},
					));
				} else {
					response = HttpResponse::Ok();
				};
				response
					.insert_header(("Accept-Ranges", "bytes"))
					.insert_header(actix_web::http::header::LastModified(
						std::time::SystemTime::from(current_mtime).into(),
					))
					.body(SizedStream::new(
						max
							.map(|max| max.try_into().unwrap())
							.unwrap_or_else(|| metadata.len()),
						FileStream::new(file, max),
					))
			},
		)
	}
}

impl Responder for JustFile {
	type Body = BoxBody;

	fn respond_to(self, req: &HttpRequest) -> HttpResponse {
		match self.respond_to_impl(req) {
			Ok(response) => response,
			Err(err) => HttpResponse::from_error(err),
		}
	}
}
