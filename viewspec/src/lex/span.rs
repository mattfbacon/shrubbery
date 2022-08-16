//! Provides [`Location`] and [`Span`] to track locations and regions within byte streams.

/// The location within a byte stream, where the first byte is `0`.
///
/// We have decided to limit the length of the byte stream to 4 gebibytes; thus a `u32` is sufficient.
pub type Location = u32;

/// A region within the input.
///
/// This is an inclusive range; thus the smallest span is that where `start == end`.
/// Pwease do not make `end` less than or equal to `start` or Fewwis will be angwy.
///
/// We do not use [`std::ops::Range`] type because it is known to have several serious API design issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
	/// The lower bound of the range (inclusive)
	pub start: Location,
	/// The upper bound of the range (inclusive)
	pub end: Location,
}

impl Span {
	/// Create a span that covers only a single byte at `location`.
	#[must_use]
	pub const fn single(location: Location) -> Self {
		Self {
			start: location,
			end: location,
		}
	}

	/// Create an arbitrary span to use when there is no actual span
	///
	/// This is useful for example when parsing from tokens directly.
	#[must_use]
	pub const fn null() -> Self {
		Self {
			start: Location::MAX,
			end: Location::MAX,
		}
	}

	/// If the span only covers one byte, return the [`Location`] of that byte, otherwise return `None`.
	#[must_use]
	pub const fn to_single_location(self) -> Option<Location> {
		if self.start == self.end {
			Some(self.start)
		} else {
			None
		}
	}

	/// Split a string into the sections before the span, within the span, and after the span.
	///
	/// If the span is invalid for the string (the string is too short, or either of the span boundaries is not on a UTF-8 codepoint boundary), then `None` will be returned.
	///
	/// If you pass the string that this span originally came from, you can safely `unwrap` the return value.
	#[must_use]
	#[allow(clippy::missing_panics_doc)] // panics should not occur in normal usage
	pub fn split_three(self, s: &str) -> Option<(&str, &str, &str)> {
		let start = self.start.try_into().unwrap();
		let end = self.end.try_into().unwrap();

		if !s.is_char_boundary(start) || !s.is_char_boundary(end) || s.len() < end || start > end {
			return None;
		}
		let (before, within_and_after) = s.split_at(start);
		let (within, after) = within_and_after.split_at(end - start);

		Some((before, within, after))
	}

	/// Get the length of the span, from the start to the end.
	#[must_use]
	pub const fn len(self) -> u32 {
		self.end - self.start + 1
	}

	/// `self.len() == 0`
	#[must_use]
	pub const fn is_empty(self) -> bool {
		self.len() == 0
	}
}
