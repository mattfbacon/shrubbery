use ormx::Table;

use super::{Color, UserId};
use crate::timestamp::Timestamp;

pub type Id = super::Id;

#[derive(Table)]
#[ormx(table = "tag_categories", insertable = Create, deletable)]
pub struct TagCategory {
	#[ormx(get_optional = by_id(Id))]
	pub id: Id,
	#[ormx(set)]
	pub name: String,
	#[ormx(set)]
	pub description: Option<String>,
	#[ormx(custom_type, set)]
	pub color: Color,
	#[ormx(custom_type, default)]
	pub created_time: Timestamp,
	pub created_by: Option<UserId>,
}

impl TagCategory {
	pub async fn count(
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
	) -> sqlx::Result<i64> {
		struct Helper {
			count: i64,
		}
		sqlx::query_as!(Helper, r#"SELECT count(*) as "count!" FROM tag_categories"#)
			.fetch_one(database)
			.await
			.map(|helper| helper.count)
	}
}
