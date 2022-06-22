use async_trait::async_trait;
use bytes::Bytes;
use multer::Field;

use crate::error::{Error, Result};
use crate::fields::{Fields, FromMultipartField, FromSingleMultipartField};

#[async_trait]
impl FromSingleMultipartField for String {
    async fn from_single_multipart_field(field: Field<'_>) -> Result<Self> {
        Ok(field.text().await?)
    }
}

#[async_trait]
impl FromSingleMultipartField for Bytes {
    async fn from_single_multipart_field(field: Field<'_>) -> Result<Self> {
        Ok(field.bytes().await?)
    }
}

macro_rules! impl_from_str {
    ($ty:ty) => {
        #[async_trait]
        impl FromSingleMultipartField for $ty {
            async fn from_single_multipart_field(field: Field<'_>) -> Result<Self> {
                let string = String::from_single_multipart_field(field).await?;
                string.parse::<$ty>().map_err(|err| Error::Custom {
                    target: stringify!($ty),
                    error: err.to_string(),
                })
            }
        }
    };
    ($($ty:ty),+ $(,)?) => {
        $(impl_from_str!($ty);)+
    }
}

impl_from_str![u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64, bool,];

#[async_trait]
impl<T: FromSingleMultipartField> FromMultipartField for Option<T> {
    async fn from_multipart_field(fields: &mut Fields<'_, '_>, field_name: &str) -> Result<Self> {
        let peeked = fields.peek().await?.ok_or(Error::UnexpectedEnd)?;
        if peeked.name() == Some(field_name) {
            Ok(Some(
                T::from_single_multipart_field(fields.next().await?.unwrap()).await?,
            ))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl<T: FromSingleMultipartField + Send> FromMultipartField for Vec<T> {
    async fn from_multipart_field(fields: &mut Fields<'_, '_>, field_name: &str) -> Result<Self> {
        let mut ret = Vec::new();
        while let Some(peeked) = fields.peek().await? {
            if peeked.name() != Some(field_name) {
                break;
            }
            ret.push(T::from_single_multipart_field(fields.next().await?.unwrap()).await?);
        }
        Ok(ret)
    }
}
