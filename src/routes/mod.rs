mod error;
mod login;
mod logout;
mod register;
mod root;

use actix_web::web::ServiceConfig;
pub fn configure(app: &mut ServiceConfig) {
	app
		.service(root::handler)
		.service(login::get_handler)
		.service(login::post_handler)
		.service(register::get_handler)
		.service(register::post_handler)
		.service(logout::handler);
}

pub use error::error_handlers;
