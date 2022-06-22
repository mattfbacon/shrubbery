#![allow(dead_code)]

use chrono::Utc;

pub type Timestamp = chrono::DateTime<Utc>;
pub type Date = chrono::Date<Utc>;
pub type Time = chrono::NaiveTime;

pub fn now() -> Timestamp {
	Utc::now()
}

pub fn is_in_past(t: &Timestamp) -> bool {
	t < &now()
}

/// for `#[serde(with)]`
pub mod html_date {
	use chrono::Utc;
	use serde::de::{self, Deserialize, Deserializer};
	use serde::ser::{Serialize, Serializer};

	use super::Date;

	pub static FORMAT: &str = "%Y-%m-%d";
	pub fn format(date: &Date) -> impl std::fmt::Display {
		date.naive_utc().format(FORMAT)
	}

	pub fn serialize<S: Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
		format(date).to_string().serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error>
	where
		D::Error: de::Error,
	{
		let raw = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
		let naive = chrono::NaiveDate::parse_from_str(&raw, FORMAT).map_err(de::Error::custom)?;
		Ok(Date::from_utc(naive, Utc))
	}
}

/// for `#[serde(with)]`
pub mod html_time {
	use serde::de::{self, Deserialize, Deserializer};
	use serde::ser::{Serialize, Serializer};

	use super::Time;

	pub static FORMAT: &str = "%T%.f";
	pub fn format(time: &Time) -> impl std::fmt::Display {
		time.format(FORMAT)
	}

	pub fn serialize<S: Serializer>(time: &Time, serializer: S) -> Result<S::Ok, S::Error> {
		format(time).to_string().serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Time, D::Error>
	where
		D::Error: de::Error,
	{
		let raw = <std::borrow::Cow<'_, str>>::deserialize(deserializer)?;
		chrono::NaiveTime::parse_from_str(&raw, FORMAT).map_err(de::Error::custom)
	}
}
