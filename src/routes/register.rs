use crate::database::{models, Database};
use crate::helpers::{internal_server_error, set_none_if_empty};
use crate::percent::PercentEncodedString;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::web::{Data, Form, Query};
use actix_web::{get, post, Either, HttpResponse, Responder};
use serde::Deserialize;

use super::login::ReturnUrl;

#[derive(askama::Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
	error: Option<String>,
	return_url: Option<PercentEncodedString>,
}

#[get("/register")]
pub async fn get_handler(Query(ReturnUrl { return_url }): Query<ReturnUrl>) -> impl Responder {
	RegisterTemplate {
		error: None,
		return_url,
	}
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
	username: String,
	password: String,
	confirm_password: String,
	email: Option<String>,
}

#[post("/register")]
pub async fn post_handler(
	Form(mut request): Form<RegisterRequest>,
	Query(ReturnUrl { return_url }): Query<ReturnUrl>,
	database: Data<Database>,
) -> actix_web::Result<Either</* private */ impl Responder, HttpResponse>> {
	use ormx::Insert as _;
	macro_rules! err {
		($($tok:tt)+) => {
			Ok(Either::Left(RegisterTemplate { error: Some(format!($($tok)*)), return_url }))
		};
	}

	set_none_if_empty(&mut request.email);
	if request.password != request.confirm_password {
		return err!("Passwords do not match");
	}

	if models::User::by_username(&**database, &request.username)
		.await
		.map_err(internal_server_error)?
		.is_some()
	{
		return err!("Username taken");
	}

	models::InsertUser {
		username: request.username,
		password: models::UserPassword::hash(&request.password).map_err(internal_server_error)?,
		email: request.email,
	}
	.insert(&**database)
	.await
	.map_err(internal_server_error)?;

	let redirect_url = match return_url {
		Some(return_url) => format!("/login?return={}", return_url),
		None => "/login".to_owned(),
	};
	let response = HttpResponse::build(HttpStatus::SEE_OTHER)
		.insert_header(("Location", redirect_url))
		.finish();
	Ok(Either::Right(response))
}
