pub mod auth;

pub fn internal_server_error<T>(err: T) -> actix_web::error::InternalError<T> {
	actix_web::error::InternalError::new(err, actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
}
