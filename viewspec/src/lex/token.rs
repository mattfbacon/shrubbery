//! Provides the `Token` type as well as some support types.

use crate::lex::error::Error;
use crate::lex::span::Span;

/// The possible tokens returned from lexing.
#[derive(Debug, PartialEq, Eq)]
pub enum Token {
	/// `&`
	And,
	/// `)`
	CloseParen,
	/// `:`
	Colon,
	/// `!`
	Not,
	/// `(`
	OpenParen,
	/// `|`
	Or,
	/// `"abc"` or `abc`
	String {
		/// The textual content of the string, with all escapes evaluated and whitespace trimmed
		content: Box<str>,
		/// Whether the string was expressed without quotes, like `abc` but not like `"abc"`.
		/// Bare strings are more limited in which characters they can contain
		bare: bool,
	},
	/// An error occurred while lexing.
	///Box
	/// This is in the `Token` enum to make errors more pervasive but simultaneously easier to handle. It is boxed to avoid incurring overhead in the non-error cases
	Error(Box<Error>),
}

impl Token {
	/// Add a span to a token to create a [`SpannedToken`].
	#[must_use]
	pub fn with_span(self, span: Span) -> SpannedToken {
		SpannedToken { span, token: self }
	}

	/// Get the type of the token<'_> as a [`Type`].
	#[must_use]
	pub fn into_type(self) -> Type {
		match self {
			Self::And => Type::And,
			Self::CloseParen => Type::CloseParen,
			Self::Colon => Type::Colon,
			Self::Error(error) => Type::Error(error),
			Self::Not => Type::Not,
			Self::OpenParen => Type::OpenParen,
			Self::Or => Type::Or,
			Self::String { .. } => Type::String,
		}
	}
}

/// A [`Token`] with a [`Span`] that stores where the token occurred in the input.
#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq, Eq))]
#[allow(clippy::module_name_repetitions)]
pub struct SpannedToken {
	/// The span of the token
	pub span: Span,
	/// The token itself
	pub token: Token,
}

/// A parallel enum to [`Token`] but without fields (except the error variant).
///
/// [`Error`] retains its error since it is generally needed even when just handling the token type.
///
/// See [`Token`] for variant documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)] // all the variants exactly match those of `Token`, which are documented
pub enum Type {
	And,
	CloseParen,
	Colon,
	Error(Box<Error>),
	Not,
	OpenParen,
	Or,
	String,
}
