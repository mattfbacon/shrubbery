mod admin;
pub mod error;
mod login;
mod logout;
mod register;
mod root;
mod tags;

macro_rules! sub {
	($app:ident, $name:ident) => {
		$app.service(actix_web::web::scope(concat!("/", stringify!($name))).configure($name::configure))
	};
	($app:ident; $($name:ident),+) => {
		$(sub!($app, $name));+
	};
}
use sub;

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	root::configure(app);

	sub!(app; admin, login, logout, register, tags);

	let files = actix_files::Files::new("/", "static")
		.default_handler(actix_web::dev::fn_service(error::default_handler));
	app.service(files);
}
