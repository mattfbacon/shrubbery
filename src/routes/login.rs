use crate::config::Config;
use crate::database::models;
use crate::database::Database;
use crate::helpers::internal_server_error;
use crate::percent::PercentEncodedString;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::web::{Data, Form, Query};
use actix_web::{get, post, Either, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ReturnUrl {
	#[serde(rename = "return")]
	pub return_url: Option<PercentEncodedString>,
}

#[derive(askama::Template)]
#[template(path = "login.html")]
struct LoginTemplate {
	error: Option<String>,
	return_url: Option<PercentEncodedString>,
}

#[get("/login")]
pub async fn get_handler(Query(ReturnUrl { return_url }): Query<ReturnUrl>) -> impl Responder {
	LoginTemplate {
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

#[post("/login")]
pub async fn post_handler(
	Form(LoginRequest {
		username,
		password,
		keep_logged_in,
	}): Form<LoginRequest>,
	Query(ReturnUrl { return_url }): Query<ReturnUrl>,
	database: Data<Database>,
	config: Data<Config>,
) -> actix_web::Result<Either</* private */ impl Responder, HttpResponse>> {
	macro_rules! err {
		($($tok:tt)+) => {
			Ok(Either::Left(LoginTemplate { error: Some(format!($($tok)*)), return_url }))
		};
	}

	let user = models::User::by_username(&**database, &username)
		.await
		.map_err(internal_server_error)?;
	let mut user = match user {
		Some(user) => user,
		None => return err!("Unknown user {:?}", username),
	};
	if !user
		.verify_password(&password)
		.map_err(internal_server_error)?
	{
		return err!("Invalid password");
	}

	user
		.set_last_login(&**database, Some(crate::timestamp::now()))
		.await
		.map_err(internal_server_error)?;

	let token = crate::token::Token::new(user.id);
	let token_cookie = token
		.encrypt_to_cookie(&config.cookie_signing_key, keep_logged_in)
		.map_err(internal_server_error)?;

	Ok(Either::Right(
		HttpResponse::build(HttpStatus::SEE_OTHER)
			.insert_header((
				"Location",
				return_url
					.map(|PercentEncodedString(decoded)| decoded)
					.unwrap_or_else(|| "/".to_owned()),
			))
			.cookie(token_cookie)
			.body("redirecting shortly!"),
	))
}
