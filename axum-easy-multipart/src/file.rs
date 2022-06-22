use async_trait::async_trait;
pub use mime::Mime;
use multer::Field;
use tempfile::{NamedTempFile, TempPath};
use tokio::io::AsyncWriteExt as _;

use crate::error::{Error, Result};
use crate::fields::FromSingleMultipartField;

#[derive(Debug)]
pub struct File {
    pub content_type: Option<Mime>,
    pub temp_path: TempPath,
    pub file_name: Option<String>,
    pub size: usize,
}

#[async_trait]
impl FromSingleMultipartField for File {
    async fn from_single_multipart_field<'a>(mut field: Field<'a>) -> Result<Self> {
        let (temp_file, temp_path) = NamedTempFile::new()
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
        })
    }
}
