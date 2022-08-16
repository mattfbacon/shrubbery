//! # Axum Easy Multipart
//!
//! Provides an easy way of extracting strongly typed data from multipart form requests.
//!
//! This crate provides the [`FromMultipart`] trait as well as [a derive macro](axum_easy_multipart_derive::FromMultipart) to make it easy to implement automatically.

#![warn(clippy::pedantic)]
#![warn(
	missing_copy_implementations,
	elided_lifetimes_in_paths,
	explicit_outlives_requirements,
	macro_use_extern_crate,
	meta_variable_misuse,
	missing_abi,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	noop_method_call,
	pointer_structural_match,
	single_use_lifetimes,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
	unused_lifetimes,
	unused_macro_rules,
	unused_qualifications,
	variant_size_differences
)]
#![allow(clippy::tabs_in_doc_comments)] // rustfmt formats our doc comments and we use tabs
#![forbid(unsafe_code)]

// to allow using the derive macro within the tests crate
#[cfg(test)]
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

/// Allows extraction of a type from an entire multipart request.
///
/// Typically this is implemented with the help of [the derive macro](derive@FromMultipart).
#[async_trait]
pub trait FromMultipart: Sized {
	/// Extract the type from a multipart request.
	///
	/// The implementation gets access to the HTTP extensions in case any subsidiary extractors need them.
	async fn from_multipart(
		multipart: &mut Multipart<'_>,
		extensions: &http::Extensions,
	) -> Result<Self>;
}

/// Derive the [`trait@FromMultipart`] trait for your struct or enum.
///
/// # Usage
///
/// All fields of the type must implement [`FromMultipartField`](fields::FromMultipartField).
/// Fields will be read in order from the multipart data, using their Rust identifier as their name. To override this, use the `rename` field attribute.
/// All fields must be named. This means that only named structs and enums with only named struct variants are allowed.
///
/// # Enums
///
/// For enums, you must use the `tag` container attribute to specify the name of the field that will contain the tag indicating which enum variant to read.
/// This field will be read first.
/// The value of the tag field will be compared with the Rust identifiers of the variants.
/// To override those names, use the `rename` variant attribute.
///
/// The order of the variants has no effect on the behavior of the generated implementation.
///
/// # Attributes
///
/// ## Container Attributes
///
/// ### `tag` (enums only)
///
/// Uses equals syntax and expects a string literal.
/// Explained above.
///
/// ## Field Attributes
///
/// ### `rename`
///
/// Uses equals syntax and expects a string literal.
/// Specifies a name to use for this field rather than its Rust identifier.
///
/// ## Variant Attributes
///
/// ### `rename`
///
/// Uses equals syntax and expects a string literal.
/// Specifies a name to use for this variant when checking the tag, rather than its Rust identifier.
///
/// # Examples
///
/// Structs:
///
/// ```no_compile
/// #[derive(FromMultipart)]
/// struct Data {
/// 	foo: u32,
/// 	bar: String,
/// 	baz: bytes::Bytes,
/// }
/// ```
///
/// Enums:
///
/// ```no_compile
/// #[derive(FromMultipart)]
/// #[multipart(tag = "action")]
/// enum PostRequest {
/// 	Rename { new_name: String, },
/// 	Delete {},
/// }
/// ```
///
/// Renaming struct fields:
///
/// ```no_compile
/// #[derive(FromMultipart)]
/// struct Data {
/// 	#[multipart(rename = "type")]
/// 	ty: String,
/// }
/// ```
///
/// Renaming enum variants, as well as fields within enums:
///
/// ```no_compile
/// #[derive(FromMultipart)]
/// #[multipart(tag = "type")]
/// enum KebabVariants {
/// 	#[multipart(rename = "play-game")]
/// 	PlayGame {
/// 		#[multipart(rename = "user")]
/// 		username: String,
/// 	},
/// 	#[multipart(rename = "leave")]
/// 	Leave {},
/// }
/// ```
pub use axum_easy_multipart_derive::FromMultipart;

#[doc(hidden)]
pub mod __private {
	pub use {async_trait, http, multer};
}
