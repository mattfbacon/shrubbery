//! Provides the [`File`] type which allows extracting a multipart field to a temporary file.

use std::marker::PhantomData;

use async_trait::async_trait;
use mime::Mime;
use multer::Field;
use tempfile::{NamedTempFile, TempPath};
use tokio::io::AsyncWriteExt as _;

use crate::error::{Error, Result};
use crate::fields::FromSingleMultipartField;

/// Allows extraction of a multipart field to a temporary file.
///
/// It uses the `MakeTempfileImpl` generic parameter to allow customizing the creation of the temporary file. This parameter defaults to [`MakeTempfileDefault`] which just uses `tempfile`'s default behavior.
pub struct File<MakeTempfileImpl: MakeTempfile = MakeTempfileDefault> {
	/// The content type of the *multipart field*, if one was provided.
	///
	/// This is provided by the client, not determined by us.
	pub content_type: Option<Mime>,
	/// The path where the temporary file with the data from the multipart field resides.
	pub temp_path: TempPath,
	/// The *file name* of the *multipart field*, if one was provided.
	///
	/// This differs from the name of the multipart field itself.
	pub file_name: Option<String>,
	/// The size of the file, in bytes.
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

/// A way for [File] to create its temporary file.
///
/// Implementations are given access to the HTTP extensions of the request being processed.
// `Send` and `Sync` bound are required for `FromSingleMultipartField` impl
pub trait MakeTempfile: Send + Sync {
	/// Gets this temporary file maker from the HTTP extensions of the request being processed.
	///
	/// Access to the extensions is useful, for example, to get the app's configuration, which has been stored as an extension, in order to determine the directory for temporary files.
	fn extract_from_extensions(extensions: &http::Extensions) -> Self;
	/// Use this temporary file maker to make a [`NamedTempFile`].
	///
	/// # Errors
	///
	/// Generally just propagates the error returned by `NamedTempFile::new`.
	fn tempfile(&self) -> std::io::Result<NamedTempFile>;
}

/// Provides the default behavior for creating [File]'s temporary file
///
/// It ignores the HTTP extensions and uses `tempfile::NamedTempFile::new()`.
#[derive(Debug, Clone, Copy)]
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
