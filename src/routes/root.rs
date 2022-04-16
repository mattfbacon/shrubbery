use crate::helpers::auth::Auth;
use actix_web::Responder;
use askama::Template;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeTemplate {}

#[actix_web::get("/")]
pub async fn handler(user: Auth) -> impl Responder {
	todo!();
	WelcomeTemplate {}
}
