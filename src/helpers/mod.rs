use http::status::StatusCode;

pub mod auth;
pub mod cookie;
pub mod or_null;
pub mod pagination;

pub use or_null::OrNull;

pub fn set_none_if_empty(opt: &mut Option<String>) {
	if opt.as_deref() == Some("") {
		*opt = None;
	}
}

pub fn io_error_to_status(err: &std::io::Error) -> StatusCode {
	use std::io::ErrorKind;
	match err.kind() {
		ErrorKind::NotFound => StatusCode::NOT_FOUND,
		ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
		_ => StatusCode::INTERNAL_SERVER_ERROR,
	}
}

/// This is temporary while askama fixes their issues with deriving `axum::IntoResponse`
macro_rules! impl_into_response {
	($name:ident) => {
		impl axum::response::IntoResponse for $name {
			fn into_response(self) -> axum::response::Response {
				match askama::Template::render(&self) {
					Ok(page) => (
						[("Content-Type", <$name as askama::Template>::MIME_TYPE)],
						page,
					)
						.into_response(),
					Err(err) => (http::StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
				}
			}
		}
	};
}
pub(crate) use impl_into_response;
