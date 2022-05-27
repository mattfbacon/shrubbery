use axum::response::{IntoResponse, Redirect};
use axum::Router;

use crate::helpers::cookie::CookiePart;

pub async fn handler() -> impl IntoResponse {
	(
		CookiePart(crate::token::remove_cookie()),
		Redirect::to("/to"),
	)
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(handler))
}
