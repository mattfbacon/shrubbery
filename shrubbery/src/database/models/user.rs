use ormx::Table;

use super::{Id, UserPassword, UserRole};
use crate::timestamp::Timestamp;

pub type UserId = Id;

#[derive(Table)]
#[ormx(table = "users", insertable = Create, deletable)]
pub struct User {
	#[ormx(get_optional = by_id(UserId))]
	pub id: UserId,
	#[ormx(get_optional = by_username(&str), set)]
	pub username: String,
	#[ormx(custom_type, by_ref, set, set_as_wildcard)]
	pub password: UserPassword,
	#[ormx(set)]
	pub email: Option<String>,
	#[ormx(default, custom_type, set)]
	pub role: UserRole,
	#[ormx(default)]
	pub created_time: Timestamp,
	#[ormx(default, set)]
	pub last_login: Option<Timestamp>,
}

impl User {
	pub async fn count(
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
	) -> sqlx::Result<i64> {
		struct Helper {
			count: i64,
		}
		sqlx::query_as!(Helper, r#"SELECT count(*) as "count!" FROM users"#)
			.fetch_one(database)
			.await
			.map(|helper| helper.count)
	}
}
