use smartstring::alias::String as SmartString;
use std::str::FromStr;

pub mod evaluate;
mod parse;

#[derive(Debug, PartialEq, Eq)]
pub enum Tag {
	Category(SmartString),
	Tag(SmartString),
	Both {
		category: SmartString,
		tag: SmartString,
	},
}

#[derive(Debug, PartialEq, Eq)]
pub enum ViewSpec {
	Tag(Tag),
	Not(Box<Self>),
	And(Box<Self>, Box<Self>),
	Or(Box<Self>, Box<Self>),
}

pub use parse::Error as ParseError;

impl FromStr for ViewSpec {
	type Err = ParseError;

	fn from_str(raw: &str) -> Result<Self, Self::Err> {
		parse::parse(raw)
	}
}

impl<'de> serde::Deserialize<'de> for ViewSpec {
	fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error>
	where
		D::Error: serde::de::Error,
	{
		Self::from_str(&<std::borrow::Cow<'de, str>>::deserialize(de)?)
			.map_err(serde::de::Error::custom)
	}
}
