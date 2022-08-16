//! Provides [`Ast`] and [`Node`].
//!
//! Together they implement a *non-recursive* AST.

use std::fmt::{self, Debug, Formatter};

pub use super::tag::Tag;

/// The key used to refer to other nodes in the AST.
///
/// To resolve a key to a node, use `Ast::resolve_key`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Key(u32);

/// A node in the AST.
///
/// Note: Nodes do not own their children; they only contain references to them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
	/// A tag, which is only leaf node in this AST
	Tag(Tag),
	/// An "and" operation, such as `a & b`
	And(Key, Key),
	/// An "or" operation, such as `a | b`
	Or(Key, Key),
	/// A "not" operation, such as `!a`
	Not(Key),
}

pub(super) struct Storage(Vec<Node>);

impl Storage {
	pub(super) fn new() -> Self {
		Self(Vec::new())
	}

	pub(super) fn insert(&mut self, item: Node) -> Key {
		let key: u32 = self
			.0
			.len()
			.try_into()
			.expect("too many items in AST storage");
		self.0.push(item);
		Key(key)
	}

	pub(super) fn get(&self, key: Key) -> &Node {
		self
			.0
			.get(usize::try_from(key.0).expect("u32 is bigger than usize, ghetto platform alert"))
			.expect("invalid key (are you mixing keys between ASTs?)")
	}
}

/// The full AST, which contains a root node as well as the storage used to resolve `Key`s
pub struct Ast {
	pub(super) storage: Storage,
	pub(super) root: Node,
}

impl Ast {
	/// Get a reference to the root node.
	#[must_use]
	pub fn root(&self) -> &Node {
		&self.root
	}

	/// Resolve a reference to a node.
	#[must_use]
	pub fn resolve_key(&self, key: Key) -> &Node {
		self.storage.get(key)
	}
}

impl Debug for Ast {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		const DEBUG_PRETTY_TABSTOP: usize = 4;

		enum StackEntry {
			ClosingParen,
			Comma,
			Node(Key),
			RootNode,
		}

		let pretty = formatter.alternate();
		let mut indentation_level = 0u32;
		let mut stack = [StackEntry::RootNode]
			.into_iter()
			.collect::<smallvec::SmallVec<[_; 50]>>();

		macro_rules! indent {
			() => {
				// use dynamic width to create indentation
				write!(
					formatter,
					"{:width$}",
					"",
					width = indentation_level as usize * DEBUG_PRETTY_TABSTOP
				)?;
			};
		}
		macro_rules! write_prefix {
			($name:expr) => {
				if pretty {
					indent!();
					writeln!(formatter, "{}(", $name)?;
					indentation_level += 1;
				} else {
					write!(formatter, "{}(", $name)?;
				}
			};
		}

		while let Some(entry) = stack.pop() {
			let node: &Node = match entry {
				StackEntry::ClosingParen => {
					if pretty {
						writeln!(formatter, ",")?; // trailing comma
						indentation_level -= 1;
						indent!();
					}
					write!(formatter, ")")?;
					continue;
				}
				StackEntry::Comma => {
					if pretty {
						writeln!(formatter, ",")?;
					} else {
						write!(formatter, ", ")?;
					}
					continue;
				}
				StackEntry::Node(key) => self.storage.get(key),
				StackEntry::RootNode => &self.root,
			};

			match node {
				Node::And(left, right) => {
					// print this now, rather than having another stack entry, since we can
					write_prefix!("And");
					// we push the child entries in reverse since items are popped from a Vec in the opposite order of insertion
					stack.extend(
						[
							StackEntry::Node(*left),
							StackEntry::Comma,
							StackEntry::Node(*right),
							StackEntry::ClosingParen,
						]
						.into_iter()
						.rev(),
					);
				}
				Node::Or(left, right) => {
					write_prefix!("Or");
					stack.extend(
						[
							StackEntry::Node(*left),
							StackEntry::Comma,
							StackEntry::Node(*right),
							StackEntry::ClosingParen,
						]
						.into_iter()
						.rev(),
					);
				}
				Node::Not(child) => {
					write_prefix!("Not");
					stack.extend(
						[StackEntry::Node(*child), StackEntry::ClosingParen]
							.into_iter()
							.rev(),
					);
				}
				Node::Tag(tag) => {
					indent!();
					write!(formatter, "{tag:?}",)?; // don't print in pretty mode, to avoid complicating indentation
				}
			}
		}

		Ok(())
	}
}
