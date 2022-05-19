use crate::database::models::User;
use crate::helpers::auth::Auth;
use actix_web::Responder;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct WelcomeTemplate {
	self_user: User,
}

#[actix_web::get("/")]
pub async fn handler(Auth(self_user): Auth) -> impl Responder {
	WelcomeTemplate { self_user }
}
