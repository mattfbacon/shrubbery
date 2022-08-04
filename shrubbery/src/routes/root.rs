use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse};
use axum::{extract, Router};

use crate::database::{models, Database};
use crate::error;
use crate::helpers::auth;
use crate::helpers::viewspec::{Error as ViewSpecError, ViewSpecOrError};

#[derive(askama::Template)]
#[template(path = "index.html")]
struct Template {
	self_user: models::User,
	search_results: Option<Result<Vec<(models::FileId, String)>, ViewSpecError>>,
	page_size: i64,
}
crate::helpers::impl_into_response!(Template);

#[derive(serde::Deserialize)]
pub struct Query {
	#[serde(rename = "search")]
	viewspec: Option<ViewSpecOrError>,
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
	let search_results = match viewspec.map(|ViewSpecOrError(result)| result) {
		Some(Ok(viewspec)) => Some(Ok(
			crate::eval_viewspec::evaluate(&viewspec, &*database, after, page_size)
				.await
				.map_err(error::Sqlx)?,
		)),
		Some(Err(error)) => Some(Err(error)),
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
