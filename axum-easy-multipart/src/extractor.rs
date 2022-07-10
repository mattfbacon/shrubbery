use std::pin::Pin;
use std::task::{Context, Poll};

use async_trait::async_trait;
use axum_core::extract::{FromRequest, RequestParts};
use axum_core::response::{IntoResponse, Response};
use bytes::Bytes;
use http::HeaderMap;
use http_body::Body;

use crate::{Error, FromMultipart};

pub struct Extractor<T: FromMultipart>(pub T);

#[derive(Debug, thiserror::Error)]
pub enum Rejection {
    #[error(transparent)]
    Error(Error),
    #[error("invalid multipart boundary")]
    InvalidBoundary,
    #[error("request body was already extracted")]
    BodyAlreadyExtracted,
}

impl IntoResponse for Rejection {
    fn into_response(self) -> Response {
        (http::StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}

pin_project_lite::pin_project! {
    struct BodyStream<B: Body<Data = Bytes>> {
        #[pin]
        inner: B,
    }
}

impl<B: Body<Data = Bytes>> futures_core::Stream for BodyStream<B> {
    type Item = Result<Bytes, <B as Body>::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_data(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size_hint = self.inner.size_hint();
        (
            size_hint.lower().try_into().unwrap_or(usize::MAX), // if the lower size hint was too large, use the largest possible value
            size_hint.upper().and_then(|upper| upper.try_into().ok()), // if the upper size hint was too large, just remove it
        )
    }
}

#[async_trait]
impl<T, B> FromRequest<B> for Extractor<T>
where
    B: Body<Data = Bytes> + Send,
    B::Error: std::error::Error + Send + Sync + 'static,
    T: FromMultipart,
{
    type Rejection = Rejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Rejection> {
        let body = req.take_body().ok_or(Rejection::BodyAlreadyExtracted)?;
        let stream = BodyStream { inner: body };
        let boundary = parse_boundary(req.headers()).ok_or(Rejection::InvalidBoundary)?;
        let mut multipart = multer::Multipart::new(stream, boundary);
        let extensions = req.extensions(); // if this is not a let binding, the future becomes !Send, apparently
        T::from_multipart(&mut multipart, extensions)
            .await
            .map(Self)
            .map_err(Rejection::Error)
    }
}

fn parse_boundary(headers: &HeaderMap) -> Option<String> {
    let content_type = headers.get(http::header::CONTENT_TYPE)?.to_str().ok()?;
    multer::parse_boundary(content_type).ok()
}
