use crate::database::{models, Database};
use crate::helpers::{auth, set_none_if_empty};
use actix_web::{web, Responder};

use super::Error;

#[derive(askama::Template)]
#[template(path = "admin/tag_categories/edit.html")]
struct Template {
	updated: bool,
	self_user: models::User,
	requested_tag_category: models::TagCategory,
	created_by_username: Option<String>,
}

pub async fn get_handler(
	auth::Admin(self_user): auth::Admin,
	path: web::Path<(models::TagCategoryId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let (tag_category_id,) = path.into_inner();

	let requested_tag_category = models::TagCategory::by_id(&**database, tag_category_id)
		.await?
		.ok_or(Error::NotFound)?;

	let created_by_username = if let Some(created_by_id) = requested_tag_category.created_by {
		models::User::by_id(&**database, created_by_id).await?
	} else {
		None
	}
	.map(|user| user.username);

	Ok(Template {
		updated: false,
		self_user,
		requested_tag_category,
		created_by_username,
	})
}

#[derive(Debug, serde::Deserialize)]
pub struct EditRequest {
	name: String,
	description: Option<String>,
	color: models::Color,
}

pub async fn post_handler(
	auth::Admin(self_user): auth::Admin,
	path: web::Path<(models::UserId,)>,
	web::Form(mut request): web::Form<EditRequest>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let database = &**database;

	set_none_if_empty(&mut request.description);

	let (tag_category_id,) = path.into_inner();
	let mut requested_tag_category = models::TagCategory::by_id(database, tag_category_id)
		.await?
		.ok_or(Error::NotFound)?;
	requested_tag_category
		.set_name(database, request.name)
		.await?;
	requested_tag_category
		.set_description(database, request.description)
		.await?;
	requested_tag_category
		.set_color(database, request.color)
		.await?;

	let created_by_username = if let Some(created_by_id) = requested_tag_category.created_by {
		models::User::by_id(database, created_by_id).await?
	} else {
		None
	}
	.map(|user| user.username);

	Ok(Template {
		updated: true,
		self_user,
		requested_tag_category,
		created_by_username,
	})
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(
		web::resource("")
			.route(web::get().to(get_handler))
			.route(web::post().to(post_handler)),
	);
}
