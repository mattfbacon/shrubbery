use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer};
use viewspec::lex;
use viewspec::lex::span::Span;
use viewspec::lex::token::Type as TokenType;
use viewspec::parse::tag::Ref as TagRef;
use viewspec::parse::{self, Ast};

use crate::eval_viewspec::UserError;

#[derive(Debug)]
pub enum Error {
	Parse(parse::Error),
	User { parsed: Ast, error: UserError },
}

#[derive(Clone, Copy)]
enum Locus {
	Span(Span),
	AfterEnd,
}

struct Diagnostic {
	message: Cow<'static, str>,
	locus: Locus,
	locus_message: Option<Cow<'static, str>>,
}

impl Diagnostic {
	fn new_spanned(message: impl Into<Cow<'static, str>>, span: Span) -> Self {
		Self {
			message: message.into(),
			locus: Locus::Span(span),
			locus_message: None,
		}
	}
	fn new_after_end(
		message: impl Into<Cow<'static, str>>,
		end_message: impl Into<Cow<'static, str>>,
	) -> Self {
		Self {
			message: message.into(),
			locus: Locus::AfterEnd,
			locus_message: Some(end_message.into()),
		}
	}
}

impl Error {
	pub fn render<'a>(&'a self, raw: &'a str) -> impl Display + 'a {
		struct Helper<'a> {
			error: &'a Error,
			raw: &'a str,
		}

		impl Display for Helper<'_> {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				self.error.render_into(formatter, self.raw)
			}
		}

		Helper { error: self, raw }
	}

	pub fn render_into(&self, f: &mut Formatter<'_>, raw: &str) -> fmt::Result {
		fn escape(s: &str) -> impl Display + '_ {
			askama_escape::MarkupDisplay::new_unsafe(s, askama_escape::Html)
		}

		struct Carets(u32);
		impl Display for Carets {
			fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
				for _ in 0..self.0 {
					write!(f, "^")?;
				}
				Ok(())
			}
		}

		let diagnostic = self.to_diagnostic();

		let ((before, within, after), len) = match diagnostic.locus {
			Locus::Span(span) => (span.split_three(raw).unwrap(), span.len()),
			Locus::AfterEnd => ((raw, " ", ""), 1),
		};
		let carets = Carets(len);
		let pipe = "<b class=\"error-block__note\"> | </b>";

		writeln!(f, "<pre class=\"error-block\"><code><strong><span class=\"error-block__error\">error</span>: {message_e}</strong>", message_e = escape(&diagnostic.message))?;
		write!(f, "{pipe}\n{pipe}{before_e}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">{carets}", before_e = escape(before))?;
		if let Some(locus_message) = diagnostic.locus_message {
			write!(f, " {}", escape(&locus_message))?;
		}
		writeln!(
			f,
			"</span>{within_e}</span>{after_e}",
			within_e = escape(within),
			after_e = escape(after)
		)?;
		writeln!(f, "{pipe}\n{pipe}\n</code></pre>")?;
		Ok(())
	}

	fn to_diagnostic(&self) -> Diagnostic {
		use lex::Error as LE;
		use parse::Error as PE;
		use UserError as UE;
		match self {
			Self::Parse(parse_error) => match parse_error {
				PE::CategoryTooLong(span) => Diagnostic::new_spanned("category is too long", *span),
				PE::ExpectedTagGot(Some((_span, TokenType::Error(error)))) => match &**error {
					LE::StringEnd => {
						Diagnostic::new_after_end("unexpected end of input", "more input needed here")
					}
					LE::InvalidEscape(span, reason) => Diagnostic {
						message: "invalid string escape".into(),
						locus: Locus::Span(*span),
						locus_message: Some(reason.to_string().into()),
					},
					LE::StringNotUtf8(..) => unreachable!(),
				},
				PE::ExpectedTagGot(got) => {
					let got_name = match got.as_ref().map(|(_span, ty)| ty) {
						None => "EOF",
						Some(TokenType::And) => "and operator",
						Some(TokenType::Or) => "or operator",
						Some(TokenType::Not) => "not operator",
						Some(TokenType::CloseParen) => "closing parenthesis",
						Some(TokenType::Colon) => "colon",
						Some(TokenType::Error(_)) => unreachable!("already checked"),
						Some(TokenType::String | TokenType::OpenParen) => {
							// these would have been consumed by expression2 as `tag` or the start of `OPEN_PAREN expression0 CLOSE_PAREN` respectively
							unreachable!("parser doesn't work like that")
						}
					};
					let locus = got
						.as_ref()
						.map(|(span, _ty)| *span)
						.map_or(Locus::AfterEnd, Locus::Span);
					Diagnostic {
						message: format!("expected tag, got {got_name}").into(),
						locus,
						locus_message: Some("expected tag here".into()),
					}
				}
				PE::UnclosedParenthesis { open_location } => Diagnostic {
					message: "unclosed parenthesis".into(),
					locus: Locus::Span(Span::single(*open_location)),
					locus_message: Some("this opening parenthesis is not closed".into()),
				},
			},
			Self::User {
				parsed,
				error: user_error,
			} => {
				let span = match user_error {
					UE::NoTagsByName(name) => parsed
						.find_map_tag(|tag| match tag.as_ref() {
							TagRef::Name(this_name, span) if name == this_name => Some(span),
							_ => None,
						})
						.unwrap(),
					UE::UnknownTag { category, name } => parsed
						.find_map_tag(|tag| match tag.as_ref() {
							TagRef::Both {
								category: this_category,
								category_span,
								name: this_name,
								name_span,
							} if category == this_category && name == this_name => Some(Span {
								start: std::cmp::min(category_span.start, name_span.start),
								end: std::cmp::max(category_span.end, name_span.end),
							}),
							_ => None,
						})
						.unwrap(),
					UE::UnknownTagCategory(category) => parsed
						.find_map_tag(|tag| match tag.as_ref() {
							TagRef::Category(this_category, category_span)
							| TagRef::Both {
								category: this_category,
								category_span,
								..
							} if category == this_category => Some(category_span),
							_ => None,
						})
						.unwrap(),
				};
				let entity_name = match user_error {
					UE::NoTagsByName(..) => "name",
					UE::UnknownTag { .. } => "tag",
					UE::UnknownTagCategory(..) => "category",
				};
				Diagnostic {
					message: user_error.to_string().into(),
					locus: Locus::Span(span),
					locus_message: Some(format!("first occurrence of the nonexistent {entity_name}").into()),
				}
			}
		}
	}
}

#[derive(Debug)]
pub struct ViewSpecOrError {
	pub parsed: Result<Ast, parse::Error>,
	pub raw: String,
}

impl<'de> Deserialize<'de> for ViewSpecOrError {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = String::deserialize(deserializer)?;
		Ok(Self {
			parsed: viewspec::lex_and_parse(raw.bytes()),
			raw,
		})
	}
}
