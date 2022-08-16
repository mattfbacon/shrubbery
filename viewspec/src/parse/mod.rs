//! Parsing tokens into an AST.
//!
//! Get started with the [`parse`] function.
//!
//! Look in the [`ast`] module for the result of parsing, or in the [`error`] module for the possible errors that can occur while parsing.
//!
//! # Goals
//!
//! This parser has a goal to avoid recursion, both at the type-level ([`Node`]s do not contain `Box<Node>`) and in functions (parser, Debug implementation)
//!
//! # Parsing Rules
//!
//! Screaming snake case represents tokens from the lexing stage. The entry point is `expression0`.
//!
//! ```text
//! expression0 = expression1 (binary_op expression1)*
//! expression1 = unary_op* expression2
//! expression2 = tag | OPEN_PAREN expression0 CLOSE_PAREN
//! tag = STRING | STRING COLON | STRING COLON STRING
//! binary_op = AND | OR
//! unary_op = NOT
//! ```

use crate::lex::span::Location;
use crate::lex::token::{SpannedToken, Token};

pub mod ast;
pub mod error;
pub mod tag;
#[cfg(test)]
mod test;

use ast::Storage;
pub use ast::{Ast, Key, Node};
pub use error::Error;

/// A convenience `Result` alias with `E` defaulting to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Parse a sequence of [`SpannedToken`] into an [`Ast`] or an [`Error`].
///
/// Takes [`SpannedToken`] rather than [`Token`] so error messages can include spans.
#[allow(clippy::missing_errors_doc)] // obvious
#[allow(clippy::missing_panics_doc)] // those panics should not occur
#[allow(clippy::too_many_lines)] // mostly boilerplate
pub fn parse(input: impl Iterator<Item = SpannedToken>) -> Result<Ast> {
	#[derive(Debug, Clone, Copy)]
	enum Operator {
		And,
		Or,
	}

	#[derive(Debug, Clone, Copy)]
	enum StackEntry {
		// Expression0 = [Expression1, Expression0After]
		Expression1,
		Expression2,
		Expression0After,
		Expression0Combiner { left_key: Key, operator: Operator },
		Expression1After { not: bool },
		Expression2AfterParen { open_location: Location },
	}

	let mut input = input.peekable();
	let mut storage = Storage::new();
	let mut stack = [StackEntry::Expression1, StackEntry::Expression0After]
		.into_iter()
		.rev()
		.collect::<smallvec::SmallVec<[_; 50]>>();
	let mut root: Option<Node> = None;

	while let Some(rule) = stack.pop() {
		match rule {
			StackEntry::Expression1 => {
				let mut not = false;
				while input.next_if(|token| token.token == Token::Not).is_some() {
					not = !not;
				}

				stack.extend(
					[
						StackEntry::Expression2,
						StackEntry::Expression1After { not },
					]
					.into_iter()
					.rev(),
				);
			}
			StackEntry::Expression2 => {
				// pseudo:
				// otherwise, set root to tag and push nothing
				if let Some(open_paren) = input.next_if(|token| token.token == Token::OpenParen) {
					stack.extend(
						[
							StackEntry::Expression1,
							StackEntry::Expression0After,
							StackEntry::Expression2AfterParen {
								open_location: open_paren.span.start,
							},
						]
						.into_iter()
						.rev(),
					);
				} else {
					root = Some(tag(&mut input)?);
				}
			}
			StackEntry::Expression0After => {
				if let Some(SpannedToken { token, .. }) =
					input.next_if(|token| matches!(token.token, Token::And | Token::Or))
				{
					let operator = match token {
						Token::And => Operator::And,
						Token::Or => Operator::Or,
						_ => unreachable!(),
					};

					let left_node = root.take().unwrap();

					stack.extend(
						[
							StackEntry::Expression1,
							StackEntry::Expression0Combiner {
								left_key: storage.insert(left_node),
								operator,
							},
							StackEntry::Expression0After,
						]
						.into_iter()
						.rev(),
					);
				}
			}
			StackEntry::Expression0Combiner { left_key, operator } => {
				let right_key = storage.insert(root.take().unwrap());
				root = Some(match operator {
					Operator::And => Node::And,
					Operator::Or => Node::Or,
				}(left_key, right_key));
			}
			StackEntry::Expression1After { not } => {
				if not {
					let child = root.take().unwrap();
					let key = storage.insert(child);
					root = Some(Node::Not(key));
				}
			}
			StackEntry::Expression2AfterParen { open_location } => {
				if input
					.next()
					.map_or(true, |token| token.token != Token::CloseParen)
				{
					return Err(Error::UnclosedParenthesis { open_location });
				}
			}
		}
	}

	Ok(Ast {
		storage,
		root: root.unwrap(),
	})
}

fn tag(input: &mut std::iter::Peekable<impl Iterator<Item = SpannedToken>>) -> Result<Node> {
	let first = input.next();
	let first = match first {
		Some(SpannedToken {
			token: Token::String { content, .. },
			..
		}) => content,
		other => {
			return Err(Error::ExpectedTagGot(
				other.map(|token| (token.span, token.token.into_type())),
			));
		}
	};
	let tag = if input.next_if(|token| token.token == Token::Colon).is_some() {
		match input.next_if(|token| {
			matches!(
				token,
				SpannedToken {
					token: Token::String { .. },
					..
				}
			)
		}) {
			Some(
				ref spanned @ SpannedToken {
					token: Token::String {
						content: ref second,
						..
					},
					..
				},
			) => match ast::Tag::both(&first, second) {
				Some(tag) => tag,
				None => return Err(Error::CategoryTooLong(spanned.span)),
			},
			_ => ast::Tag::category(&first),
		}
	} else {
		ast::Tag::name(&first)
	};
	Ok(Node::Tag(tag))
}

/* OLD RECURSIVE IMPLEMENTATION

/// Parse a sequence of `SpannedToken` into an `Ast`.
///
/// This function takes `SpannedToken` rather than `Token` so error messages can include spans.
pub fn parse(input: impl Iterator<Item = SpannedToken>) -> Result<Ast, Error> {
	let mut input = input.peekable();
	let mut storage = Storage::with_key();
	let root = expression0(&mut input, &mut storage)?;
	Ok(Ast { storage, root })
}

fn expression0(
	input: &mut Peekable<impl Iterator<Item = SpannedToken>>,
	storage: &mut Storage,
) -> Result<Node, Error> {
	let mut ret = expression1(input, storage).map_err(|err| err.context("expression0"))?;
	loop {
		match input.next_if(|token| matches!(token.token, Token::And | Token::Or)) {
			Some(SpannedToken {
				token: operator, ..
			}) => {
				let right_operand =
					expression1(input, storage).map_err(|err| err.context("expression0"))?;
				let left_operand_key = storage.insert(ret);
				let right_operand_key = storage.insert(right_operand);
				ret = match operator {
					Token::And => Node::And,
					Token::Or => Node::Or,
					_ => unreachable!(),
				}(left_operand_key, right_operand_key);
			}
			None => break Ok(ret),
		}
	}
}

fn expression1(
	input: &mut Peekable<impl Iterator<Item = SpannedToken>>,
	storage: &mut Storage,
) -> Result<Node, Error> {
	let mut should_not = false;

	while input.next_if(|token| token.token == Token::Not).is_some() {
		should_not = !should_not;
	}

	let child = expression2(input, storage).map_err(|err| err.context("expression1"))?;
	if should_not {
		let key = storage.insert(child);
		Ok(Node::Not(key))
	} else {
		Ok(child)
	}
}

fn expression2(
	input: &mut Peekable<impl Iterator<Item = SpannedToken>>,
	storage: &mut Storage,
) -> Result<Node, Error> {
	static EXPECTED: &[TokenType] = &[TokenType::OpenParen, TokenType::String];
	let err = |got| {
		Error::Expected {
			expected: &EXPECTED,
			got,
			entity: &"expression layer 2",
		}
		.context("expression2")
	};
	let maybe_open_paren = input.peek().ok_or_else(|| err(None))?;
	match &maybe_open_paren.token {
		Token::OpenParen => {
			let open_paren = input.next().unwrap();
			let ret = match expression0(input, storage).map_err(|err| err.context("expression2")) {
				Err(Error {
					contexts,
					error: Error::Expected { got: None, .. },
				}) => Err(Error {
					contexts,
					error: Error::UnclosedParenthesis {
						open_location: open_paren.start_location,
					},
				}),
				other => other,
			}?;
			if input
				.next()
				.map_or(true, |token| token.token != Token::CloseParen)
			{
				return Err(
					Error::UnclosedParenthesis {
						open_location: open_paren.start_location,
					}
					.context("expression2"),
				);
			}
			Ok(ret)
		}
		Token::String { .. } => tag(input, storage),
		other => Err(err(Some(other.type_()))),
	}
}

*/
