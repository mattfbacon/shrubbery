#![forbid(unsafe_code)]

extern crate self as axum_easy_multipart;

use async_trait::async_trait;
use multer::Multipart;

pub mod error;
pub mod fields;
#[cfg(feature = "file")]
pub mod file;
mod impls;
#[cfg(test)]
mod test;

pub use error::{Error, Result};

#[async_trait]
pub trait FromMultipart: Sized {
    async fn from_multipart(multipart: &mut Multipart<'_>) -> Result<Self>;
}

pub use axum_easy_multipart_derive::FromMultipart;

pub mod exports {
    pub use multer;
}
