use crate::database::Database;
use crate::helpers::auth::Admin;
use actix_web::web::{self, ServiceConfig};
use actix_web::{post, Responder};

#[derive(serde::Deserialize)]
pub struct PostData {
	sql: String,
}

#[post("/admin/sql")]
pub async fn post_handler(
	Admin(self_user): Admin,
	web::Form(PostData { sql }): web::Form<PostData>,
	database: web::Data<Database>,
) -> impl Responder {
	let result = sqlx::query(&sql).execute(&**database).await;
	format!("{:#?}", result)
}

pub fn configure(app: &mut ServiceConfig) {
	app.service(post_handler);
}
