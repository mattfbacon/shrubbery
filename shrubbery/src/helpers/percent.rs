use std::fmt::{self, Display, Formatter};

use percent_encoding::{percent_decode, percent_encode};
use serde::de::{Deserializer, Error as DeError};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

#[inline]
pub fn encode(data: &[u8]) -> String {
	percent_encode(data, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// Encodes the string when serialized, and decodes it when deserialized. The internal representation is the **decoded** string.
#[derive(Debug)]
#[repr(transparent)]
pub struct EncodedString(String);

impl EncodedString {
	pub fn to_encoded(&self) -> String {
		encode(self.0.as_bytes())
	}
	pub fn into_decoded(self) -> String {
		self.0
	}
}

impl Display for EncodedString {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		self.to_encoded().fmt(formatter)
	}
}

impl Serialize for EncodedString {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.to_encoded().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for EncodedString {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
	where
		D::Error: DeError,
	{
		percent_decode(String::deserialize(deserializer)?.as_bytes())
			.decode_utf8()
			.map_err(DeError::custom)
			.map(|cow| Self(cow.into_owned()))
	}
}
