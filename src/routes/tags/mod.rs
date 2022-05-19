use crate::database::{models, Database};
use crate::helpers::{auth, pagination};
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{web, Responder, ResponseError};

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
	auth::Auth(self_user): auth::Auth,
	web::Query(pagination): web::Query<pagination::Query>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let database = &**database;

	let num_pages = std::cmp::max(
		models::Tag::count(database).await? / pagination.page_size,
		1,
	);

	if pagination.page >= num_pages {
		return Err(Error::Page);
	}

	let tags = sqlx::query_as!(Tag, r#"SELECT tags.id, tags.name, tags.description, tag_categories.name as "category?", tags.created_time, users.username as "created_by?" FROM tags LEFT JOIN users ON tags.created_by = users.id LEFT JOIN tag_categories ON tags.category = tag_categories.id ORDER BY tags.id OFFSET $1 LIMIT $2"#, pagination.offset(), pagination.limit()).fetch_all(database).await?;

	let tag_categories = shared::get_tag_categories_lean(database).await?;

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
	pagination: web::Query<pagination::Query>,
	web::Form(req): web::Form<CreateRequest>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
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
	.insert(&**database)
	.await?;

	get_handler(auth::Auth(self_user), pagination, database).await
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(
		web::resource("/tags")
			.route(web::get().to(get_handler))
			.route(web::post().to(post_handler)),
	);
	delete::configure(app);
	edit::configure(app);
}
