use crate::database::{models, Database};
use crate::helpers::auth;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use ormx::Delete as _;

pub async fn post_handler(
	auth::Admin(_self_user): auth::Admin,
	req: HttpRequest,
	path: web::Path<(models::UserId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, super::Error> {
	let user_id = path.into_inner().0;
	models::User::delete_row(&**database, user_id).await?;
	Ok(
		HttpResponse::SeeOther()
			.insert_header(("Location", format!("/admin/users?{}", req.query_string())))
			.finish(),
	)
}

pub fn configure(app: &mut web::ServiceConfig) {
	app.service(web::resource("/admin/users/{user_id}/delete").route(web::post().to(post_handler)));
}
