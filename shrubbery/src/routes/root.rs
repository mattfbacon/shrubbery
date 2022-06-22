use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth;
use crate::viewspec::ViewSpec;

#[derive(askama::Template)]
#[template(path = "index.html")]
struct Template {
	self_user: models::User,
	search_results: Option<Vec<(models::FileId, String)>>,
	page_size: i64,
}
crate::helpers::impl_into_response!(Template);

#[derive(serde::Deserialize)]
pub struct Query {
	#[serde(rename = "search")]
	viewspec: Option<ViewSpec>,
	after: Option<models::FileId>,
	#[serde(default = "crate::helpers::pagination::default_page_size")]
	page_size: i64,
}

pub async fn get_handler(
	auth::Auth(self_user): auth::Auth,
	extract::Query(Query {
		viewspec,
		after,
		page_size,
	}): extract::Query<Query>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<impl IntoResponse, ErrorResponse> {
	let search_results = match viewspec {
		Some(viewspec) => Some(
			viewspec
				.evaluate(&*database, after, page_size)
				.await
				.map_err(error::Sqlx)?,
		),
		None => None,
	};
	Ok(Template {
		self_user,
		search_results,
		page_size,
	})
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler))
}
