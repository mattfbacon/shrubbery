use crate::database::models::User;
use crate::helpers::auth::Auth;
use actix_web::{web, Responder};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct WelcomeTemplate {
	self_user: User,
}

pub async fn handler(Auth(self_user): Auth) -> impl Responder {
	WelcomeTemplate { self_user }
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(web::resource("/").route(web::get().to(handler)));
}
