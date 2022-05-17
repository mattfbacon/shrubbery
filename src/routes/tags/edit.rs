use crate::database::{models, Database};
use crate::helpers::{auth, set_none_if_empty, OrNull};
use actix_web::{web, Responder};

use super::Error;

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

pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	path: web::Path<(models::TagId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let database = &**database;
	let (tag_id,) = path.into_inner();

	let requested_tag = Tag::by_id(database, tag_id).await?.ok_or(Error::NotFound)?;

	let tag_categories = super::shared::get_tag_categories_lean(database).await?;

	Ok(Template {
		updated: false,
		self_user,
		requested_tag,
		tag_categories,
	})
}

#[derive(Debug, serde::Deserialize)]
pub struct EditRequest {
	name: String,
	description: Option<String>,
	category: OrNull<models::TagCategoryId>,
}

pub async fn post_handler(
	auth::Editor(self_user): auth::Editor,
	path: web::Path<(models::TagId,)>,
	web::Form(mut request): web::Form<EditRequest>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let database = &**database;

	set_none_if_empty(&mut request.description);

	let (tag_id,) = path.into_inner();

	sqlx::query!(
		"UPDATE tags SET name = $2, description = $3, category = $4 WHERE id = $1",
		tag_id,
		&request.name,
		request.description.as_ref(),
		request.category.into_option()
	)
	.execute(database)
	.await?;

	let requested_tag = Tag::by_id(database, tag_id).await?.ok_or(Error::NotFound)?;

	let tag_categories = super::shared::get_tag_categories_lean(database).await?;

	Ok(Template {
		updated: true,
		self_user,
		requested_tag,
		tag_categories,
	})
}

pub fn configure(app: &mut web::ServiceConfig) {
	app.service(
		web::resource("/tags/{tag_id}")
			.route(web::get().to(get_handler))
			.route(web::post().to(post_handler)),
	);
}
