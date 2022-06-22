use std::collections::hash_map::{Entry, HashMap};
use std::fmt::{Debug, Display};

use axum::extract::multipart::{Field, Multipart, MultipartError};
use axum::response::ErrorResponse;
use serde::de::value::MapDeserializer;
use serde::de::DeserializeOwned;

use crate::database::models;
use crate::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("duplicate value for field {0:?}")]
	DuplicateField(String),
	#[error("multipart error: {0}")]
	Multipart(#[from] MultipartError),
	#[error("multipart field had no name")]
	NoName,
	#[error("{0}")]
	Custom(String),
}
crate::error::impl_response!(Error, BAD_REQUEST);

impl serde::de::Error for Error {
	fn custom<T: Display>(value: T) -> Self {
		Self::Custom(value.to_string())
	}
}

pub async fn get_one<'a>(
	multipart: &'a mut Multipart,
	name: &'static str,
) -> Result<Field<'a>, ErrorResponse> {
	let field = multipart
		.next_field()
		.await
		.map_err(error::Multipart)?
		.ok_or(error::ExpectedField(name))?;
	if field.name() != Some(name) {
		return Err(error::WrongFieldOrder(name).into());
	}
	Ok(field)
}

pub async fn get_one_text<'a>(
	multipart: &'a mut Multipart,
	name: &'static str,
) -> Result<String, ErrorResponse> {
	Ok(
		get_one(multipart, name)
			.await?
			.text()
			.await
			.map_err(error::Multipart)?,
	)
}

pub async fn deserialize_from_multipart<D: DeserializeOwned>(
	multipart: &mut Multipart,
) -> Result<D, Error> {
	let mut data: HashMap<String, String> = HashMap::new();

	while let Some(field) = multipart.next_field().await? {
		let name = match field.name() {
			Some(name) => name.to_owned(),
			None => return Err(Error::NoName), // name must be present for form data
		};
		let content = field.text().await?.to_owned();
		match data.entry(name) {
			Entry::Vacant(entry) => entry.insert(content),
			Entry::Occupied(entry) => return Err(Error::DuplicateField(entry.key().clone())),
		};
	}

	let deserializer = MapDeserializer::new(data.into_iter());
	D::deserialize(deserializer)
}

pub struct WriteToFile<'a> {
	file_field: Field<'a>,
}

impl<'a> WriteToFile<'a> {
	pub async fn start(
		multipart: &'a mut Multipart,
		field_name: &'static str,
	) -> Result<(models::MediaType, WriteToFile<'a>), ErrorResponse> {
		let file_field = crate::helpers::multipart::get_one(multipart, field_name).await?;
		let media_type = match file_field.content_type().and_then(|ct| ct.split_once('/')) {
			Some(("image", _)) => models::MediaType::Image,
			Some(("video", _)) => models::MediaType::Video,
			Some((_, _)) | None => {
				return Err(
					error::BadRequest(std::borrow::Cow::Borrowed(
						"missing, invalid, or unrecognized Content-Type for file field",
					))
					.into(),
				)
			}
		};

		Ok((media_type, Self { file_field }))
	}

	#[inline]
	pub async fn replace(
		self,
		file_id: models::FileId,
		config: &'_ crate::config::Config,
	) -> Result<(), ErrorResponse> {
		self.finish(true, file_id, config).await
	}

	#[inline]
	pub async fn create(
		self,
		file_id: models::FileId,
		config: &'_ crate::config::Config,
	) -> Result<(), ErrorResponse> {
		self.finish(false, file_id, config).await
	}

	async fn finish(
		mut self,
		replacement: bool,
		file_id: models::FileId,
		config: &'_ crate::config::Config,
	) -> Result<(), ErrorResponse> {
		use futures::TryStreamExt as _;
		use tokio::io::AsyncWriteExt as _;

		let final_path = config.file_storage.join(file_id.to_string());
		if replacement && !final_path.exists() {
			return Err(error::EntityNotFound("file").into());
		}

		let mut temp_file = crate::helpers::TempFile::create(&final_path)
			.await
			.map_err(|err| error::Io("opening temporary storage file", err))?;
		while let Some(chunk) = self.file_field.try_next().await.map_err(error::Multipart)? {
			temp_file
				.as_mut()
				.write_all(&chunk)
				.await
				.map_err(|err| error::Io("writing to temporary storage file", err))?;
		}
		temp_file
			.move_into_place()
			.await
			.map_err(|err| error::Io("overwriting file with temporary file", err))?;

		Ok(())
	}
}
