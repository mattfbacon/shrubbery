use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::routing::get;
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, pagination};

mod delete;
mod edit;
mod shared;

struct Tag {
	pub id: models::TagId,
	pub name: String,
	pub description: Option<String>,
	pub category: Option<String>,
	pub created_time: crate::timestamp::Timestamp,
	pub created_by: Option<String>,
}

#[derive(askama::Template)]
#[template(path = "tags/index.html")]
struct Template {
	self_user: models::User,
	tags: Vec<Tag>,
	tag_categories: Vec<(models::TagCategoryId, String)>,
	pagination: pagination::Template,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	auth::Auth(self_user): auth::Auth,
	extract::Query(pagination): extract::Query<pagination::Query>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	let num_pages = std::cmp::max(
		models::Tag::count(database).await.map_err(error::Sqlx)? / pagination.page_size(),
		1,
	);

	if pagination.page() >= num_pages {
		return Err(error::EntityNotFound("page").into());
	}

	let tags = sqlx::query_as!(Tag, r#"SELECT tags.id, tags.name, tags.description, tag_categories.name as "category?", tags.created_time, users.username as "created_by?" FROM tags LEFT JOIN users ON tags.created_by = users.id LEFT JOIN tag_categories ON tags.category = tag_categories.id ORDER BY tags.id OFFSET $1 LIMIT $2"#, pagination.offset(), pagination.limit()).fetch_all(database).await.map_err(error::Sqlx)?;

	let tag_categories = shared::get_tag_categories_lean(database)
		.await
		.map_err(error::Sqlx)?;

	Ok(Template {
		self_user,
		tags,
		tag_categories,
		pagination: pagination::Template::from_query(pagination, num_pages),
	})
}

#[derive(serde::Deserialize)]
pub struct CreateRequest {
	name: String,
	description: String,
	category: Option<models::TagCategoryId>,
}

pub async fn post_handler(
	auth::Editor(self_user): auth::Editor,
	pagination: extract::Query<pagination::Query>,
	extract::Form(req): extract::Form<CreateRequest>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	use ormx::Insert as _;

	models::tag::Create {
		name: req.name,
		description: if req.description.is_empty() {
			None
		} else {
			Some(req.description)
		},
		category: req.category,
		created_by: Some(self_user.id),
	}
	.insert(&*database)
	.await
	.map_err(error::Sqlx)?;

	get_handler(
		auth::Auth(self_user),
		pagination,
		extract::Extension(database),
	)
	.await
}

pub fn configure() -> Router {
	Router::new()
		.route("/", get(get_handler).post(post_handler))
		.nest("/:tag_id", delete::configure().merge(edit::configure()))
}
