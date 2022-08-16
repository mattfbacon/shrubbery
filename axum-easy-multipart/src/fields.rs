use async_trait::async_trait;
use bytes::Bytes;
use multer::{Error as MultipartError, Field, Multipart};

use crate::error::Error;

// This type statically preserves multer's field exclusivity guarantee
pub struct Fields<'r, 'a> {
	multipart: &'a mut Multipart<'r>,
	field: Option<Field<'r>>,
}

impl<'r, 'a> Fields<'r, 'a> {
	pub fn new(multipart: &'a mut Multipart<'r>) -> Self {
		Self {
			multipart,
			field: None,
		}
	}

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

	pub async fn next(&mut self) -> Result<Option<Field<'r>>, MultipartError> {
		match self.field.take() {
			Some(field) => Ok(Some(field)),
			None => self.multipart.next_field().await,
		}
	}
}

impl<'r, 'a> Fields<'r, 'a> {
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

	#[inline]
	pub async fn extract_text(&mut self, field_name: &str) -> Result<String, Error> {
		self
			.extract_field(field_name)
			.await?
			.text()
			.await
			.map_err(Error::Multipart)
	}

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

#[async_trait]
pub trait FromMultipartField: Sized {
	async fn from_multipart_field(
		fields: &mut Fields<'_, '_>,
		field_name: &str,
		extensions: &http::Extensions,
	) -> crate::error::Result<Self>;
}

#[async_trait]
pub trait FromSingleMultipartField: Sized {
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
