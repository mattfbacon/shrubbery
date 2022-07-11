use axum::response::{IntoResponse, Redirect};
use axum::Router;

use crate::helpers::cookie::Part as CookiePart;

pub async fn handler() -> impl IntoResponse {
	(
		CookiePart::new_removal(crate::token::COOKIE_NAME),
		Redirect::to("/to"),
	)
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(handler))
}
