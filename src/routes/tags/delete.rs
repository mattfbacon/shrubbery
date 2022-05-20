use crate::database::{models, Database};
use crate::helpers::auth;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use ormx::Delete as _;

pub async fn post_handler(
	_self_user: auth::Editor,
	req: HttpRequest,
	path: web::Path<(models::TagId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, super::Error> {
	let tag_id = path.into_inner().0;
	models::Tag::delete_row(&**database, tag_id).await?;
	Ok(
		HttpResponse::SeeOther()
			.insert_header(("Location", format!("/tags?{}", req.query_string())))
			.finish(),
	)
}

pub fn configure(app: &mut web::ServiceConfig) {
	app.service(web::resource("/delete").route(web::post().to(post_handler)));
}
