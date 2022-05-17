use serde::de::{Deserializer, Error as _};
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Deserialize, Debug)]
#[serde(from = "Impl<T>")]
pub struct OrNull<T>(Option<T>);

struct Null;

impl<'de> Deserialize<'de> for Null {
	fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
		match &*<Cow<'_, str>>::deserialize(de)? {
			"null" => Ok(Self),
			_ => Err(D::Error::custom("not null")),
		}
	}
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Impl<T> {
	Null(Null),
	Value(T),
}

impl<T> From<Impl<T>> for OrNull<T> {
	fn from(impl_: Impl<T>) -> Self {
		match impl_ {
			Impl::Null(..) => Self(None),
			Impl::Value(value) => Self(Some(value)),
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
