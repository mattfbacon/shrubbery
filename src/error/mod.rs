use axum::response::{IntoResponse, Response};

mod template;
pub use template::default_handler;

/// This just makes sure that any error responses are endorsed by us, rather than just being arbitrary `IntoResponse` types
pub trait Error: IntoResponse {}

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

macro_rules! impl_response {
	($struct_name:ident, $status:ident) => {
		impl IntoResponse for $struct_name {
			fn into_response(self) -> Response {
				template::error_response(&self, http::StatusCode::$status)
			}
		}
		impl Error for $struct_name {}
	};
}

impl_response!(Sqlx, INTERNAL_SERVER_ERROR);
impl_response!(EntityNotFound, NOT_FOUND);
impl_response!(PasswordHash, INTERNAL_SERVER_ERROR);
impl_response!(Encrypt, INTERNAL_SERVER_ERROR);
impl_response!(Decrypt, INTERNAL_SERVER_ERROR);
