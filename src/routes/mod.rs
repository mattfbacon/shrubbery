mod error;
mod login;
mod logout;
mod register;
mod root;

use actix_web::web::ServiceConfig;
pub fn configure(app: &mut ServiceConfig) {
	app.service(root::handler);
	app.service(login::get_handler);
	app.service(login::post_handler);
	app.service(login::post_handler);
	app.service(register::get_handler);
	app.service(logout::handler);
}

pub use error::error_handlers;
