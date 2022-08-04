//! This module centers around `Error`, but also contains support types for more specific error reasons.

use super::span::{Location, Span};

/// Types of complex escapes, that is, those that are not simple, like `\n`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexEscapeType {
	/// A 24-bit escape, such as `\u{bade5c}` (6 hex digits provides 24 bits of data)
	Unicode24Bit,
	/// An 8-bit escape, such as `\xff` (2 hex digits provides 8 bits of data)
	Unicode8Bit,
}

impl std::fmt::Display for ComplexEscapeType {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		formatter.write_str(match self {
			Self::Unicode24Bit => "24-bit",
			Self::Unicode8Bit => "8-bit",
		})
	}
}

/// Reasons why a string escape is invalid
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)] // ending with `Error` is clearer
#[allow(variant_size_differences)] // no better way
pub enum InvalidEscapeError {
	/// The character used for a simple escape, such as the `n` in `\n`, was unrecognized
	#[error("unknown simple escape {0:?}")]
	UnknownSimpleEscape(char),
	/// 24- and 8-bit escapes expect only hex digits, and an invalid digit was found
	#[error("invalid hex digit in {0} escape")]
	NonHex(ComplexEscapeType),
	/// Unlike 8-bit escapes, 24-bit escapes are delimited, and too many digits were found inside the brackets. The maximum number of digits is 6, which provides the full 24 bits of data
	#[error("too many digits in 24-bit escape; the maximum is 6")]
	TooMany24Bit,
	/// The first two characters of a 24-bit escape, `\u`, were found, but the following character was not an opening bracket
	#[error("expected open bracket for 24-bit escape")]
	MissingOpenBracket,
	/// The brackets of a 24-bit escape were left empty, like `\u{}`
	#[error("empty 24-bit escape")]
	Empty24Bit,
	/// The value expressed by the 24-bit escape resulted in an invalid character
	#[error("24-bit escape results in an invalid character")]
	InvalidResult,
}

/// Reasons why lexing can fail
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
#[allow(variant_size_differences)] // no better way
pub enum Error {
	/// This occurs for various reasons, such as missing an end quote or not finishing an escape
	#[error("unexpected end of input while reading string")]
	StringEnd,
	/// Strings in Rust are required to be UTF-8 but we take `Iterator<Item = u8>` as our input. Within that byte stream, invalid UTF-8 was found
	#[error("string is not valid UTF-8")]
	StringNotUtf8(Location),
	/// An invalid string escape was found. See the embedded `InvalidEscapeError` for more information
	#[error("invalid string escape: {1}")]
	InvalidEscape(Span, #[source] InvalidEscapeError),
}
