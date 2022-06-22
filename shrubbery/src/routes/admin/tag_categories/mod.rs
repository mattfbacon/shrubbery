use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth::Admin;
use crate::helpers::pagination;

mod delete;
mod edit;

struct TagCategoryWithUserResolved {
	pub id: models::TagCategoryId,
	pub name: String,
	pub description: Option<String>,
	pub color: models::Color,
	pub created_time: crate::timestamp::Timestamp,
	pub created_by: Option<String>,
}

#[derive(askama::Template)]
#[template(path = "admin/tag_categories/index.html")]
struct Template {
	self_user: models::User,
	tag_categories: Vec<TagCategoryWithUserResolved>,
	pagination: pagination::Template,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	Admin(self_user): Admin,
	extract::Query(pagination): extract::Query<pagination::Query>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	let num_pages = std::cmp::max(
		models::TagCategory::count(database)
			.await
			.map_err(error::Sqlx)?
			/ pagination.page_size(),
		1,
	);
	if pagination.page() >= num_pages {
		return Err(error::EntityNotFound("page").into());
	}

	let tag_categories = sqlx::query_as!(TagCategoryWithUserResolved, r#"SELECT tag_categories.id, tag_categories.name, tag_categories.description, tag_categories.color as "color: models::Color", tag_categories.created_time, users.username as "created_by?" FROM tag_categories LEFT JOIN users ON tag_categories.created_by = users.id ORDER BY tag_categories.id OFFSET $1 LIMIT $2"#, pagination.offset(), pagination.limit()).fetch_all(database).await.map_err(error::Sqlx)?;

	Ok(Template {
		self_user,
		tag_categories,
		pagination: pagination::Template::from_query(pagination, num_pages),
	})
}

#[derive(serde::Deserialize)]
pub struct CreateRequest {
	name: String,
	description: String,
	color: models::Color,
}

pub async fn post_handler(
	Admin(self_user): Admin,
	extract::Query(pagination): extract::Query<pagination::Query>,
	extract::Form(req): extract::Form<CreateRequest>,
	database: extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	use ormx::Insert as _;

	models::tag_category::Create {
		name: req.name,
		description: if req.description.is_empty() {
			None
		} else {
			Some(req.description)
		},
		color: req.color,
		created_by: Some(self_user.id),
	}
	.insert(&**database)
	.await
	.map_err(error::Sqlx)?;

	get_handler(Admin(self_user), extract::Query(pagination), database).await
}

pub fn configure() -> Router {
	Router::new()
		.route("/", axum::routing::get(get_handler).post(post_handler))
		.nest(
			"/:tag_category_id",
			delete::configure().merge(edit::configure()),
		)
}
