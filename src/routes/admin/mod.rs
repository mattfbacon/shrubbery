use crate::database::models::User;
use crate::helpers::auth::Admin;
use actix_web::{web, Responder};

pub mod sql;
pub mod tag_categories;
pub mod users;

#[derive(askama::Template)]
#[template(path = "admin/index.html")]
struct Template {
	self_user: User,
}

pub async fn handler(Admin(self_user): Admin) -> impl Responder {
	Template { self_user }
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(web::resource("/admin").route(web::get().to(handler)));
	sql::configure(app);
	tag_categories::configure(app);
	users::configure(app);
}
