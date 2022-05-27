use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth::Admin;
use crate::helpers::pagination;

mod delete;
mod edit;

#[derive(askama::Template)]
#[template(path = "admin/users/index.html")]
struct Template {
	self_user: models::User,
	users: Vec<models::User>,
	pagination: pagination::Template,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	Admin(self_user): Admin,
	extract::Query(pagination): extract::Query<pagination::Query>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	use ormx::Table as _;
	let num_pages = std::cmp::max(
		models::User::count(&*database).await.map_err(error::Sqlx)? / pagination.page_size(),
		1,
	);

	if pagination.page() >= num_pages {
		return Err(error::EntityNotFound("page").into());
	}

	let users = models::User::all_paginated(&*database, pagination.offset(), pagination.limit())
		.await
		.map_err(error::Sqlx)?;

	Ok(Template {
		self_user,
		users,
		pagination: pagination::Template::from_query(pagination, num_pages),
	})
}

pub fn configure() -> Router {
	Router::new()
		.route("/", axum::routing::get(get_handler))
		.nest("/:user_id", delete::configure().merge(edit::configure()))
}
