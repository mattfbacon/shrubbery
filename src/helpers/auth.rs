use std::sync::Arc;

use axum::async_trait;
use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use axum::response::{IntoResponse, Response};
use headers::HeaderMapExt as _;
use http::StatusCode;

use crate::database::models::{self, User};

pub struct Auth(pub User);

struct Params {
	uri: String,
	token: Option<String>,
	database: Arc<crate::database::Database>,
	config: Arc<crate::config::Config>,
}

impl Auth {
	fn get_params(req: &RequestParts<Body>) -> Params {
		let uri = req.uri().to_string();
		let token = req
			.headers()
			.typed_get::<headers::Cookie>()
			.and_then(|cookies| cookies.get("token").map(|token| token.to_owned()));
		let database = Arc::clone(
			req
				.extensions()
				.get::<Arc<crate::database::Database>>()
				.expect("Could not get database from app data"),
		);
		let config = Arc::clone(
			req
				.extensions()
				.get::<Arc<crate::config::Config>>()
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
		let token = crate::token::Token::decrypt(&token, &config.cookie_signing_key)
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

	fn status_code(&self) -> StatusCode {
		use AuthError::*;
		match self {
			// redirect to /login, which will redirect back to the current page after the user logs in
			NoToken { .. } | Expired { .. } | UserDeleted { .. } => StatusCode::SEE_OTHER,
			Invalid => StatusCode::BAD_REQUEST,
			Forbidden => StatusCode::FORBIDDEN,
			Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

impl IntoResponse for AuthError {
	fn into_response(self) -> Response {
		let mut builder = Response::builder().status(self.status_code());

		if self.should_remove_token() {
			builder.headers_mut().unwrap().insert(
				"Set-Cookie",
				crate::token::remove_cookie()
					.encoded()
					.to_string()
					.parse()
					.unwrap(),
			);
		}

		if let Some(redirect_to) = self.redirect_to() {
			debug_assert!(self.status_code().is_redirection());
			builder = builder.header(
				"Location",
				format!(
					"/login?return={}",
					crate::percent::percent_encode(redirect_to.as_bytes())
				),
			);
		}

		builder
			.body(axum::body::boxed(http_body::Full::from(self.to_string())))
			.unwrap()
	}
}

#[async_trait]
impl FromRequest<Body> for Auth {
	type Rejection = AuthError;

	async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
		let params = Self::get_params(req);
		Self::extract(params).await
	}
}

macro_rules! role_extractor {
	($both:ident) => {
		role_extractor!($both, $both);
	};
	($extractor_name:ident, $min_role:ident) => {
		pub struct $extractor_name(pub User);

		#[async_trait]
		impl FromRequest<Body> for $extractor_name {
			type Rejection = AuthError;

			async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
				let params = Auth::get_params(req);
				let Auth(user) = Auth::extract(params).await?;
				if user.role < models::UserRole::$min_role {
					Err(AuthError::Forbidden)
				} else {
					Ok(Self(user))
				}
			}
		}
	};
}

role_extractor!(Admin);
role_extractor!(Editor);
