use std::marker::PhantomData;

use async_trait::async_trait;
use mime::Mime;
use multer::Field;
use tempfile::{NamedTempFile, TempPath};
use tokio::io::AsyncWriteExt as _;

use crate::error::{Error, Result};
use crate::fields::FromSingleMultipartField;

pub struct File<MakeTempfileImpl: MakeTempfile = MakeTempfileDefault> {
	pub content_type: Option<Mime>,
	pub temp_path: TempPath,
	pub file_name: Option<String>,
	pub size: usize,
	_make_tempfile_impl: PhantomData<MakeTempfileImpl>,
}

impl<T: MakeTempfile> std::fmt::Debug for File<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("File")
			.field("content_type", &self.content_type)
			.field("temp_path", &self.temp_path)
			.field("file_name", &self.file_name)
			.field("size", &self.size)
			.finish()
	}
}

// `Send` and `Sync` bound are required for `FromSingleMultipartField` impl
pub trait MakeTempfile: Send + Sync {
	fn extract_from_extensions(extensions: &http::Extensions) -> Self;
	fn tempfile(&self) -> std::io::Result<NamedTempFile>;
}

#[derive(Debug)]
pub struct MakeTempfileDefault;

impl MakeTempfile for MakeTempfileDefault {
	fn extract_from_extensions(_extensions: &http::Extensions) -> Self {
		Self
	}

	fn tempfile(&self) -> std::io::Result<NamedTempFile> {
		tempfile::NamedTempFile::new()
	}
}

#[async_trait]
impl<MakeTempfileImpl: MakeTempfile> FromSingleMultipartField for File<MakeTempfileImpl> {
	async fn from_single_multipart_field<'a>(
		mut field: Field<'a>,
		extensions: &http::Extensions,
	) -> Result<Self> {
		let make_tempfile = MakeTempfileImpl::extract_from_extensions(extensions);
		let (temp_file, temp_path) = make_tempfile
			.tempfile()
			.map_err(|error| Error::Custom {
				target: "File",
				error: format!("while creating temporary file: {error}"),
			})?
			.into_parts();
		let mut temp_file = tokio::fs::File::from_std(temp_file);
		let mut size = 0usize;

		while let Some(chunk) = field.chunk().await.map_err(Error::Multipart)? {
			size += chunk.len();
			temp_file
				.write_all(&chunk)
				.await
				.map_err(|error| Error::Custom {
					target: "File",
					error: format!("while writing to temporary file: {error}"),
				})?;
		}

		Ok(Self {
			content_type: field.content_type().cloned(),
			temp_path,
			file_name: field.file_name().map(str::to_owned),
			size,
			_make_tempfile_impl: PhantomData,
		})
	}
}
