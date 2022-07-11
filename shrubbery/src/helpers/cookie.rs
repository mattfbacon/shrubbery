use axum::response::{IntoResponseParts, ResponseParts};
use cookie::Cookie;

/// Simple wrapper around `cookie::Cookie` that implements `axum::response::IntoResponseParts`
pub struct Part<'a>(pub Cookie<'a>);

impl IntoResponseParts for Part<'_> {
	type Error = std::convert::Infallible;

	fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
		res.headers_mut().insert("Set-Cookie", self.encode());
		Ok(res)
	}
}

impl<'a> From<Cookie<'a>> for Part<'a> {
	fn from(inner: Cookie<'a>) -> Self {
		Self(inner)
	}
}

impl Part<'_> {
	pub fn encode(&self) -> http::HeaderValue {
		self.0.to_string().parse().unwrap() // we assert that an encoded value should always be a valid header value
	}

	pub fn onto_builder(&self, builder: http::response::Builder) -> http::response::Builder {
		builder.header("Set-Cookie", self.encode())
	}
}

impl<'a> Part<'a> {
	pub fn new_removal(name: &'a str) -> Self {
		let mut cookie = Cookie::named(name);
		cookie.make_removal();
		Self(cookie)
	}
}
