use crate::database::{models, Database};
use crate::helpers::auth;
use crate::viewspec::ViewSpec;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::{web, Responder};

#[derive(askama::Template)]
#[template(path = "index.html")]
struct Template {
	self_user: models::User,
	search_results: Option<Vec<(models::FileId, String)>>,
	page_size: i64,
}

#[derive(serde::Deserialize)]
pub struct Query {
	viewspec: Option<ViewSpec>,
	after: Option<models::FileId>,
	#[serde(default = "crate::helpers::pagination::default_page_size")]
	page_size: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("{0}")]
	Sqlx(#[from] sqlx::Error),
}

impl actix_web::ResponseError for Error {
	fn status_code(&self) -> HttpStatus {
		match self {
			Self::Sqlx(..) => HttpStatus::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> actix_web::HttpResponse {
		crate::routes::error::error_response(self)
	}
}

pub async fn get_handler(
	auth::Auth(self_user): auth::Auth,
	web::Query(Query {
		viewspec,
		after,
		page_size,
	}): web::Query<Query>,
	database: web::Data<Database>,
) -> Result<impl Responder, Error> {
	let search_results = match viewspec {
		Some(viewspec) => Some(viewspec.evaluate(&**database, after, page_size).await?),
		None => None,
	};
	Ok(Template {
		self_user,
		search_results,
		page_size,
	})
}

pub fn configure(app: &mut actix_web::web::ServiceConfig) {
	app.service(web::resource("/").route(web::get().to(get_handler)));
}
