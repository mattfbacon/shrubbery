mod _static;
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
	_static::configure(app);

	sub!(app; admin, login, logout, register, tags);
}
