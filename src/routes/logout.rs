use crate::helpers::remove_cookie;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{get, HttpResponse};

#[get("/logout")]
pub async fn handler() -> HttpResponse {
	HttpResponse::build(HttpStatus::SEE_OTHER)
		.insert_header(("Location", "/login"))
		.cookie(remove_cookie("token"))
		.finish()
}
