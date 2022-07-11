use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, set_none_if_empty, OrNull};

pub struct Tag {
	pub id: models::TagId,
	pub name: String,
	pub description: Option<String>,
	pub category: Option<models::TagCategoryId>,
	pub created_time: crate::timestamp::Timestamp,
	pub created_by: Option<String>,
}

impl Tag {
	pub async fn by_id(database: &Database, id: models::TagId) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(Self, r#"SELECT tags.id, tags.name, tags.description, tags.category, tags.created_time, (SELECT name FROM users WHERE id = tags.id) as created_by FROM tags WHERE id = $1"#, id).fetch_optional(database).await
	}
}

#[derive(askama::Template)]
#[template(path = "tags/edit.html")]
struct Template {
	updated: bool,
	self_user: models::User,
	requested_tag: Tag,
	tag_categories: Vec<(models::TagCategoryId, String)>,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	extract::Path((tag_id,)): extract::Path<(models::TagId,)>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	let requested_tag = Tag::by_id(database, tag_id)
		.await
		.map_err(error::Sqlx)?
		.ok_or(error::EntityNotFound("tag"))?;

	let tag_categories = super::shared::get_tag_categories_lean(database)
		.await
		.map_err(error::Sqlx)?;

	Ok(Template {
		updated: false,
		self_user,
		requested_tag,
		tag_categories,
	})
}

#[derive(Debug, serde::Deserialize)]
pub struct PostRequest {
	name: String,
	description: Option<String>,
	category: OrNull<models::TagCategoryId>,
}

pub async fn post_handler(
	auth::Editor(self_user): auth::Editor,
	extract::Path((tag_id,)): extract::Path<(models::TagId,)>,
	extract::Form(mut request): extract::Form<PostRequest>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	set_none_if_empty(&mut request.description);

	sqlx::query!(
		"UPDATE tags SET name = $2, description = $3, category = $4 WHERE id = $1",
		tag_id,
		&request.name,
		request.description.as_ref(),
		request.category.into_option()
	)
	.execute(database)
	.await
	.map_err(error::Sqlx)?;

	let requested_tag = Tag::by_id(database, tag_id)
		.await
		.map_err(error::Sqlx)?
		.ok_or(error::EntityNotFound("tag"))?;

	let tag_categories = super::shared::get_tag_categories_lean(database)
		.await
		.map_err(error::Sqlx)?;

	Ok(Template {
		updated: true,
		self_user,
		requested_tag,
		tag_categories,
	})
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler).post(post_handler))
}
