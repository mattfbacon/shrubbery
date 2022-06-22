use axum::response::IntoResponse;
use axum::Router;

use crate::database::models::User;
use crate::helpers::auth::Admin;

pub mod sql;
pub mod tag_categories;
pub mod users;

#[derive(askama::Template)]
#[template(path = "admin/index.html")]
struct Template {
	self_user: User,
}

crate::helpers::impl_into_response!(Template);

pub async fn handler(Admin(self_user): Admin) -> impl IntoResponse {
	Template { self_user }
}

use super::sub;

pub fn configure() -> Router {
	let mut app = Router::new();
	app = app.route("/", axum::routing::get(handler));
	sub!(app; sql, tag_categories, users);
	app
}
