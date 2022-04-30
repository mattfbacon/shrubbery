mod error;
mod login;
mod logout;
mod register;
mod root;

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(root::handler);
	app.service(login::get_handler);
	app.service(login::post_handler);
	app.service(login::post_handler);
	app.service(register::get_handler);
	app.service(logout::handler);
	let files = actix_files::Files::new("/", "static");
	app.service(files);
}

pub use error::error_handlers;
