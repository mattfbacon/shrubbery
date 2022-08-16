use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer};
use viewspec::lex;
use viewspec::lex::span::Span;
use viewspec::lex::token::Type as TokenType;
use viewspec::parse::{self, Ast};

#[derive(Debug)]
pub struct Error(pub parse::Error);

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

		#[derive(Clone, Copy)]
		enum ErrorLocus {
			Span(Span),
			AfterEnd,
		}

		use lex::Error as LE;
		use parse::Error as PE;
		let (message, locus, locus_message): (
			Cow<'static, str>,
			ErrorLocus,
			Option<Cow<'static, str>>,
		) = match &self.0 {
			PE::CategoryTooLong(span) => ("category is too long".into(), ErrorLocus::Span(*span), None),
			PE::ExpectedTagGot(Some((_span, TokenType::Error(error)))) => match &**error {
				LE::StringEnd => (
					"unexpected end of input".into(),
					ErrorLocus::AfterEnd,
					Some("more input needed here".into()),
				),
				LE::InvalidEscape(span, reason) => (
					"invalid string escape".into(),
					ErrorLocus::Span(*span),
					Some(reason.to_string().into()),
				),
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
					.map_or(ErrorLocus::AfterEnd, ErrorLocus::Span);
				(
					format!("expected tag, got {got_name}").into(),
					locus,
					Some("expected tag here".into()),
				)
			}
			PE::UnclosedParenthesis { open_location } => (
				"unclosed parenthesis".into(),
				ErrorLocus::Span(Span::single(*open_location)),
				Some("this opening parenthesis is not closed".into()),
			),
		};

		let ((before, within, after), len) = match locus {
			ErrorLocus::Span(span) => (span.split_three(raw).unwrap(), span.len()),
			ErrorLocus::AfterEnd => ((raw, " ", ""), 1),
		};
		let carets = Carets(len);
		let pipe = "<b class=\"error-block__note\"> | </b>";

		writeln!(f, "<pre class=\"error-block\"><code><strong><span class=\"error-block__error\">error</span>: {message_e}</strong>", message_e = escape(&message))?;
		write!(f, "{pipe}\n{pipe}{before_e}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">{carets}", before_e = escape(before))?;
		if let Some(locus_message) = locus_message {
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
}

#[derive(Debug)]
pub struct ViewSpecOrError {
	pub parsed: Result<Ast, Error>,
	pub raw: String,
}

impl<'de> Deserialize<'de> for ViewSpecOrError {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = String::deserialize(deserializer)?;
		Ok(Self {
			parsed: viewspec::lex_and_parse(raw.bytes()).map_err(Error),
			raw,
		})
	}
}
