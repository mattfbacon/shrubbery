use chrono::Duration;
use cookie::Cookie;
use serde::{Deserialize, Serialize};

use crate::database::models::UserId;
use crate::timestamp::{self, Timestamp};

pub mod crypt;
pub use crypt::Key;

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
	#[serde(with = "chrono::serde::ts_seconds")]
	expires_at: Timestamp,
	pub user_id: UserId,
}

impl Token {
	pub fn new(user_id: UserId) -> Self {
		let token_expiration_time = Duration::days(1); // not const
		Self {
			user_id,
			expires_at: timestamp::now() + token_expiration_time,
		}
	}

	pub fn is_expired(&self) -> bool {
		timestamp::is_in_past(&self.expires_at)
	}
}

pub fn remove_cookie() -> Cookie<'static> {
	let mut cookie = Cookie::named("token");
	cookie.make_removal();
	cookie
}
