//! # Viewspec
//!
//! Lexing and parsing of "viewspecs", which are configurations for filtering items based on tags.
//!
//! The language may be extended later with properties or other features. Assuming I don't do anything stupid, those added features should always be backwards-compatible.

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
#![deny(unsafe_code)]

pub mod lex;
pub mod parse;

/// Lex and parse in one simple function.
///
/// # Errors
///
/// Returns a [`parse::Error`] variant if parsing fails.
pub fn lex_and_parse(input: impl Iterator<Item = u8>) -> parse::Result<parse::Ast> {
	parse::parse(lex::lex(input))
}
