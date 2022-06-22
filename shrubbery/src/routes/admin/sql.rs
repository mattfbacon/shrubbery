use std::sync::Arc;

use axum::response::IntoResponse;
use axum::{extract, Router};

use crate::database::Database;
use crate::helpers::auth;

#[derive(serde::Deserialize)]
pub struct PostData {
	sql: String,
}

pub async fn post_handler(
	auth::Admin(_self_user): auth::Admin,
	extract::Form(PostData { sql }): extract::Form<PostData>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> impl IntoResponse {
	let result = sqlx::query(&sql).execute(&*database).await;
	format!("{:#?}", result)
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::post(post_handler))
}
