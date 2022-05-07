pub mod error;
mod login;
mod logout;
mod register;
mod root;

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(root::handler);
	login::configure(app);
	register::configure(app);
	app.service(logout::handler);
	let files = actix_files::Files::new("/", "static")
		.default_handler(actix_web::dev::fn_service(error::default_handler));
	app.service(files);
}
