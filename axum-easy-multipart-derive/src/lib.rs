//! The derive macro companion of `axum-easy-multipart`.
//!
//! All documentation is in that crate.

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
#![forbid(unsafe_code)]

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;
use syn::{
	parse_macro_input, Attribute, Data, DataEnum, DeriveInput, Error, Fields, FieldsNamed, LitStr,
};

mod attribute;

type Result<T, E = Error> = core::result::Result<T, E>;

#[proc_macro_derive(FromMultipart, attributes(multipart))]
pub fn wrap_derive(input: TokenStream1) -> TokenStream1 {
	let input = parse_macro_input!(input as DeriveInput);
	derive(input).into()
}

fn derive(input: DeriveInput) -> TokenStream {
	let input_span = input.span();
	let name = input.ident;
	let generics = input.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

	let implementation = match input.data {
		Data::Struct(data) => derive_struct(input_span, data.fields, &quote!(Self)),
		Data::Enum(data) => derive_enum(&name, &input.attrs, data),
		Data::Union(_data) => Err(Error::new(
			input_span,
			"FromMultipart can only be derived on structs and enums",
		)),
	}
	.unwrap_or_else(Error::into_compile_error);

	quote! {
			#[::axum_easy_multipart::__private::async_trait::async_trait]
			impl #impl_generics ::axum_easy_multipart::FromMultipart for #name #ty_generics #where_clause {
					async fn from_multipart(multipart: &mut ::axum_easy_multipart::__private::multer::Multipart<'_>, extensions: &::axum_easy_multipart::__private::http::Extensions) -> ::axum_easy_multipart::Result<Self> {
							#[allow(unused_variables, unused_mut)]
							let mut fields = ::axum_easy_multipart::fields::Fields::new(multipart);
							#implementation
					}
			}
	}
}

fn derive_struct(span: Span, fields: Fields, ident: &TokenStream) -> Result<TokenStream> {
	let body = match fields {
		Fields::Named(fields) => derive_named_struct(&fields, ident),
		_ => Err(Error::new(
			span,
			"FromMultipart may only be derived on named structs",
		)),
	}?;

	Ok(body)
}

fn derive_named_struct(input: &FieldsNamed, ident: &TokenStream) -> Result<TokenStream> {
	let items = input.named
		.iter()
		.map(|field| -> Result<(_, _)> {
				let actual_ident = field.ident.as_ref().unwrap();
				let multipart_name = attribute::get_rename(&field.attrs)?.unwrap_or_else(|| LitStr::new(&actual_ident.to_string(), Span::call_site()));
				let internal_ident = make_internal_ident(actual_ident);
				let field_getter = quote! { let #internal_ident = ::axum_easy_multipart::fields::FromMultipartField::from_multipart_field(&mut fields, #multipart_name, extensions).await?; };
				let constructor_field = quote!{ #actual_ident: #internal_ident };
				Ok((field_getter, constructor_field))
		}).collect::<Result<Vec<_>>>()?;
	let field_getters = items.iter().map(|(field_getter, _)| field_getter);
	let constructor_fields = items.iter().map(|(_, constructor_field)| constructor_field);
	Ok(quote! {
			#(#field_getters)*
			Ok(#ident { #(#constructor_fields),* })
	})
}

fn derive_enum(name: &Ident, attributes: &[Attribute], input: DataEnum) -> Result<TokenStream> {
	let tag_field = attribute::get_enum_tag(input.enum_token.span(), attributes)?;

	let name = name.to_string();
	let mut variant_set = std::collections::HashSet::new();

	let bodies = input
		.variants
		.into_iter()
		.map(move |variant| -> Result<_> {
			let tag_override = attribute::get_rename(&variant.attrs)?;
			let tag_name =
				tag_override.unwrap_or_else(|| LitStr::new(&variant.ident.to_string(), Span::call_site()));
			if !variant_set.insert(tag_name.value()) {
				return Err(Error::new(tag_name.span(), "duplicate variant tag"));
			}
			let ident = &variant.ident;
			let body = derive_struct(variant.span(), variant.fields, &quote!(Self::#ident))?;
			Ok(quote! {
					#tag_name => {
							#body
					}
			})
		})
		.collect::<Result<Vec<_>>>()?;

	let body = quote! {
			let variant_tag = fields.extract_text(#tag_field).await?;
			#[deny(unreachable_patterns)]
			#[allow(match_single_binding)]
			match variant_tag.as_str() {
					#(#bodies)*
					_ => Err(::axum_easy_multipart::error::Error::Custom {
							target: #name,
							error: "no variants matched".to_owned(),
					})
			}
	};
	Ok(body)
}

fn make_internal_ident(ident: &Ident) -> Ident {
	format_ident!("__axum_easy_multipart_field_{}", ident)
}
