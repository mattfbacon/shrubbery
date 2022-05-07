use crate::database::{models, Database};
use crate::helpers::auth;
use actix_web::error::InternalError;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::web::{self, ServiceConfig};
use actix_web::{post, HttpRequest, HttpResponse, Responder};
use ormx::Delete as _;

#[post("/admin/users/{user_id}/delete")]
pub async fn post_handler(
	auth::Admin(_self_user): auth::Admin,
	req: HttpRequest,
	path: web::Path<(models::UserId,)>,
	database: web::Data<Database>,
) -> Result<impl Responder, actix_web::Error> {
	let user_id = path.into_inner().0;
	models::User::delete_row(&**database, user_id)
		.await
		.map_err(|err| InternalError::new(err, HttpStatus::INTERNAL_SERVER_ERROR))?;
	Ok(
		HttpResponse::SeeOther()
			.insert_header(("Location", format!("/admin/users?{}", req.query_string())))
			.finish(),
	)
}

pub fn configure(app: &mut ServiceConfig) {
	app.service(post_handler);
}
