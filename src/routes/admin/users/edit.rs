use crate::database::{models, Database};
use crate::helpers::{auth, set_none_if_empty};
use actix_web::{get, post, web, Responder};

use super::Error;

#[derive(askama::Template)]
#[template(path = "admin/users/edit.html")]
struct Template {
	updated: bool,
	self_user: models::User,
	requested_user: models::User,
}

#[get("/admin/users/{user_id}")]
pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	path: web::Path<(models::UserId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let (user_id,) = path.into_inner();
	let requested_user = models::User::by_id(&**database, user_id)
		.await?
		.ok_or(Error::NotFound)?;
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

#[post("/admin/users/{user_id}")]
pub async fn post_handler(
	auth::Admin(self_user): auth::Admin,
	path: web::Path<(models::UserId,)>,
	web::Form(mut request): web::Form<EditRequest>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let database = &**database;

	set_none_if_empty(&mut request.password);
	set_none_if_empty(&mut request.email);

	let (user_id,) = path.into_inner();
	let mut requested_user = models::User::by_id(database, user_id)
		.await?
		.ok_or(Error::NotFound)?;
	requested_user
		.set_username(database, request.username)
		.await?;
	if let Some(password) = request.password {
		requested_user
			.set_password(
				database,
				models::UserPassword::hash(&password).map_err(|_| Error::Password)?,
			)
			.await?;
	}
	requested_user.set_email(database, request.email).await?;
	requested_user.set_role(database, request.role).await?;
	Ok(Template {
		updated: true,
		self_user,
		requested_user,
	})
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(get_handler);
	app.service(post_handler);
}
