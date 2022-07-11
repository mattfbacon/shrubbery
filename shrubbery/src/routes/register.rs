use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};
use serde::Deserialize;

use super::login::ReturnUrl;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::percent::EncodedString as PercentEncodedString;
use crate::helpers::set_none_if_empty;

#[derive(askama::Template)]
#[template(path = "register.html")]
struct Template {
	error: Option<String>,
	return_url: Option<PercentEncodedString>,
}
crate::helpers::impl_into_response!(Template);

pub async fn get_handler(
	extract::Query(ReturnUrl { return_url }): extract::Query<ReturnUrl>,
) -> impl IntoResponse {
	Template {
		error: None,
		return_url,
	}
}

#[derive(Debug, Deserialize)]
pub struct PostRequest {
	username: String,
	password: String,
	confirm_password: String,
	email: Option<String>,
}

pub async fn post_handler(
	extract::Form(mut request): extract::Form<PostRequest>,
	extract::Query(ReturnUrl { return_url }): extract::Query<ReturnUrl>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
) -> Result<Response, ErrorResponse> {
	use ormx::Insert as _;
	macro_rules! err {
		($($tok:tt)+) => {
			Ok(Template { error: Some(format!($($tok)+)), return_url }.into_response())
		};
	}

	set_none_if_empty(&mut request.email);
	if request.password != request.confirm_password {
		return err!("Passwords do not match");
	}

	if models::User::by_username(&*database, &request.username)
		.await
		.map_err(error::Sqlx)?
		.is_some()
	{
		return err!("Username taken");
	}

	models::user::Create {
		username: request.username,
		password: models::UserPassword::hash(&request.password).map_err(|_| error::PasswordHash)?,
		email: request.email,
	}
	.insert(&*database)
	.await
	.map_err(error::Sqlx)?;

	// construct `Redirect` inside `match` due to `String` vs `str`
	let redirect = match return_url {
		Some(return_url) => Redirect::to(&format!("/login?return={}", return_url)),
		None => Redirect::to("/login"),
	};
	Ok(redirect.into_response())
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler).post(post_handler))
}
