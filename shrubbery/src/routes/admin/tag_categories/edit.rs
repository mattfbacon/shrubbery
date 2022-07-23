use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, set_none_if_empty};

#[derive(askama::Template)]
#[template(path = "admin/tag_categories/edit.html")]
struct Template {
	updated: bool,
	self_user: models::User,
	requested_tag_category: super::TagCategoryWithUserResolved,
}
crate::helpers::impl_into_response!(Template);

impl super::TagCategoryWithUserResolved {
	async fn get_by_id(
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
		id: models::TagCategoryId,
	) -> sqlx::Result<Option<Self>> {
		sqlx::query_as!(super::TagCategoryWithUserResolved, r#"SELECT tag_categories.id, tag_categories.name, tag_categories.description, tag_categories.color as "color: models::Color", tag_categories.created_time as "created_time: crate::timestamp::Timestamp", (SELECT name FROM users WHERE id = tag_categories.created_by) as created_by FROM tag_categories WHERE id = $1"#, id).fetch_optional(database).await
	}
}

pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	extract::Path((tag_category_id,)): extract::Path<(models::TagCategoryId,)>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	let requested_tag_category =
		super::TagCategoryWithUserResolved::get_by_id(database, tag_category_id)
			.await
			.map_err(error::Sqlx)?
			.ok_or(error::EntityNotFound("tag category"))?;

	Ok(Template {
		updated: false,
		self_user,
		requested_tag_category,
	})
}

#[derive(Debug, serde::Deserialize)]
pub struct PostRequest {
	name: String,
	description: Option<String>,
	color: models::Color,
}

pub async fn post_handler(
	auth::Admin(self_user): auth::Admin,
	extract::Path((tag_category_id,)): extract::Path<(models::TagCategoryId,)>,
	extract::Form(mut request): extract::Form<PostRequest>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	set_none_if_empty(&mut request.description);

	let query_result = sqlx::query!(
		"UPDATE tag_categories SET name = $2, description = $3, color = $4 WHERE id = $1",
		tag_category_id,
		request.name,
		request.description,
		request.color as _
	)
	.execute(database)
	.await
	.map_err(error::Sqlx)?;
	if query_result.rows_affected() == 0 {
		return Err(error::EntityNotFound("tag category").into());
	}

	let requested_tag_category =
		super::TagCategoryWithUserResolved::get_by_id(database, tag_category_id)
			.await
			.map_err(error::Sqlx)?
			.ok_or(error::EntityNotFound("tag category"))?;

	Ok(Template {
		updated: true,
		self_user,
		requested_tag_category,
	})
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler).post(post_handler))
}
