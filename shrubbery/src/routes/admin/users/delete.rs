use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect};
use axum::{extract, Router};
use http::request::Parts;
use ormx::Delete as _;

use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth;

pub async fn post_handler(
	auth::Admin(_self_user): auth::Admin,
	req: Parts,
	extract::Path((user_id,)): extract::Path<(models::UserId,)>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	models::User::delete_row(&*database, user_id)
		.await
		.map_err(error::Sqlx)?;
	Ok(Redirect::to(&format!(
		"/admin/users?{}",
		req.uri.query().unwrap_or("")
	)))
}

pub fn configure() -> Router {
	Router::new().route("/delete", axum::routing::post(post_handler))
}
