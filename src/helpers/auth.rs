use crate::database::models::{self, User};
use crate::helpers::remove_cookie;
use actix_web::dev::Payload;
use actix_web::http::StatusCode as HttpStatus;
use actix_web::web::Data as WebData;
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use std::future::Future;
use std::sync::Arc;

pub struct Auth(pub User);

struct Params {
	uri: String,
	token: Option<actix_web::cookie::Cookie<'static>>,
	database: Arc<crate::database::Database>,
	config: Arc<crate::config::Config>,
}

impl Auth {
	fn get_params(req: &HttpRequest) -> Params {
		let uri = req.uri().to_string();
		let token = req.cookie("token");
		let database = Arc::clone(
			req
				.app_data::<WebData<crate::database::Database>>()
				.expect("Could not get database from app data"),
		);
		let config = Arc::clone(
			req
				.app_data::<WebData<crate::config::Config>>()
				.expect("Could not get config from app data"),
		);
		Params {
			uri,
			token,
			database,
			config,
		}
	}
	async fn extract(
		Params {
			uri,
			token,
			database,
			config,
		}: Params,
	) -> Result<Self, AuthError> {
		let token = match token {
			Some(token) => token,
			None => return Err(AuthError::NoToken { redirect_to: uri }),
		};
		let token = crate::token::Token::decrypt(token.value(), &config.cookie_signing_key)
			.map_err(|_| AuthError::Invalid)?;
		let token = match token {
			Some(token) => token,
			None => return Err(AuthError::Expired { redirect_to: uri }),
		};
		let user_id = token.user_id;
		if let Some(user) = crate::database::models::User::by_id(&*database, user_id).await? {
			Ok(Self(user))
		} else {
			// this user ID was issued by us but is no longer valid, which means the user was deleted.
			Err(AuthError::UserDeleted { redirect_to: uri })
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
	#[error("no token cookie")]
	NoToken { redirect_to: String },
	#[error("you do not have permission to access this page")]
	Forbidden,
	#[error("invalid token")]
	Invalid,
	#[error("token was expired")]
	// we don't say "token does not exist" because signing the token protects us from the user providing arbitrary tokens
	Expired { redirect_to: String },
	#[error("user was deleted")]
	UserDeleted { redirect_to: String },
	#[error("sqlx error: {0}")]
	Sqlx(#[from] sqlx::Error),
}

impl AuthError {
	fn redirect_to(&self) -> Option<&str> {
		use AuthError::*;
		// be exhaustive in case we add another variant
		match self {
			NoToken { redirect_to } | Expired { redirect_to } | UserDeleted { redirect_to } => {
				Some(redirect_to)
			}
			Forbidden | Invalid | Sqlx(_) => None,
		}
	}

	fn should_remove_token(&self) -> bool {
		use AuthError::*;
		// ditto
		match self {
			NoToken { .. } | Expired { .. } | UserDeleted { .. } | Invalid => true,
			Forbidden | Sqlx(_) => false,
		}
	}
}

impl actix_web::ResponseError for AuthError {
	fn status_code(&self) -> HttpStatus {
		use AuthError::*;
		match self {
			// redirect to /login, which will redirect back to the current page after the user logs in
			NoToken { .. } | Expired { .. } | UserDeleted { .. } => HttpStatus::TEMPORARY_REDIRECT,
			Invalid => HttpStatus::BAD_REQUEST,
			Forbidden => HttpStatus::FORBIDDEN,
			Sqlx(_) => HttpStatus::INTERNAL_SERVER_ERROR,
		}
	}

	fn error_response(&self) -> HttpResponse {
		let mut builder = HttpResponse::build(self.status_code());
		if self.should_remove_token() {
			builder.cookie(remove_cookie("token"));
		}
		if let Some(redirect_to) = self.redirect_to() {
			debug_assert!(self.status_code().is_redirection());
			builder.insert_header((
				"Location",
				format!(
					"/login?return={}",
					crate::percent::percent_encode(redirect_to.as_bytes())
				),
			));
		}
		builder.body(self.to_string())
	}
}

impl FromRequest for Auth {
	type Error = AuthError;
	type Future = impl Future<Output = Result<Self, Self::Error>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		let params = Self::get_params(req);
		Self::extract(params)
	}
}

pub struct Admin(pub User);

impl FromRequest for Admin {
	type Error = <Auth as FromRequest>::Error;
	type Future = impl Future<Output = Result<Self, Self::Error>>;

	fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
		let params = Auth::get_params(req);
		async move {
			let Auth(user) = Auth::extract(params).await?;
			if user.role < models::UserRole::Admin {
				Err(AuthError::Forbidden)
			} else {
				Ok(Self(user))
			}
		}
	}
}
