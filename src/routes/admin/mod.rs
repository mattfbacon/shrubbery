use crate::database::models::User;
use crate::helpers::auth::Admin;
use actix_web::{get, Responder};

pub mod sql;
pub mod users;

#[derive(askama::Template)]
#[template(path = "admin/index.html")]
struct Template {
	self_user: User,
}

#[get("/admin")]
pub async fn handler(Admin(self_user): Admin) -> impl Responder {
	Template { self_user }
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(handler);
	sql::configure(app);
	users::configure(app);
}
