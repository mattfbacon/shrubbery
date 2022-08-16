use axum::response::{IntoResponse, Response};
use http::StatusCode;

#[derive(askama::Template)]
#[template(path = "error.html")]
struct Template {
	code: u16,
	description: Option<&'static str>,
	associated_error: String,
}
crate::helpers::impl_into_response!(Template);

pub fn error_response(error: &dyn std::error::Error, status_code: StatusCode) -> Response {
	let template = Template {
		code: status_code.as_u16(),
		description: status_code.canonical_reason(),
		associated_error: error.to_string(),
	};

	IntoResponse::into_response((status_code, template))
}

#[cfg(not(debug_assertions))]
pub async fn default_handler() -> (StatusCode, impl IntoResponse) {
	let template = Template {
		code: 404,
		description: Some(StatusCode::NOT_FOUND.canonical_reason().unwrap()),
		associated_error: "Page not found".to_owned(),
	};

	(StatusCode::NOT_FOUND, template)
}
