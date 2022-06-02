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

pub use template::error_response;

macro_rules! impl_response {
	($struct_name:ident, $status:ident) => {
		impl IntoResponse for $struct_name {
			fn into_response(self) -> Response {
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
