use crate::database::{models, Database};
use crate::helpers::auth;
use actix_web::web::{self, ServiceConfig};
use actix_web::{HttpRequest, HttpResponse, Responder};
use ormx::Delete as _;

pub async fn post_handler(
	auth::Admin(_self_user): auth::Admin,
	req: HttpRequest,
	path: web::Path<(models::TagCategoryId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, super::Error> {
	let tag_category_id = path.into_inner().0;
	models::TagCategory::delete_row(&**database, tag_category_id).await?;
	Ok(
		HttpResponse::SeeOther()
			.insert_header(("Location", format!("/admin/users?{}", req.query_string())))
			.finish(),
	)
}

pub fn configure(app: &mut ServiceConfig) {
	app.service(
		web::resource("/admin/tag_categories/{user_id}/delete").route(web::post().to(post_handler)),
	);
}
