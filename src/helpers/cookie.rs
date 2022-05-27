use axum::response::{IntoResponseParts, ResponseParts};
use cookie::Cookie;

/// Simple wrapper around `cookie::Cookie` that implements `axum::response::IntoResponseParts`
pub struct CookiePart<'a>(pub Cookie<'a>);

impl IntoResponseParts for CookiePart<'_> {
	type Error = std::convert::Infallible;

	fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
		res.headers_mut().insert("Set-Cookie", self.encode());
		Ok(res)
	}
}

impl<'a> From<Cookie<'a>> for CookiePart<'a> {
	fn from(inner: Cookie<'a>) -> Self {
		Self(inner)
	}
}

impl CookiePart<'_> {
	fn encode(&self) -> http::HeaderValue {
		self.0.to_string().parse().unwrap() // we assert that an encoded value should always be a valid header value
	}

	fn onto_builder(&self, builder: http::response::Builder) -> http::response::Builder {
		builder.header("Set-Cookie", self.encode())
	}
}
