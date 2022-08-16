#![forbid(unsafe_code)]

extern crate self as axum_easy_multipart;

use async_trait::async_trait;
use multer::Multipart;

pub mod error;
pub mod extractor;
pub mod fields;
#[cfg(feature = "file")]
pub mod file;
mod impls;
#[cfg(test)]
mod test;

pub use error::{Error, Result};
pub use extractor::Extractor;

#[async_trait]
pub trait FromMultipart: Sized {
	async fn from_multipart(
		multipart: &mut Multipart<'_>,
		extensions: &http::Extensions,
	) -> Result<Self>;
}

pub use axum_easy_multipart_derive::FromMultipart;

#[doc(hidden)]
pub mod __private {
	pub use {async_trait, http, multer};
}
