use std::fmt::{self, Display, Formatter};
use std::ops::Add;

use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

/// Enforces UTC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "OffsetDateTime")]
pub struct Timestamp(PrimitiveDateTime);

impl Timestamp {
	pub fn now() -> Self {
		let now_offset = OffsetDateTime::now_utc();
		now_offset.into()
	}

	pub fn is_in_past(self) -> bool {
		self < Self::now()
	}

	pub fn date(self) -> Date {
		self.0.date()
	}

	pub fn time(self) -> Time {
		Time(self.0.time())
	}

	pub fn html_date(self) -> impl Display {
		struct Helper(Date);

		impl Display for Helper {
			fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
				self
					.0
					.format_into(&mut FmtToIo(f), html_date::FORMAT)
					.unwrap();
				Ok(())
			}
		}

		Helper(self.date())
	}

	pub fn html_time(self) -> impl Display {
		self.time().display_html()
	}
}

impl Display for Timestamp {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		self
			.0
			.format_into(
				&mut FmtToIo(f),
				time::macros::format_description!(
					"[day padding:none] [month repr:short] [year] [hour]:[minute]:[second]"
				),
			)
			.unwrap();
		Ok(())
	}
}

impl From<Timestamp> for OffsetDateTime {
	fn from(timestamp: Timestamp) -> Self {
		timestamp.0.assume_utc()
	}
}

impl From<OffsetDateTime> for Timestamp {
	fn from(offset_dt: OffsetDateTime) -> Self {
		let in_utc = offset_dt.to_offset(UtcOffset::UTC);
		Self(PrimitiveDateTime::new(in_utc.date(), in_utc.time()))
	}
}

impl From<std::time::SystemTime> for Timestamp {
	fn from(system_time: std::time::SystemTime) -> Self {
		OffsetDateTime::from(system_time).into()
	}
}

impl From<Timestamp> for std::time::SystemTime {
	fn from(timestamp: Timestamp) -> Self {
		timestamp.0.assume_utc().into()
	}
}

impl<T> Add<T> for Timestamp
where
	PrimitiveDateTime: Add<T, Output = PrimitiveDateTime>,
{
	type Output = Self;

	fn add(self, offset: T) -> Self {
		Self(self.0 + offset)
	}
}

macro_rules! impl_sqlx_type_via_conversion {
	($ty:ty, $convert_to:ty) => {
		impl sqlx::Type<sqlx::Postgres> for $ty {
			fn type_info() -> sqlx::postgres::PgTypeInfo {
				<$convert_to as sqlx::Type<sqlx::Postgres>>::type_info()
			}

			fn compatible(ty: &sqlx::postgres::PgTypeInfo) -> bool {
				<$convert_to as sqlx::Type<sqlx::Postgres>>::compatible(ty)
			}
		}

		impl sqlx::Encode<'_, sqlx::Postgres> for $ty {
			fn encode_by_ref(&self, args: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
				<$convert_to as sqlx::Encode<'_, sqlx::Postgres>>::encode(
					<$convert_to as From<Self>>::from(<Self as Clone>::clone(self)),
					args,
				)
			}

			fn encode(self, args: &mut sqlx::postgres::PgArgumentBuffer) -> sqlx::encode::IsNull {
				<$convert_to as sqlx::Encode<'_, sqlx::Postgres>>::encode(
					<$convert_to as From<Self>>::from(self),
					args,
				)
			}

			fn produces(&self) -> Option<sqlx::postgres::PgTypeInfo> {
				<$convert_to as sqlx::Encode<'_, sqlx::Postgres>>::produces(
					&<$convert_to as From<Self>>::from(<Self as Clone>::clone(self)),
				)
			}

			fn size_hint(&self) -> usize {
				<$convert_to as sqlx::Encode<'_, sqlx::Postgres>>::size_hint(
					&<$convert_to as From<Self>>::from(<Self as Clone>::clone(self)),
				)
			}
		}

		impl<'r> sqlx::Decode<'r, sqlx::Postgres> for $ty {
			fn decode(value: sqlx::postgres::PgValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
				<$convert_to as sqlx::Decode<'r, sqlx::Postgres>>::decode(value)
					.map(<Self as From<$convert_to>>::from)
			}
		}
	};
}

impl_sqlx_type_via_conversion!(Timestamp, OffsetDateTime);

pub use time::Date;

/// Enforces UTC
#[derive(Debug, Clone, Copy)]
pub struct Time(time::Time);

impl Time {
	pub fn display_html(self) -> impl Display {
		struct Helper(time::Time);

		impl Display for Helper {
			fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
				self
					.0
					.format_into(&mut FmtToIo(f), html_time::FORMAT)
					.unwrap();
				Ok(())
			}
		}

		Helper(self.0)
	}
}

struct FmtToIo<F>(pub F);

impl<F: fmt::Write> std::io::Write for FmtToIo<F> {
	fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
		match std::str::from_utf8(bytes) {
			Ok(s) => {
				self.0.write_str(s).unwrap();
				Ok(bytes.len())
			}
			Err(err) => match err.valid_up_to() {
				0 => Err(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"data written is not valid UTF-8",
				)),
				up_to => {
					self.write_all(&bytes[..up_to])?;
					Ok(up_to)
				}
			},
		}
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

/// for `#[serde(with)]`
pub mod html_date {
	#![allow(clippy::trivially_copy_pass_by_ref)] // needed to provide the serde interface
	use std::fmt::{self, Display, Formatter};

	use serde::de::{self, Deserialize, Deserializer};
	use serde::ser::Serializer;

	use super::Date;

	pub const FORMAT: &[time::format_description::FormatItem<'static>] =
		time::macros::format_description!("[year]-[month]-[day]");

	pub fn format(date: Date) -> impl Display {
		struct Helper(Date);

		impl Display for Helper {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				let date = self.0;
				let mut writer = super::FmtToIo(formatter);
				date.format_into(&mut writer, FORMAT).unwrap();
				Ok(())
			}
		}

		Helper(date)
	}

	pub fn serialize<S: Serializer>(&date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(&format(date))
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error>
	where
		D::Error: de::Error,
	{
		let raw = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
		Ok(Date::parse(&raw, FORMAT).map_err(de::Error::custom)?)
	}
}

/// for `#[serde(with)]`
pub mod html_time {
	#![allow(clippy::trivially_copy_pass_by_ref)] // needed to provide the serde interface
	use std::fmt::{self, Display, Formatter};

	use serde::de::{self, Deserialize, Deserializer};
	use serde::ser::Serializer;

	use super::Time;

	pub static FORMAT: &[time::format_description::FormatItem<'static>] =
		time::macros::format_description!("[hour]:[minute]:[second].[subsecond digits:1+]");
	pub fn format(time: Time) -> impl Display {
		struct Helper(time::Time);

		impl Display for Helper {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				self
					.0
					.format_into(&mut super::FmtToIo(formatter), FORMAT)
					.unwrap();
				Ok(())
			}
		}

		Helper(time.0)
	}

	pub fn serialize<S: Serializer>(&time: &Time, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.collect_str(&format(time))
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Time, D::Error>
	where
		D::Error: de::Error,
	{
		let raw = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
		time::Time::parse(&raw, FORMAT)
			.map(Time)
			.map_err(de::Error::custom)
	}
}

pub mod unix {
	#![allow(clippy::trivially_copy_pass_by_ref)] // needed to provide the serde interface

	use serde::de::{self, Deserialize, Deserializer};
	use serde::ser::{Serialize, Serializer};

	use super::Timestamp;

	pub fn serialize<S: Serializer>(
		&timestamp: &Timestamp,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		timestamp
			.0
			.assume_utc()
			.unix_timestamp()
			.serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error>
	where
		D::Error: de::Error,
	{
		let unix = Deserialize::deserialize(deserializer)?;

		time::OffsetDateTime::from_unix_timestamp(unix)
			.map_err(serde::de::Error::custom)
			.map(Timestamp::from)
	}
}
