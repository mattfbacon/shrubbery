use proc_macro2::Span;
use syn::spanned::Spanned as _;
use syn::{Attribute, Error, Lit, LitStr, Meta, MetaList, MetaNameValue, Path};

use super::Result;

pub(crate) trait MetaValue: Sized {
	fn friendly_name() -> &'static str;
	fn extract(meta: Meta) -> Result<Self, Meta>;
}

impl MetaValue for MetaNameValue {
	fn friendly_name() -> &'static str {
		"name-value"
	}

	fn extract(meta: Meta) -> Result<Self, Meta> {
		match meta {
			Meta::NameValue(expr) => Ok(expr),
			other => Err(other),
		}
	}
}

impl MetaValue for MetaList {
	fn friendly_name() -> &'static str {
		"list"
	}

	fn extract(meta: Meta) -> Result<Self, Meta> {
		match meta {
			Meta::List(expr) => Ok(expr),
			other => Err(other),
		}
	}
}

impl MetaValue for Path {
	fn friendly_name() -> &'static str {
		"path"
	}

	fn extract(meta: Meta) -> Result<Self, Meta> {
		match meta {
			Meta::Path(expr) => Ok(expr),
			other => Err(other),
		}
	}
}

impl MetaValue for LitStr {
	fn friendly_name() -> &'static str {
		"string-valued name-value"
	}

	fn extract(meta: Meta) -> Result<Self, Meta> {
		match meta {
			Meta::NameValue(MetaNameValue {
				lit: Lit::Str(expr),
				..
			}) => Ok(expr),
			other => Err(other),
		}
	}
}

pub(crate) fn get<V: MetaValue>(
	attributes: &[Attribute],
	attribute_name: &str,
) -> Result<Option<V>> {
	use syn::punctuated::Punctuated;

	fn parse_meta_stream(
		tokens: syn::parse::ParseStream<'_>,
	) -> Result<Punctuated<Meta, syn::Token![,]>> {
		let inner;
		syn::parenthesized!(inner in tokens);
		Punctuated::parse_separated_nonempty(&inner)
	}

	let mut ret = None;

	for attribute in attributes {
		if !attribute
			.path
			.get_ident()
			.map_or(false, |path| path == "multipart")
		{
			continue;
		}

		let meta_stream = syn::parse::Parser::parse2(parse_meta_stream, attribute.tokens.clone())?;

		for meta in meta_stream {
			match &meta {
				Meta::NameValue(MetaNameValue { path, .. })
				| Meta::List(MetaList { path, .. })
				| Meta::Path(path)
					if path
						.get_ident()
						.map_or(false, |path| path == attribute_name) =>
				{
					if ret.is_some() {
						return Err(Error::new(
							attribute.span(),
							format!("duplicate value for {attribute_name:?} attribute"),
						));
					}
					ret = Some(V::extract(meta).map_err(|other_meta| {
						Error::new(
							other_meta.span(),
							format!(
								"expected {} meta for {attribute_name:?} attribute",
								V::friendly_name()
							),
						)
					})?);
				}
				_ => continue,
			}
		}
	}

	Ok(ret)
}

pub(crate) fn get_enum_tag(item_span: Span, attributes: &[Attribute]) -> Result<String> {
	let tag_name = get::<LitStr>(attributes, "tag")?.ok_or_else(|| Error::new(item_span, "enum requires \"tag\" attribute to specify the name of the field that determines which variant will be deserialized"))?;
	Ok(tag_name.value())
}

pub(crate) fn get_rename(attributes: &[Attribute]) -> Result<Option<LitStr>> {
	get::<LitStr>(attributes, "rename")
}
