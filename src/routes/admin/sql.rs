use crate::database::Database;
use crate::helpers::auth::Admin;
use actix_web::{web, Responder};

#[derive(serde::Deserialize)]
pub struct PostData {
	sql: String,
}

pub async fn post_handler(
	Admin(_self_user): Admin,
	web::Form(PostData { sql }): web::Form<PostData>,
	database: web::Data<Database>,
) -> impl Responder {
	let result = sqlx::query(&sql).execute(&**database).await;
	format!("{:#?}", result)
}

pub fn configure(app: &mut web::ServiceConfig) {
	app.service(web::resource("/admin/sql").route(web::post().to(post_handler)));
}
