mod error;
mod login;
mod register;
mod root;

use actix_web::web::ServiceConfig;
pub fn configure(app: &mut ServiceConfig) {
	app
		.service(root::handler)
		.service(login::get_handler)
		.service(login::post_handler)
		.service(register::get_handler)
		.service(register::post_handler);
}

pub use error::error_handlers;
