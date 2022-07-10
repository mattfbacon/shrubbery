use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};

use crate::config::Config;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth;

#[derive(askama::Template)]
#[template(path = "files/upload.html")]
struct Template {
	self_user: models::User,
}
crate::helpers::impl_into_response!(Template);

async fn get_handler(auth::Editor(self_user): auth::Editor) -> impl IntoResponse {
	Template { self_user }
}

#[derive(axum_easy_multipart::FromMultipart)]
struct PostRequest {
	name: String,
	description: Option<String>,
	file: axum_easy_multipart::file::File,
}

async fn post_handler(
	auth::Editor(_self_user): auth::Editor,
	axum_easy_multipart::Extractor(mut req): axum_easy_multipart::Extractor<PostRequest>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
	extract::Extension(config): extract::Extension<Arc<Config>>,
) -> Result<Response, ErrorResponse> {
	let database = &*database;
	req.description = req.description.filter(|desc| !desc.is_empty());

	let media_type = req
		.file
		.content_type
		.and_then(models::MediaType::from_mime)
		.ok_or(error::BadContentType)?;
	let record = sqlx::query!(
		"INSERT INTO files (name, description, media_type) VALUES ($1, $2, $3) RETURNING id",
		req.name,
		req.description,
		media_type as _,
	)
	.fetch_one(database)
	.await
	.map_err(error::Sqlx)?;

	let file_id = record.id;
	match req
		.file
		.temp_path
		.persist_noclobber(config.file_storage.join(format!("{file_id}")))
		.map_err(|error| error.error)
	{
		Ok(()) => Ok(Redirect::to(&format!("/files/{file_id}?created")).into_response()),
		Err(error) => {
			let _ = sqlx::query!("DELETE FROM files WHERE id = $1", file_id)
				.execute(database)
				.await;
			Err(error::Io("storing file in filesystem", error).into())
		}
	}
}

pub fn configure() -> Router {
	let mut app = Router::new();
	app = app.route("/", axum::routing::get(get_handler).post(post_handler));
	app
}
