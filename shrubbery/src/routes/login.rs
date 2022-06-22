use std::sync::Arc;

use axum::response::{ErrorResponse, IntoResponse, Redirect, Response};
use axum::{extract, Router};
use serde::Deserialize;

use crate::config::Config;
use crate::database::{models, Database};
use crate::error;
use crate::helpers::cookie::CookiePart;
use crate::percent::PercentEncodedString;

#[derive(Deserialize)]
pub struct ReturnUrl {
	#[serde(rename = "return")]
	pub return_url: Option<PercentEncodedString>,
}

#[derive(askama::Template)]
#[template(path = "login.html")]
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

#[derive(Deserialize)]
pub struct LoginRequest {
	username: String,
	password: String,
	#[serde(default = "default_keep_logged_in")]
	keep_logged_in: bool,
}

const fn default_keep_logged_in() -> bool {
	false
}

pub async fn post_handler(
	extract::Form(LoginRequest {
		username,
		password,
		keep_logged_in,
	}): extract::Form<LoginRequest>,
	extract::Query(ReturnUrl { return_url }): extract::Query<ReturnUrl>,
	extract::Extension(database): extract::Extension<Arc<Database>>,
	extract::Extension(config): extract::Extension<Arc<Config>>,
) -> Result<Response, ErrorResponse> {
	macro_rules! err {
		($($tok:tt)+) => {
			Ok(Template { error: Some(format!($($tok)+)), return_url }.into_response())
		};
	}

	let database = &*database;

	let user = models::User::by_username(database, &username)
		.await
		.map_err(error::Sqlx)?;
	let mut user = match user {
		Some(user) => user,
		None => return err!("Unknown user {:?}", username),
	};
	if !user
		.verify_password(&password)
		.map_err(|_| error::PasswordHash)?
	{
		return err!("Invalid password");
	}

	user
		.set_last_login(database, Some(crate::timestamp::now()))
		.await
		.map_err(error::Sqlx)?;

	let token = crate::token::Token::new(user.id);
	let token_cookie = token
		.encrypt_to_cookie(&config.cookie_signing_key, keep_logged_in)
		.map_err(error::Encrypt)?;

	Ok(
		(
			CookiePart(token_cookie),
			Redirect::to(
				&return_url
					.map(|PercentEncodedString(decoded)| decoded)
					.unwrap_or_else(|| "/".to_owned()),
			),
		)
			.into_response(),
	)
}

pub fn configure() -> Router {
	Router::new().route("/", axum::routing::get(get_handler).post(post_handler))
}
