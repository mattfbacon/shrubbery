use axum::{
	response::{IntoResponse, Response},
	routing::get,
	Router,
};

async fn favicon() -> impl IntoResponse {
	(
		Response::builder()
			.header("Content-Type", "image/x-icon")
			.body(())
			.unwrap(),
		std::include_bytes!("../../static/favicon.ico").as_slice(),
	)
}

async fn css() -> impl IntoResponse {
	(
		Response::builder()
			.header("Content-Type", "text/css")
			.body(())
			.unwrap(),
		std::include_bytes!("../../static/res/style/main.css").as_slice(),
	)
}

pub fn configure() -> Router {
	Router::new()
		.route("/favicon.ico", get(favicon))
		.route("/res/style/main.css", get(css))
}
