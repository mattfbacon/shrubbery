use super::{Id, TagCategoryId, UserId};
use crate::timestamp::Timestamp;
use ormx::Table;

pub type TagId = Id;

#[derive(Table)]
#[ormx(table = "tags", insertable = Create, deletable)]
pub struct Tag {
	#[ormx(get_optional = by_id(TagId))]
	pub id: TagId,
	pub name: String,
	pub description: Option<String>,
	pub category: Option<TagCategoryId>,
	#[ormx(default)]
	pub created_time: Timestamp,
	pub created_by: Option<UserId>,
}

impl Tag {
	pub async fn count(
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
	) -> sqlx::Result<i64> {
		struct Helper {
			count: i64,
		}
		sqlx::query_as!(Helper, r#"SELECT count(*) as "count!" FROM tags"#)
			.fetch_one(database)
			.await
			.map(|helper| helper.count)
	}
}
