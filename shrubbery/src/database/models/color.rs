use std::borrow::Cow;
use std::str::FromStr;

use sqlx::encode::IsNull;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, Postgres, Type};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
	pub red: u8,
	pub green: u8,
	pub blue: u8,
}

impl Color {
	pub fn to_hex(self) -> String {
		format!("#{:02x}{:02x}{:02x}", self.red, self.green, self.blue)
	}

	pub fn from_hex(hex: &str) -> Result<Self, FromHexError> {
		if hex.len() != 7 || hex.as_bytes()[0] != b'#' {
			return Err(FromHexError::Format);
		}
		Ok(Self {
			red: u8::from_str_radix(&hex[1..3], 16)?,
			green: u8::from_str_radix(&hex[3..5], 16)?,
			blue: u8::from_str_radix(&hex[5..7], 16)?,
		})
	}
}

#[derive(Debug, Error)]
pub enum FromHexError {
	#[error("string should be in format `#abcdef`")]
	Format,
	#[error("bad hex integer: {0}")]
	Integer(#[from] std::num::ParseIntError),
}

impl Type<Postgres> for Color {
	fn type_info() -> PgTypeInfo {
		<String as Type<Postgres>>::type_info()
	}

	fn compatible(ty: &PgTypeInfo) -> bool {
		<String as Type<Postgres>>::compatible(ty)
	}
}

impl Encode<'_, Postgres> for Color {
	fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
		<_ as Encode<'_, Postgres>>::encode_by_ref(&self.to_hex(), buf)
	}

	fn produces(&self) -> Option<PgTypeInfo> {
		Some(Self::type_info())
	}

	fn size_hint(&self) -> usize {
		"\"#abcdef\"".len()
	}
}

impl<'r> Decode<'r, Postgres> for Color {
	fn decode(value: PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
		Ok(Self::from_hex(&<Cow<'_, str>>::decode(value)?)?)
	}
}

impl FromStr for Color {
	type Err = FromHexError;
	fn from_str(hex: &str) -> Result<Self, Self::Err> {
		Self::from_hex(hex)
	}
}

impl serde::Serialize for Color {
	fn serialize<S: serde::ser::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
		serde::Serialize::serialize(&self.to_hex(), ser)
	}
}

impl<'de> serde::Deserialize<'de> for Color {
	fn deserialize<D: serde::de::Deserializer<'de>>(de: D) -> Result<Self, D::Error>
	where
		D::Error: serde::de::Error,
	{
		<Cow<'de, str>>::deserialize(de)
			.and_then(|raw| Self::from_hex(&raw).map_err(serde::de::Error::custom))
	}
}

#[cfg(test)]
mod test {
	use super::Color;

	#[test]
	fn to_hex() {
		assert_eq!(
			Color {
				red: 0,
				green: 0,
				blue: 0
			}
			.to_hex(),
			"#000000"
		);

		assert_eq!(
			Color {
				red: 0x01,
				green: 0x34,
				blue: 0x56,
			}
			.to_hex(),
			"#013456"
		);
	}
}
