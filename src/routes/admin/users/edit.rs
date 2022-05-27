use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::{auth, set_none_if_empty};

#[derive(askama::Template)]
#[template(path = "admin/users/edit.html")]
struct Template {
	updated: bool,
	self_user: models::User,
	requested_user: models::User,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	extract::Path((user_id,)): extract::Path<(models::UserId,)>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let requested_user = models::User::by_id(&*database, user_id)
		.await
		.map_err(error::Sqlx)?
		.ok_or(error::EntityNotFound("user"))?;
	Ok(Template {
		updated: false,
		self_user,
		requested_user,
	})
}

#[derive(Debug, serde::Deserialize)]
pub struct EditRequest {
	username: String,
	password: Option<String>,
	email: Option<String>,
	role: models::UserRole,
}

pub async fn post_handler(
	auth::Admin(self_user): auth::Admin,
	extract::Path((user_id,)): extract::Path<(models::UserId,)>,
	extract::Form(mut request): extract::Form<EditRequest>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let database = &*database;

	set_none_if_empty(&mut request.password);
	set_none_if_empty(&mut request.email);

	let mut requested_user = models::User::by_id(database, user_id)
		.await
		.map_err(error::Sqlx)?
		.ok_or(error::EntityNotFound("user"))?;
	requested_user
		.set_username(database, request.username)
		.await
		.map_err(error::Sqlx)?;
	if let Some(password) = request.password {
		requested_user
			.set_password(
				database,
				models::UserPassword::hash(&password).map_err(|_| error::PasswordHash)?,
			)
			.await
			.map_err(error::Sqlx)?;
	}
	requested_user
		.set_email(database, request.email)
		.await
		.map_err(error::Sqlx)?;
	requested_user
		.set_role(database, request.role)
		.await
		.map_err(error::Sqlx)?;
	Ok(Template {
		updated: true,
		self_user,
		requested_user,
	})
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler).post(post_handler))
}
