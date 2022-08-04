//! This module centers around `Error`, but also contains a utility type for a stack of contexts describing the stack of parsing rules that were passed through to reach the point where the error occurred

use crate::lex::span::{Location, Span};
use crate::lex::token::Type as TokenType;

/// Reasons why parsing can fail
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[allow(variant_size_differences)] // no better way
pub enum Error {
	/// While attempting to store the category in a `Tag`, it was too long for the length to be represented
	#[error("category too long; the maximum length is 65535 bytes")]
	CategoryTooLong(Span),
	/// While parsing a tag, `0` was found, or an EOF was found if `0` is `None`.
	#[error("expected a tag but got {0:?}")]
	ExpectedTagGot(Option<(Span, TokenType)>),
	/// Expected a closing parenthesis for the opening parenthesis found at `location`
	#[error("unclosed parenthesis")]
	UnclosedParenthesis {
		/// The location of the unmatched opening parenthesis
		open_location: Location,
	},
}
