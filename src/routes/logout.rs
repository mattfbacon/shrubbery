use crate::helpers::remove_cookie;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{web, HttpResponse};

pub async fn handler() -> HttpResponse {
	HttpResponse::build(HttpStatus::SEE_OTHER)
		.insert_header(("Location", "/login"))
		.cookie(remove_cookie("token"))
		.finish()
}

pub fn configure(app: &mut web::ServiceConfig) {
	app.service(web::resource("").route(web::get().to(handler)));
}
