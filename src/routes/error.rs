use actix_web::body::BoxBody;
use actix_web::dev::ServiceResponse;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::middleware::ErrorHandlerResponse;
use actix_web::middleware::ErrorHandlers;
use actix_web::Responder as _;

pub fn error_handlers() -> ErrorHandlers<BoxBody> {
	#[derive(askama::Template)]
	#[template(path = "error.html")]
	struct ErrorTemplate {
		code: u16,
		description: String,
		associated_error: Option<String>,
	}

	let mut handlers = ErrorHandlers::new();
	for handled in [
		// HttpStatus::BAD_REQUEST,
		// HttpStatus::UNAUTHORIZED,
		// HttpStatus::FORBIDDEN,
		HttpStatus::NOT_FOUND,
		// HttpStatus::METHOD_NOT_ALLOWED,
		HttpStatus::INTERNAL_SERVER_ERROR,
	] {
		handlers = handlers.handler(handled, move |old: ServiceResponse| {
			let response = ErrorTemplate {
				code: handled.as_u16(),
				description: handled.canonical_reason().unwrap_or("").to_owned(),
				associated_error: old.response().error().map(|err| err.to_string()),
			};
			let response = response
				.customize()
				.with_status(handled)
				.respond_to(old.request());
			Ok(ErrorHandlerResponse::Response(
				old
					.into_response(response)
					.map_into_boxed_body()
					.map_into_left_body(),
			))
		});
	}
	handlers
}
