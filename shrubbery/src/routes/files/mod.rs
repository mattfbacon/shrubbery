use axum::Router;

pub mod id;

pub fn configure() -> Router {
	let mut app = Router::new();

	app = app.nest("/:file_id", id::configure());

	app
}
