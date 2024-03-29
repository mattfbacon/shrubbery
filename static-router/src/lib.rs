//! A static file router that serves the directory dynamically in debug mode and statically embeds the files and routes in release mode.
//!
//! To get started, see the [`static_router!`] macro.

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

use std::path::PathBuf;

use proc_macro2::{Literal, TokenStream, TokenTree};
use proc_macro_error::{abort, abort_call_site, proc_macro_error};
use quote::quote;

mod mime;

/// Create a static files router with the given name and static directory path.
///
/// The static directory path is relative to the crate root, not the caller file.
///
/// # Syntax
///
/// The macro expects as arguments an identifier, then a comma, then a string literal.
///
/// # Examples
///
/// Create a router named `router` that serves the files in `static`.
///
/// ```no_compile
/// static_router!(router, "static");
/// ```
#[proc_macro_error]
#[proc_macro]
pub fn static_router(tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let tokens = TokenStream::from(tokens);
	let mut tokens = tokens.into_iter();

	let router_name = match tokens
		.next()
		.unwrap_or_else(|| abort_call_site!("expected router name"))
	{
		TokenTree::Ident(ident) => ident,
		other => abort!(other, "expected router name"),
	};

	match tokens
		.next()
		.unwrap_or_else(|| abort_call_site!("expected comma"))
	{
		TokenTree::Punct(punct) if punct.as_char() == ',' => (),
		other => abort!(other, "expected comma"),
	}

	let static_path = match tokens
		.next()
		.unwrap_or_else(|| abort_call_site!("expected static directory path"))
	{
		ref token @ TokenTree::Literal(ref literal) => match litrs::Literal::from(literal) {
			litrs::Literal::String(s_lit) => s_lit.into_value().into_owned(),
			_ => abort!(token, "expected static directory path"),
		},
		other => abort!(other, "expected static directory path"),
	};

	let static_router = make_static_router(&static_path);
	let dynamic_router = make_dynamic_router(&static_path);

	quote! {
		pub fn #router_name() -> ::axum::Router {
			#[cfg(debug_assertions)]
			{ #dynamic_router }
			#[cfg(not(debug_assertions))]
			{ #static_router }
		}
	}
	.into()
}

fn make_static_router(root_path: &str) -> TokenStream {
	let root_path = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join(root_path);

	let routes = walkdir::WalkDir::new(&root_path)
		.follow_links(true)
		.into_iter()
		.filter_map(|entry| match entry {
			Ok(entry) => {
				let actual_path = entry.path();

				if entry.file_type().is_dir() {
					return None;
				}

				let user_path = actual_path.strip_prefix(&root_path).unwrap().to_str().unwrap();
				let user_path = format!("/{user_path}");

				let actual_path_lit = Literal::string(actual_path.to_str().unwrap());
				let user_path_lit = Literal::string(&user_path);

				let mime = actual_path.extension().unwrap_or_else(|| abort_call_site!("missing extension on {:?}: needed to determine MIME type", actual_path)).to_str().and_then(mime::ext_to_mime).unwrap_or_else(|| abort_call_site!("invalid or unrecognized extension on {:?}", actual_path));
				let mime_lit = Literal::string(mime);

				Some(quote! {
					router = router.route(#user_path_lit, ::axum::routing::get(|| ([("Content-Type", #mime_lit)], ::std::include_bytes!(#actual_path_lit))));
				})
			}
			Err(error) => abort_call_site!("error walking directories: {}", error),
		});

	quote! {
		let mut router = ::axum::Router::new();
		#(#routes)*
		router
	}
}

fn make_dynamic_router(path: &str) -> TokenStream {
	quote! {
		::axum::Router::new().fallback(::axum::routing::get_service(::tower_http::services::ServeDir::new(#path)).handle_error(|err| async move { crate::error::Io("reading static file", err) }))
	}
}
