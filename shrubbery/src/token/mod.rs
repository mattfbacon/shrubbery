use serde::{Deserialize, Serialize};
use time::Duration;

use crate::database::models::UserId;
use crate::timestamp::Timestamp;

pub mod crypt;
pub use crypt::Key;

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
	#[serde(with = "crate::timestamp::unix")]
	expires_at: Timestamp,
	pub user_id: UserId,
}

const TOKEN_EXPIRATION_TIME: Duration = Duration::days(1);

impl Token {
	pub fn new(user_id: UserId) -> Self {
		Self {
			user_id,
			expires_at: Timestamp::now() + TOKEN_EXPIRATION_TIME,
		}
	}

	pub fn is_expired(&self) -> bool {
		self.expires_at.is_in_past()
	}
}

pub static COOKIE_NAME: &str = "token";
