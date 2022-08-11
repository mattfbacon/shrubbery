use std::fmt::{self, Display, Formatter};

use viewspec::parse::tag::{Tag, TagRef};
use viewspec::parse::{Ast, Node};

use crate::database::models;

struct Bindings<'a> {
	bindings: Vec<&'a str>,
}

impl<'a> Bindings<'a> {
	const STARTING_VALUE: usize = 1;

	pub fn new() -> Self {
		Self {
			bindings: Vec::default(),
		}
	}

	pub fn next(&mut self, value: &'a str) -> impl Display {
		struct Helper(usize);

		impl Display for Helper {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				write!(formatter, "${}", self.0)
			}
		}

		let index = self.bindings.len();
		self.bindings.push(value);
		Helper(index + Self::STARTING_VALUE)
	}

	pub fn as_values(&self) -> impl Iterator<Item = &'a str> + '_ {
		self.bindings.iter().copied()
	}
}

fn make_condition_for_tag<'a>(buf: &mut Formatter<'_>, tag: &'a Tag, bindings: &mut Bindings<'a>) {
	match tag.as_ref() {
		TagRef::Category(category) => write!(buf, "file_tags.tag IN (SELECT id FROM tags WHERE category = (SELECT id FROM tag_categories WHERE name = {}))", bindings.next(category)),
		TagRef::Name(name) => write!(buf, "file_tags.tag IN (SELECT id FROM tags WHERE name = {})", bindings.next(name)),
		TagRef::Both { category, name } => write!(buf, "file_tags.tag IN (SELECT id FROM tags WHERE name = {} AND category = (SELECT id FROM tag_categories WHERE name = {}))", bindings.next(name), bindings.next(category)),
	}.unwrap();
}

fn make_query_condition<'a>(
	buf: &mut Formatter<'_>,
	viewspec: &'a Ast,
	bindings: &mut Bindings<'a>,
) {
	use smallvec::SmallVec;

	#[derive(Clone, Copy)]
	enum StackEntry {
		AndInfix,
		OrInfix,
		CloseParen,
		Node(viewspec::parse::Key),
		Root,
	}

	let mut stack: SmallVec<[_; 50]> = [StackEntry::Root].into_iter().collect();

	while let Some(entry) = stack.pop() {
		let node = match entry {
			StackEntry::AndInfix => {
				buf.write_str(") AND (").unwrap();
				continue;
			}
			StackEntry::OrInfix => {
				buf.write_str(") OR (").unwrap();
				continue;
			}
			StackEntry::CloseParen => {
				buf.write_str(")").unwrap();
				continue;
			}
			StackEntry::Root => viewspec.root(),
			StackEntry::Node(key) => viewspec.resolve_key(key),
		};

		match node {
			Node::Tag(tag) => make_condition_for_tag(buf, &tag, bindings),
			Node::Not(inner) => {
				buf.write_str("NOT (").unwrap();
				stack.extend(
					[StackEntry::Node(*inner), StackEntry::CloseParen]
						.into_iter()
						.rev(),
				);
			}
			Node::And(a, b) => {
				buf.write_str("(").unwrap();
				stack.extend(
					[
						StackEntry::Node(*a),
						StackEntry::AndInfix,
						StackEntry::Node(*b),
						StackEntry::CloseParen,
					]
					.into_iter()
					.rev(),
				);
			}
			Node::Or(a, b) => {
				buf.write_str("(").unwrap();
				stack.extend(
					[
						StackEntry::Node(*a),
						StackEntry::OrInfix,
						StackEntry::Node(*b),
						StackEntry::CloseParen,
					]
					.into_iter()
					.rev(),
				);
			}
		}
	}
}

fn make_query(viewspec: &Ast, after: Option<models::FileId>, limit: i64) -> (String, Bindings<'_>) {
	use std::cell::Cell;

	struct ConditionHelper<'a, 'b> {
		viewspec: &'b Ast,
		bindings: Cell<Option<&'a mut Bindings<'b>>>,
	}

	impl Display for ConditionHelper<'_, '_> {
		fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
			make_query_condition(formatter, self.viewspec, self.bindings.take().unwrap());
			Ok(())
		}
	}

	let mut bindings = Bindings::new();
	let query = format!(
		"SELECT files.id, files.name FROM file_tags LEFT JOIN files ON files.id = file_tags.file WHERE {} AND file_tags.file > {} ORDER BY file_tags.file LIMIT {}",
		ConditionHelper {
			viewspec,
			bindings: Cell::new(Some(&mut bindings)),
		},
		after.unwrap_or(-1),
		limit,
	);

	(query, bindings)
}

pub async fn evaluate(
	viewspec: &Ast,
	database: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
	after: Option<models::FileId>,
	page_size: i64,
) -> sqlx::Result<Vec<(models::FileId, String)>> {
	let (query, bindings) = make_query(viewspec, after, page_size);
	let mut query = sqlx::query_as(&query);
	for binding in bindings.as_values() {
		query = query.bind(binding);
	}
	query.fetch_all(database).await
}

#[cfg(test)]
mod test {
	use std::fmt::{self, Display, Formatter};

	/// A naive implementation of `make_query_condition` that uses recursion.
	/// The behavior of this function will be compared to that of the actual implementation, and they should always have the same result.
	fn make_query_condition_naive<'short, 'formatter, 'data>(
		formatter: &'short mut Formatter<'formatter>,
		viewspec: &'data super::Ast,
		bindings: &'short mut super::Bindings<'data>,
	) {
		use viewspec::parse::Node;

		struct Helper<'short, 'formatter, 'data> {
			formatter: &'short mut Formatter<'formatter>,
			viewspec: &'data super::Ast,
			bindings: &'short mut super::Bindings<'data>,
		}

		impl<'short, 'formatter, 'data> Helper<'short, 'formatter, 'data> {
			fn go(&mut self, node: &'data Node) {
				match node {
					Node::And(l, r) => {
						self.formatter.write_str("(").unwrap();
						self.go(self.viewspec.resolve_key(*l));
						self.formatter.write_str(") AND (").unwrap();
						self.go(self.viewspec.resolve_key(*r));
						self.formatter.write_str(")").unwrap();
					}
					Node::Or(l, r) => {
						self.formatter.write_str("(").unwrap();
						self.go(self.viewspec.resolve_key(*l));
						self.formatter.write_str(") OR (").unwrap();
						self.go(self.viewspec.resolve_key(*r));
						self.formatter.write_str(")").unwrap();
					}
					Node::Not(child) => {
						self.formatter.write_str("NOT (").unwrap();
						self.go(self.viewspec.resolve_key(*child));
						self.formatter.write_str(")").unwrap();
					}
					Node::Tag(tag) => super::make_condition_for_tag(self.formatter, tag, self.bindings),
				}
			}
		}

		Helper {
			formatter,
			viewspec,
			bindings,
		}
		.go(viewspec.root())
	}

	#[test]
	fn make_query_condition() {
		let cases = [
			"a",
			"a & b",
			"a & (b | c)",
			"!a:b & (c:d | e:f)",
			"a & b: & c: & d:e",
			r#""de":"fg" & "bac":"def" & ("a\x20c":de | !f)"#,
		];

		for case in cases {
			struct Helper<'data> {
				viewspec: &'data super::Ast,
				func: for<'short, 'formatter> fn(
					&'short mut Formatter<'formatter>,
					viewspec: &'data super::Ast,
					bindings: &'short mut super::Bindings<'data>,
				),
			}

			impl Display for Helper<'_> {
				fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
					let mut bindings = super::Bindings::new();
					(self.func)(formatter, self.viewspec, &mut bindings);
					Ok(())
				}
			}

			let viewspec = viewspec::lex_and_parse(case.bytes()).unwrap();
			let actual = Helper {
				viewspec: &viewspec,
				func: super::make_query_condition,
			}
			.to_string();
			let naive = Helper {
				viewspec: &viewspec,
				func: make_query_condition_naive,
			}
			.to_string();
			assert_eq!(actual, naive);
		}
	}
}
