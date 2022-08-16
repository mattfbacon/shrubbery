//! Provides [`Error`] as well as some support types for more specific error reasons.

use super::span::{Location, Span};

/// Types of complex escapes.
///
/// Complex escapes are those that are not simple like `\n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexEscapeType {
	/// A 24-bit escape such as `\u{bade5c}`.
	///
	/// Up to 6 hex digits provide up to 24 bits of data.
	Unicode24Bit,
	/// An 8-bit escape such as `\xff`.
	///
	/// Up to 2 hex digits provide up to 8 bits of data.
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

/// Reasons why a string escape is invalid.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)] // ending with `Error` is clearer
#[allow(variant_size_differences)] // no better way
pub enum InvalidEscapeError {
	/// The character used for a simple escape, such as the `n` in `\n`, was unrecognized.
	#[error("unknown simple escape {0:?}")]
	UnknownSimpleEscape(char),
	/// An invalid hex digit was found.
	///
	/// 24- and 8-bit escapes expect only hex digits.
	/// The field specifies which type of escape it was.
	#[error("invalid hex digit in {0} escape")]
	NonHex(ComplexEscapeType),
	/// Too many digits were found inside the brackets of a 24-bit escape.
	///
	/// Unlike 8-bit escapes, 24-bit escapes are delimited.
	/// The maximum number of digits is 6.
	#[error("too many digits in 24-bit escape; the maximum is 6")]
	TooMany24Bit,
	/// The first two characters of a 24-bit escape, `\u`, were found, but the following character was not an opening bracket.
	#[error("expected open bracket for 24-bit escape")]
	MissingOpenBracket,
	/// The brackets of a 24-bit escape were left empty, like `\u{}`.
	#[error("empty 24-bit escape")]
	Empty24Bit,
	/// The numerical value expressed by the 24-bit escape would result in an invalid character.
	///
	/// The escape itself was valid but the numerical value was not a valid Unicode scalar value.
	#[error("24-bit escape results in an invalid character")]
	InvalidResult,
}

/// Reasons why lexing can fail.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
#[allow(variant_size_differences)] // no better way
pub enum Error {
	/// This occurs for various reasons, such as missing an end quote or not finishing an escape.
	#[error("unexpected end of input while reading string")]
	StringEnd,
	/// Within the input stream, invalid UTF-8 was found.
	///
	/// Strings in Rust are required to be UTF-8 but we take `Iterator<Item = u8>` as our input, so it is possible for invalid UTF-8 to occur.
	///
	/// The location of the invalid escape is included for error reporting.
	#[error("string is not valid UTF-8")]
	StringNotUtf8(Location),
	/// An invalid string escape was found.
	///
	/// See the embedded [`InvalidEscapeError`] for more information.
	///
	/// The span of the invalid escape is included for error reporting.
	#[error("invalid string escape: {1}")]
	InvalidEscape(Span, #[source] InvalidEscapeError),
}
