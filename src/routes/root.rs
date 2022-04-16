use crate::database::models::User;
use crate::helpers::auth::Auth;
use actix_web::Responder;
use askama::Template;

#[derive(Template)]
#[template(path = "home.html")]
struct WelcomeTemplate {
	user: User,
}

#[actix_web::get("/")]
pub async fn handler(Auth(user): Auth) -> impl Responder {
	WelcomeTemplate { user }
}
