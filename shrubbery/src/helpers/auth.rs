use std::sync::Arc;

use axum::async_trait;
use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use axum::response::{IntoResponse, Response};
use headers::HeaderMapExt as _;
use http::StatusCode;

use crate::database::models::{self, User};
use crate::helpers::cookie::Part as CookiePart;

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
			.and_then(|cookies| cookies.get("token").map(str::to_owned));
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
	) -> Result<Self> {
		let token = match token {
			Some(token) => token,
			None => return Err(Error::NoToken { redirect_to: uri }),
		};
		let token = crate::token::Token::decrypt(&token, &config.cookie_signing_key)
			.map_err(|_| Error::Invalid)?;
		let token = match token {
			Some(token) => token,
			None => return Err(Error::Expired { redirect_to: uri }),
		};
		let user_id = token.user_id;
		if let Some(user) = crate::database::models::User::by_id(&*database, user_id).await? {
			Ok(Self(user))
		} else {
			// this user ID was issued by us but is no longer valid, which means the user was deleted.
			Err(Error::UserDeleted { redirect_to: uri })
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

pub type Result<T, E = Error> = core::result::Result<T, E>;

impl Error {
	fn redirect_to(&self) -> Option<&str> {
		// be exhaustive in case we add another variant
		match self {
			Self::NoToken { redirect_to }
			| Self::Expired { redirect_to }
			| Self::UserDeleted { redirect_to } => Some(redirect_to),
			Self::Forbidden | Self::Invalid | Self::Sqlx(_) => None,
		}
	}

	fn should_remove_token(&self) -> bool {
		// ditto
		match self {
			Self::NoToken { .. } | Self::Expired { .. } | Self::UserDeleted { .. } | Self::Invalid => {
				true
			}
			Self::Forbidden | Self::Sqlx(_) => false,
		}
	}

	fn status_code(&self) -> StatusCode {
		match self {
			// redirect to /login, which will redirect back to the current page after the user logs in
			Self::NoToken { .. } | Self::Expired { .. } | Self::UserDeleted { .. } => {
				StatusCode::SEE_OTHER
			}
			Self::Invalid => StatusCode::BAD_REQUEST,
			Self::Forbidden => StatusCode::FORBIDDEN,
			Self::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}

impl IntoResponse for Error {
	fn into_response(self) -> Response {
		let mut builder = Response::builder().status(self.status_code());

		if self.should_remove_token() {
			builder = CookiePart::new_removal(crate::token::COOKIE_NAME).onto_builder(builder);
		}

		if let Some(redirect_to) = self.redirect_to() {
			debug_assert!(self.status_code().is_redirection());
			builder = builder.header(
				"Location",
				format!(
					"/login?return={}",
					super::percent::encode(redirect_to.as_bytes())
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
	type Rejection = Error;

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
			type Rejection = Error;

			async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
				let params = Auth::get_params(req);
				let Auth(user) = Auth::extract(params).await?;
				if user.role < models::UserRole::$min_role {
					Err(Error::Forbidden)
				} else {
					Ok(Self(user))
				}
			}
		}
	};
}

role_extractor!(Admin);
role_extractor!(Editor);
