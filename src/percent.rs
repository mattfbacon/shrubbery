use std::fmt::{self, Display, Formatter};

use percent_encoding::percent_decode;
use serde::{de::Deserializer, de::Error as DeError, ser::Serializer, Deserialize, Serialize};

#[inline]
pub fn percent_encode(data: &[u8]) -> String {
	percent_encoding::percent_encode(data, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// Encodes the string when serialized, and decodes it when deserialized. The internal representation is the **decoded** string.
#[derive(Debug)]
#[repr(transparent)]
pub struct PercentEncodedString(pub String);

impl PercentEncodedString {
	pub fn encode(&self) -> String {
		percent_encode(self.0.as_bytes())
	}
}

impl Display for PercentEncodedString {
	fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
		self.encode().fmt(formatter)
	}
}

impl Serialize for PercentEncodedString {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		self.encode().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for PercentEncodedString {
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
