use std::borrow::Cow;
use std::str::FromStr;

use serde::de::{Deserializer, Error as _};
use serde::Deserialize;

/// The purpose of `OrNull` is to allow deserializing something like a number from an HTML form, where null is a possibility.
/// Unlike in some other contexts where omitting the field entirely could be used to signify the lack of value, HTML forms seem to always include every field, and you only have the choice of what value to put for that field.
/// Within that restriction, the two best options I see for that value are an empty string and the string "null".
/// However, HTML itself seems to prefer the latter, since when you leave off the `value` attribute on an `option` in a `select`, it will write the string "null" if that field is selected.
///
/// Note that `OrNull` uses `FromStr` to get the value of the non-null variant, rather than `Deserialize`. The reason for this is that HTML forms, at least when using Axum's `Form` extractor, are stringly typed.
/// Thus, attempting to deserialize something other than a string (or something that delegates to deserializing a string, of course) seems to always fail.
/// Within this context, `FromStr` seems like the best trait to use, since it is specifically meant for parsing data from strings.
/// Make sure your `T`'s `FromStr` implementation doesn't accept null; otherwise, which is chosen, null or your type, upon receiving `null`, is undefined.
#[derive(Debug)]
pub struct OrNull<T>(Option<T>);

impl<'de, T: FromStr> Deserialize<'de> for OrNull<T>
where
	T::Err: std::fmt::Display,
{
	fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
		match &*<Cow<'_, str>>::deserialize(de)? {
			"null" => Ok(Self(None)),
			other => T::from_str(&other)
				.map(Some)
				.map(Self)
				.map_err(D::Error::custom),
		}
	}
}

impl<T> From<OrNull<T>> for Option<T> {
	fn from(OrNull(opt): OrNull<T>) -> Self {
		opt
	}
}

impl<T> OrNull<T> {
	pub fn into_option(self) -> Option<T> {
		Option::from(self)
	}
}
