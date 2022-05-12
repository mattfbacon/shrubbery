mod admin;
pub mod error;
mod login;
mod logout;
mod register;
mod root;

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(root::handler);
	admin::configure(app);
	login::configure(app);
	logout::configure(app);
	register::configure(app);
	let files = actix_files::Files::new("/", "static")
		.default_handler(actix_web::dev::fn_service(error::default_handler));
	app.service(files);
}
