use async_trait::async_trait;
use bytes::Bytes;
use multer::Field;

use crate::error::{Error, Result};
use crate::fields::{Fields, FromMultipartField, FromSingleMultipartField};

#[async_trait]
impl FromSingleMultipartField for String {
	async fn from_single_multipart_field(
		field: Field<'_>,
		_extensions: &http::Extensions,
	) -> Result<Self> {
		Ok(field.text().await?)
	}
}

#[async_trait]
impl FromSingleMultipartField for Bytes {
	async fn from_single_multipart_field(
		field: Field<'_>,
		_extensions: &http::Extensions,
	) -> Result<Self> {
		Ok(field.bytes().await?)
	}
}

/// Implement [`FromSingleMultipartField`] for a type based on its `FromStr` implementation.
///
/// The implementation will extract the field as a `String`, then use the type's `FromStr` implementation.
/// The error from the `FromStr` implementation must implement `Display`.
#[macro_export]
macro_rules! impl_for_from_str {
	($ty:ty) => {
		#[$crate::__private::async_trait::async_trait]
		impl $crate::fields::FromSingleMultipartField for $ty {
			async fn from_single_multipart_field(field: $crate::__private::multer::Field<'_>, extensions: &$crate::__private::http::Extensions) -> $crate::error::Result<Self> {
				let string = <String as $crate::fields::FromSingleMultipartField>::from_single_multipart_field(field, extensions).await?;
				string.parse::<$ty>().map_err(|err| $crate::error::Error::Custom {
					target: stringify!($ty),
					error: err.to_string(),
				})
			}
		}
	};
	($($ty:ty),+ $(,)?) => {
		$(impl_for_from_str!($ty);)+
	}
}

impl_for_from_str![u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, bool,];

#[async_trait]
impl<T: FromSingleMultipartField> FromMultipartField for Option<T> {
	async fn from_multipart_field(
		fields: &mut Fields<'_, '_>,
		field_name: &str,
		extensions: &http::Extensions,
	) -> Result<Self> {
		let peeked = fields.peek().await?.ok_or(Error::UnexpectedEnd)?;
		if peeked.name() == Some(field_name) {
			Ok(Some(
				T::from_single_multipart_field(fields.next().await?.unwrap(), extensions).await?,
			))
		} else {
			Ok(None)
		}
	}
}

/// Implement [`FromMultipartField`] for a type with a single generic parameter based on its [`Default`] and [`Extend`] implementations.
///
/// This is intended for collection types.
/// The implementation will create the collection using `Default`, then extend it with `Extend` as it reads elements from the multipart request.
///
/// A bound of [`FromSingleMultipartField`] will be added to the generic parameter, as well as [`Send`] due to implementation details.
#[macro_export]
macro_rules! impl_for_default_plus_extend {
	($ty:ty) => {
		#[async_trait]
		impl<T: $crate::fields::FromSingleMultipartField + ::std::marker::Send> $crate::fields::FromMultipartField for $ty where $ty: ::std::default::Default + ::std::iter::Extend<T> {
			async fn from_multipart_field(
				fields: &mut $crate::fields::Fields<'_, '_>,
				field_name: &str,
				extensions: &$crate::__private::http::Extensions,
			) -> Result<Self> {
				let mut ret = <$ty>::default();
				while let Some(peeked) = fields.peek().await? {
					if peeked.name() != Some(field_name) {
						break;
					}
					ret.extend([T::from_single_multipart_field(fields.next().await?.unwrap(), extensions).await?]);
				}
				Ok(ret)
			}
		}
	};
	($($ty:ty),+ $(,)?) => {
		$(impl_for_default_plus_extend!($ty);)+
	}
}

impl_for_default_plus_extend!(
	Vec<T>,
	std::collections::HashSet<T>,
	std::collections::BTreeSet<T>,
);
