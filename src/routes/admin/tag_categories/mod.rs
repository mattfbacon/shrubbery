use crate::database::{models, Database};
use crate::helpers::auth::Admin;
use crate::helpers::pagination;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{web, Responder, ResponseError};

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

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid or nonexistent page")]
	Page,
	#[error("user not found")]
	NotFound,
	#[error("{0}")]
	Sqlx(#[from] sqlx::Error),
}

impl ResponseError for Error {
	fn status_code(&self) -> HttpStatus {
		match self {
			Self::Page => HttpStatus::NOT_FOUND,
			Self::NotFound => HttpStatus::NOT_FOUND,
			Self::Sqlx(..) => HttpStatus::INTERNAL_SERVER_ERROR,
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
	let num_pages = std::cmp::max(
		models::TagCategory::count(&**database).await? / pagination.page_size(),
		1,
	);
	if pagination.page() >= num_pages {
		return Err(Error::Page);
	}

	let tag_categories = sqlx::query_as!(TagCategoryWithUserResolved, r#"SELECT tag_categories.id, tag_categories.name, tag_categories.description, tag_categories.color as "color: models::Color", tag_categories.created_time, users.username as "created_by?" FROM tag_categories LEFT JOIN users ON tag_categories.created_by = users.id ORDER BY tag_categories.id OFFSET $1 LIMIT $2"#, pagination.offset(), pagination.limit()).fetch_all(&**database).await?;

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
	web::Query(pagination): web::Query<pagination::Query>,
	web::Form(req): web::Form<CreateRequest>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
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
	.await?;

	get_handler(Admin(self_user), web::Query(pagination), database).await
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(
		web::resource("/admin/tag_categories")
			.route(web::get().to(get_handler))
			.route(web::post().to(post_handler)),
	);
	delete::configure(app);
	edit::configure(app);
}
