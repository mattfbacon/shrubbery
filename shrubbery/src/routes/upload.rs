use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};

use crate::config::Config;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, multipart};

#[derive(askama::Template)]
#[template(path = "files/upload.html")]
struct Template {
	self_user: models::User,
}
crate::helpers::impl_into_response!(Template);

async fn get_handler(auth::Editor(self_user): auth::Editor) -> impl IntoResponse {
	Template { self_user }
}

async fn post_handler(
	auth::Editor(_self_user): auth::Editor,
	mut multipart: extract::Multipart,
	extract::Extension(database): extract::Extension<Arc<Database>>,
	extract::Extension(config): extract::Extension<Arc<Config>>,
) -> Result<Response, ErrorResponse> {
	let database = &*database;

	let name_field = multipart::get_one_text(&mut multipart, "name").await?;
	let description_field = Some(multipart::get_one_text(&mut multipart, "description").await?)
		.filter(|content| !content.is_empty());
	let (media_type, write_state) = multipart::WriteToFile::start(&mut multipart, "file").await?;
	let record = sqlx::query!(
		"INSERT INTO files (name, description, media_type) VALUES ($1, $2, $3) RETURNING id",
		name_field,
		description_field,
		media_type as _,
	)
	.fetch_one(database)
	.await
	.map_err(error::Sqlx)?;
	let file_id = record.id;
	write_state.create(file_id, &config).await?;
	Ok(Redirect::to(&format!("/files/{file_id}?created")).into_response())
}

pub fn configure() -> Router {
	let mut app = Router::new();
	app = app.route("/", axum::routing::get(get_handler).post(post_handler));
	app
}
