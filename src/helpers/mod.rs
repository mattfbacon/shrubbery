use actix_web::cookie::Cookie;

pub mod auth;
pub mod or_null;
pub mod pagination;

pub use or_null::OrNull;

pub fn internal_server_error<T>(err: T) -> actix_web::error::InternalError<T> {
	actix_web::error::InternalError::new(err, actix_web::http::StatusCode::INTERNAL_SERVER_ERROR)
}

pub fn remove_cookie(name: &str) -> Cookie<'_> {
	let mut ret = Cookie::new(name, "");
	ret.make_removal();
	ret
}

pub fn set_none_if_empty(opt: &mut Option<String>) {
	if opt.as_deref() == Some("") {
		*opt = None;
	}
}
