use axum::Router;

mod _static;
mod admin;
mod login;
mod logout;
mod register;
mod root;
mod tags;

macro_rules! sub {
	($app:ident, $name:ident) => {
		$app = $app.nest(concat!("/", stringify!($name)), $name::configure())
	};
	($app:ident; $($name:ident),+) => {
		$(sub!($app, $name));+
	};
}
pub(crate) use sub;

macro_rules! merge {
	($app:ident, $name:ident) => {
		$app = $app.merge($name::configure());
	};
	($app:ident; $($name:ident),+) => {
		$(merge!($app, $name));+
	};
}

pub fn configure() -> Router {
	let mut app = Router::new();

	merge!(app; root, _static);
	sub!(app; admin, login, logout, register, tags);

	app = app.fallback(axum::handler::Handler::into_service(
		crate::error::default_handler,
	));

	app
}
