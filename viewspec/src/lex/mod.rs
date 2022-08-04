//! The first half of the crate: parsing bytes into spanned tokens.
//! Get started with the `lex` function.
//! Look in the `token` module for the possible tokens after lexing, or in `error` for the possible errors that can occur while lexing.

use std::iter::Peekable;

pub mod error;
mod location_tracker;
pub mod span;
#[cfg(test)]
mod test;
pub mod token;

pub use error::Error;
use error::{ComplexEscapeType, InvalidEscapeError};
use location_tracker::LocationTracker;
use span::{Location, Span};
pub use token::{SpannedToken, Token};

#[derive(Debug)]
struct Lexer<I: Iterator<Item = u8>> {
	input: Peekable<LocationTracker<I>>,
}

fn char_is_special(ch: u8) -> bool {
	b"&|!():\"".contains(&ch)
}

impl<I: Iterator<Item = u8>> Lexer<I> {
	fn new(input: I) -> Self {
		Self {
			input: LocationTracker::new(input).peekable(),
		}
	}
}

impl<I: Iterator<Item = u8>> Lexer<I> {
	fn next_skip_whitespace(&mut self) -> Option<(Location, u8)> {
		loop {
			match self.input.next() {
				Some((_location, ch)) if ch.is_ascii_whitespace() => continue,
				other => break other,
			}
		}
	}

	fn read_8bit_escape(&mut self) -> Result<char, Error> {
		let mut numbers = [0u8; 2];
		for number in &mut numbers {
			let (location, ch) = self.input.next().ok_or(Error::StringEnd)?;
			*number = char::from(ch.to_ascii_lowercase())
				.to_digit(16)
				.ok_or_else(|| {
					Error::InvalidEscape(
						Span::single(location),
						InvalidEscapeError::NonHex(ComplexEscapeType::Unicode8Bit),
					)
				})?
				.try_into()
				.unwrap();
		}
		let unicode_value = (numbers[0] << 4) | numbers[1];
		Ok(unicode_value as char)
	}

	fn read_24bit_escape(&mut self, slash_location: Location) -> Result<char, Error> {
		const HEX_DIGIT_BITS: u32 = 4;

		let (open_bracket_location, open_bracket) = self.input.next().ok_or(Error::StringEnd)?;
		if open_bracket != b'{' {
			return Err(Error::InvalidEscape(
				Span {
					start: slash_location,
					end: open_bracket_location,
				},
				InvalidEscapeError::MissingOpenBracket,
			));
		}
		let mut unicode_value = 0;
		let mut num_digits = 0;
		let close_bracket_location = loop {
			let (location, ch) = self.input.next().ok_or(Error::StringEnd)?;
			if ch == b'}' {
				break location;
			}
			if num_digits >= 6 {
				return Err(Error::InvalidEscape(
					Span::single(location),
					InvalidEscapeError::TooMany24Bit,
				));
			}
			let this_digit_value = char::from(ch.to_ascii_lowercase())
				.to_digit(16)
				.ok_or_else(|| {
					Error::InvalidEscape(
						Span::single(location),
						InvalidEscapeError::NonHex(ComplexEscapeType::Unicode24Bit),
					)
				})?;
			debug_assert!(this_digit_value < 2u32.pow(HEX_DIGIT_BITS));
			unicode_value |= this_digit_value;
			unicode_value <<= HEX_DIGIT_BITS;
			num_digits += 1;
		};

		unicode_value >>= HEX_DIGIT_BITS; // undo the last shift, which was unnecessary.

		if num_digits == 0 {
			return Err(Error::InvalidEscape(
				Span {
					start: open_bracket_location,
					end: close_bracket_location,
				},
				InvalidEscapeError::Empty24Bit,
			));
		}

		char::from_u32(unicode_value).ok_or(Error::InvalidEscape(
			Span {
				start: slash_location,
				end: close_bracket_location,
			},
			InvalidEscapeError::InvalidResult,
		))
	}

	fn read_string_escape(&mut self, slash_location: Location) -> Result<char, Error> {
		let (location, ch) = self.input.next().ok_or(Error::StringEnd)?;
		match ch {
			b'x' => self.read_8bit_escape(),
			b'u' => self.read_24bit_escape(slash_location),
			b'n' => Ok('\n'),
			b'r' => Ok('\r'),
			b't' => Ok('\t'),
			b'0' => Ok('\0'),
			b'\\' => Ok('\\'),
			b'"' => Ok('"'),
			invalid => Err(Error::InvalidEscape(
				Span {
					start: slash_location,
					end: location,
				},
				InvalidEscapeError::UnknownSimpleEscape(invalid as char),
			)),
		}
	}

	/// It is assumed that the first `"` was already read
	fn read_string(&mut self, open_quote_location: Location) -> (Span, Result<String, Error>) {
		let mut ret = Vec::new();
		let mut last_location = open_quote_location;

		while let Some((location, ch)) = self.input.next() {
			last_location = location;
			match ch {
				b'\\' => {
					let ch = match self.read_string_escape(location) {
						Ok(ch) => ch,
						Err(err) => {
							return (
								Span {
									start: open_quote_location,
									end: location,
								},
								Err(err),
							)
						}
					};
					let mut buf = [0u8; 4];
					ret.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
				}
				b'"' => {
					let span = Span {
						start: open_quote_location,
						end: location,
					};
					return (
						span,
						String::from_utf8(ret).map_err(|err| {
							let valid_up_to: Location = err.utf8_error().valid_up_to().try_into().unwrap();
							let invalid_char_start_location = open_quote_location + 1 + valid_up_to;
							Error::StringNotUtf8(invalid_char_start_location)
						}),
					);
				}
				other => ret.push(other),
			}
		}
		(
			Span {
				start: open_quote_location,
				end: last_location,
			},
			Err(Error::StringEnd),
		)
	}

	fn read_bare_string(
		&mut self,
		first_byte_location: Location,
		first_byte: u8,
	) -> (Span, Result<String, Error>) {
		let mut ret = Vec::from([first_byte]);
		let mut last_location = first_byte_location;

		while let Some((location, ch)) = self.input.next_if(|&(_location, ch)| !char_is_special(ch)) {
			last_location = location;
			ret.push(ch);
		}

		let mut ret = match String::from_utf8(ret) {
			Ok(ret) => ret,
			Err(err) => {
				let valid_up_to: Location = err.utf8_error().valid_up_to().try_into().unwrap();
				let invalid_char_start_location = first_byte_location + valid_up_to;
				return (
					Span {
						start: first_byte_location,
						end: last_location,
					},
					Err(Error::StringNotUtf8(invalid_char_start_location)),
				);
			}
		};

		let spaces_trimmed_len = ret.trim_end().len();
		ret.truncate(spaces_trimmed_len);

		(
			Span {
				start: first_byte_location,
				end: first_byte_location + Location::try_from(spaces_trimmed_len).unwrap() - 1,
			},
			Ok(ret),
		)
	}
}

impl<I: Iterator<Item = u8>> Iterator for Lexer<I> {
	type Item = SpannedToken;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_skip_whitespace().map(|(location, ch)| {
			match ch {
				b'&' => Token::And,
				b'|' => Token::Or,
				b'!' => Token::Not,
				b'(' => Token::OpenParen,
				b')' => Token::CloseParen,
				b':' => Token::Colon,
				b'"' => {
					let (span, result) = self.read_string(location);
					return match result {
						Ok(content) => Token::String {
							content: content.into(),
							bare: false,
						},
						Err(error) => Token::Error(Box::new(error)),
					}
					.with_span(span);
				}
				other => {
					let (span, result) = self.read_bare_string(location, other);
					return match result {
						Ok(content) => Token::String {
							content: content.into(),
							bare: true,
						},
						Err(error) => Token::Error(Box::new(error)),
					}
					.with_span(span);
				}
			}
			.with_span(Span::single(location))
		})
	}
}

/// Parse a sequence of bytes into a sequence of spanned tokens
pub fn lex(input: impl IntoIterator<Item = u8>) -> impl Iterator<Item = SpannedToken> {
	Lexer::new(input.into_iter())
}
