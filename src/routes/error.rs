use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode as HttpStatus;
use actix_web::HttpResponse;

#[derive(askama::Template)]
#[template(path = "error.html")]
struct Template {
	code: u16,
	description: Option<&'static str>,
	associated_error: String,
}

pub fn error_response<E>(error: &E) -> HttpResponse
where
	E: actix_web::ResponseError + std::error::Error,
{
	use askama::Template as _;
	let status_code = error.status_code();
	let template = Template {
		code: status_code.as_u16(),
		description: status_code.canonical_reason(),
		associated_error: error.to_string(),
	};
	match template.render() {
		Ok(template) => HttpResponse::build(status_code)
			.insert_header(("Content-Type", "text/html"))
			.body(template),
		Err(error) => HttpResponse::build(HttpStatus::INTERNAL_SERVER_ERROR).body(error.to_string()),
	}
}

pub async fn default_handler(req: ServiceRequest) -> Result<ServiceResponse, actix_web::Error> {
	use askama::Template as _;

	let req = req.into_parts().0;
	let res = Template {
		code: 404,
		description: Some("Not Found"),
		associated_error: "Page not found".to_string(),
	}
	.render()
	.unwrap();
	Ok(ServiceResponse::new(
		req,
		HttpResponse::NotFound()
			.insert_header(("Content-Type", Template::MIME_TYPE))
			.body(res),
	))
}
