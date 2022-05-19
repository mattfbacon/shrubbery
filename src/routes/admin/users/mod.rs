use crate::database::{models, Database};
use crate::helpers::auth::Admin;
use crate::helpers::pagination;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{web, Responder, ResponseError};

mod delete;
mod edit;

#[derive(askama::Template)]
#[template(path = "admin/users/index.html")]
struct Template {
	self_user: models::User,
	users: Vec<models::User>,
	pagination: pagination::Template,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid or nonexistent page")]
	Page,
	#[error("user not found")]
	NotFound,
	#[error("{0}")]
	Sqlx(#[from] sqlx::Error),
	#[error("password hash error")]
	Password,
}

impl ResponseError for Error {
	fn status_code(&self) -> HttpStatus {
		match self {
			Self::Page => HttpStatus::NOT_FOUND,
			Self::NotFound => HttpStatus::NOT_FOUND,
			Self::Sqlx(..) => HttpStatus::INTERNAL_SERVER_ERROR,
			Self::Password => HttpStatus::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> actix_web::HttpResponse {
		crate::routes::error::error_response(self)
	}
}

pub async fn get_handler(
	Admin(self_user): Admin,
	web::Query(pagination): web::Query<pagination::Query>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	use ormx::Table as _;
	let num_pages = std::cmp::max(
		models::User::count(&**database).await? / pagination.page_size(),
		1,
	);

	if pagination.page() >= num_pages {
		return Err(Error::Page);
	}

	let users =
		models::User::all_paginated(&**database, pagination.offset(), pagination.limit()).await?;

	Ok(Template {
		self_user,
		users,
		pagination: pagination::Template::from_query(pagination, num_pages),
	})
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(web::resource("/admin/users").route(web::get().to(get_handler)));
	delete::configure(app);
	edit::configure(app);
}
