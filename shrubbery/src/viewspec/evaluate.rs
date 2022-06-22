use smartstring::alias::String as SmartString;

use super::{Tag, ViewSpec};
use crate::database::models;

struct Bindings<'a> {
	bindings: Vec<&'a SmartString>,
	starting_value: usize,
}

impl<'a> Bindings<'a> {
	pub fn new(starting_value: usize) -> Self {
		Self {
			bindings: Default::default(),
			starting_value,
		}
	}

	pub fn next(&mut self, value: &'a SmartString) -> impl std::fmt::Display {
		struct Helper(usize);

		impl std::fmt::Display for Helper {
			fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
				write!(formatter, "${}", self.0)
			}
		}

		let index = self.bindings.len();
		self.bindings.push(value);
		Helper(index + self.starting_value)
	}

	pub fn as_values(&self) -> &[&'a SmartString] {
		&self.bindings
	}
}

impl Tag {
	fn make_condition<'a>(&'a self, bindings: &mut Bindings<'a>) -> String {
		match self {
			Self::Category(category) => format!("file_tags.tag IN (SELECT id FROM tags WHERE category = (SELECT id FROM tag_categories WHERE name = {}))", bindings.next(category)),
			Self::Tag(tag) => format!("file_tags.tag IN (SELECT id FROM tags WHERE name = {})", bindings.next(tag)),
			Self::Both { category, tag } => format!("file_tags.tag IN (SELECT id FROM tags WHERE name = {} AND category = (SELECT id FROM tag_categories WHERE name = {}))", bindings.next(tag), bindings.next(category)),
		}
	}
}

impl ViewSpec {
	fn make_query_helper<'a>(&'a self, bindings: &mut Bindings<'a>) -> String {
		match self {
			Self::Tag(tag) => tag.make_condition(bindings),
			Self::Not(inner) => format!("NOT ({})", inner.make_query_helper(bindings)),
			Self::And(a, b) => format!(
				"({}) AND ({})",
				a.make_query_helper(bindings),
				b.make_query_helper(bindings)
			),
			Self::Or(a, b) => format!(
				"({}) OR ({})",
				a.make_query_helper(bindings),
				b.make_query_helper(bindings)
			),
		}
	}

	fn make_query(&self, after: Option<models::FileId>, limit: i64) -> (String, Bindings<'_>) {
		let mut bindings = Bindings::new(1);
		let query = self.make_query_helper(&mut bindings);
		let query = format!("SELECT files.id, files.name FROM file_tags LEFT JOIN files ON files.id = file_tags.file WHERE {} AND file_tags.file > {} ORDER BY file_tags.file LIMIT {}", query, after.unwrap_or(-1), limit);
		(query, bindings)
	}

	pub async fn evaluate(
		&self,
		database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
		after: Option<models::FileId>,
		page_size: i64,
	) -> sqlx::Result<Vec<(models::FileId, String)>> {
		let (query, bindings) = self.make_query(after, page_size);
		let mut query = sqlx::query_as(&query);
		for binding in bindings.as_values() {
			query = query.bind(binding.as_str());
		}
		query.fetch_all(database).await
	}
}
