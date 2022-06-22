use std::borrow::Cow;

use axum::response::{IntoResponse, Response};

mod template;
pub use template::default_handler;

#[derive(Debug, thiserror::Error)]
#[error("SQL error: {0}")]
pub struct Sqlx(#[source] pub sqlx::Error);

#[derive(Debug, thiserror::Error)]
#[error("{0} not found")]
pub struct EntityNotFound(pub &'static str);

#[derive(Debug, thiserror::Error)]
#[error("password hash error")]
pub struct PasswordHash;

#[derive(Debug, thiserror::Error)]
#[error("error while encrypting: {0}")]
pub struct Encrypt(#[source] pub crate::token::crypt::EncryptError);

#[derive(Debug, thiserror::Error)]
#[error("error while decrypting: {0}")]
pub struct Decrypt(#[source] pub crate::token::crypt::DecryptError);

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Multipart(#[source] pub axum::extract::multipart::MultipartError);

#[derive(Debug, thiserror::Error)]
#[error("IO error while {0}: {1}")]
pub struct Io(pub &'static str, #[source] pub std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct BadRequest(pub Cow<'static, str>);

#[derive(Debug, thiserror::Error)]
#[error("wrong field order (expected {0:?} field)")]
pub struct WrongFieldOrder(pub &'static str);

#[derive(Debug, thiserror::Error)]
#[error("unexpected end of fields (expected {0:?} field)")]
pub struct ExpectedField(pub &'static str);

pub use template::error_response;

macro_rules! impl_response {
	($struct_name:ident, $status:ident) => {
		impl axum::response::IntoResponse for $struct_name {
			fn into_response(self) -> axum::response::Response {
				crate::error::error_response(&self, http::StatusCode::$status)
			}
		}
	};
}
pub(crate) use impl_response;

impl_response!(Sqlx, INTERNAL_SERVER_ERROR);
impl_response!(EntityNotFound, NOT_FOUND);
impl_response!(PasswordHash, INTERNAL_SERVER_ERROR);
impl_response!(Encrypt, INTERNAL_SERVER_ERROR);
impl_response!(Decrypt, INTERNAL_SERVER_ERROR);
impl_response!(Multipart, BAD_REQUEST);
impl_response!(BadRequest, BAD_REQUEST);
impl_response!(WrongFieldOrder, BAD_REQUEST);
impl_response!(ExpectedField, BAD_REQUEST);

impl IntoResponse for Io {
	fn into_response(self) -> Response {
		use std::io::ErrorKind;

		use http::StatusCode;

		let status_code = match self.1.kind() {
			ErrorKind::NotFound => StatusCode::NOT_FOUND,
			// I have found that this is typically an error in the server, so not using this:
			// ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
			_ => StatusCode::INTERNAL_SERVER_ERROR,
		};
		crate::error::error_response(&self, status_code)
	}
}
