//! Provides the [`Fields`] extraction helper as well as [`FromMultipartField`] and [`FromSingleMultipartField`] traits.

use async_trait::async_trait;
use bytes::Bytes;
use multer::{Error as MultipartError, Field, Multipart};

use crate::error::Error;

/// A multipart field extraction helper that allows extracting fields by name as well as peeking at fields.
// It also statically preserves multer's field exclusivity guarantee.
#[derive(Debug)]
pub struct Fields<'r, 'a> {
	multipart: &'a mut Multipart<'r>,
	field: Option<Field<'r>>,
}

impl<'r, 'a> Fields<'r, 'a> {
	/// Create a new [Fields] based on the multipart data in `multipart`.
	pub fn new(multipart: &'a mut Multipart<'r>) -> Self {
		Self {
			multipart,
			field: None,
		}
	}

	/// Peek at the next field of the multipart data without consuming it.
	///
	/// # Errors
	///
	/// Same as the `next_field` method of [`Multipart`].
	pub async fn peek(&mut self) -> Result<Option<&Field<'r>>, MultipartError> {
		match self.field {
			Some(ref field) => Ok(Some(field)),
			None => match self.multipart.next_field().await? {
				Some(next_field) => {
					self.field = Some(next_field);
					Ok(self.field.as_ref())
				}
				None => Ok(None),
			},
		}
	}

	/// Get the next field of the multipart data.
	///
	/// # Errors
	///
	/// Same as the `next_field` method of [`Multipart`].
	pub async fn next(&mut self) -> Result<Option<Field<'r>>, MultipartError> {
		match self.field.take() {
			Some(field) => Ok(Some(field)),
			None => self.multipart.next_field().await,
		}
	}
}

impl<'r> Fields<'r, '_> {
	/// Extract a field by the name `field_name`.
	///
	/// # Errors
	///
	/// - `Error::UnexpectedEnd` if there are no more fields left in the multipart data.
	/// - `Error::ExpectedField` if the extracted field has the wrong name.
	/// - `Error::Multipart` if any multipart error occurs.
	pub async fn extract_field(&mut self, field_name: &str) -> Result<Field<'r>, Error> {
		let field = self
			.next()
			.await
			.map_err(Error::Multipart)?
			.ok_or(Error::UnexpectedEnd)?;
		if field.name() == Some(field_name) {
			Ok(field)
		} else {
			Err(Error::ExpectedField(field_name.to_owned()))
		}
	}

	/// Extract the UTF-8 text of a field by the name `field_name`.
	///
	/// # Errors
	///
	/// Same as `extract_field`, or if getting the text of the field causes a multipart error.
	#[inline]
	pub async fn extract_text(&mut self, field_name: &str) -> Result<String, Error> {
		self
			.extract_field(field_name)
			.await?
			.text()
			.await
			.map_err(Error::Multipart)
	}

	/// Extract the bytes of a field by the name `field_name`.
	///
	/// # Errors
	///
	/// Same as `extract_field`, or if getting the bytes of the field causes a multipart error.
	#[inline]
	pub async fn extract_bytes(&mut self, field_name: &str) -> Result<Bytes, Error> {
		self
			.extract_field(field_name)
			.await?
			.bytes()
			.await
			.map_err(Error::Multipart)
	}
}

/// Allows extraction of a type from a multipart field(s) with a given name.
///
/// This differs from [FromSingleMultipartField] in that it is given access to the [Fields] instance directly, so it can be used to extract sequences such as `Vec` and `HashSet`, which are represented as a sequence of fields with the same name, and `Option`s, where `None` is represented as no field with the given name.
#[async_trait]
pub trait FromMultipartField: Sized {
	/// Extract the type from the multipart field(s) with the name `field_name`, using `fields` to get them.
	///
	/// The implementation gets access to the HTTP extensions in case any subsidiary extractors need them.
	async fn from_multipart_field(
		fields: &mut Fields<'_, '_>,
		field_name: &str,
		extensions: &http::Extensions,
	) -> crate::error::Result<Self>;
}

/// Allows extraction of a type from a single multipart field.
///
/// This differs from [FromMultipartField] in that implementations are given an already-extracted field to process, rather than being allowed to extract zero or more fields themselves.
///
/// However, all implementors also get a [FromMultipartField] implementation for free, which extracts a single field with the given name and passes it to `from_single_multipart_field`.
#[async_trait]
pub trait FromSingleMultipartField: Sized {
	/// Extract the type from the multipart field `field`.
	///
	/// The implementation gets access to the HTTP extensions so it can vary its extraction behavior.
	async fn from_single_multipart_field(
		field: Field<'_>,
		extensions: &http::Extensions,
	) -> crate::error::Result<Self>;
}

#[async_trait]
impl<T: FromSingleMultipartField> FromMultipartField for T {
	async fn from_multipart_field(
		fields: &mut Fields<'_, '_>,
		field_name: &str,
		extensions: &http::Extensions,
	) -> crate::error::Result<Self> {
		let field = fields.extract_field(field_name).await?;
		T::from_single_multipart_field(field, extensions).await
	}
}
