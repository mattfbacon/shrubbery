use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer};
use viewspec::lex;
use viewspec::lex::span::Span;
use viewspec::lex::token::Type as TokenType;
use viewspec::parse::{self, Ast};

#[derive(Debug)]
pub struct Error {
	pub raw: String,
	pub error: parse::Error,
}

impl Error {
	pub fn render(&self) -> impl Display + '_ {
		struct Helper<'a>(&'a Error);

		impl Display for Helper<'_> {
			fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
				self.0.render_into(formatter)
			}
		}

		Helper(self)
	}

	pub fn render_into(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

		write!(f, "<pre class=\"error-block\"><code>")?;
		match &self.error {
			parse::Error::CategoryTooLong(span) => {
				let (before, within, after) = span.split_three(&self.raw).unwrap();
				writeln!(
					f,
					"<strong><span class=\"error-block__error\">error</span>: category is too long</strong>"
				)?;
				writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				write!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				writeln!(f, "{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">{carets}</span>{}</span>{}",
					escape(before),
					escape(within),
					escape(after),
					carets = Carets(span.len()),
				)?;
			}
			parse::Error::ExpectedTagGot(Some((_span, TokenType::Error(ref error)))) => match &**error {
				lex::Error::StringEnd => {
					writeln!(
						f,
						"<strong><span class=\"error-block__error\">error</span>: unexpected end of input</strong>"
					)?;
					writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
					write!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
					writeln!(f, "{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">^ more input needed here</span>&nbsp;</span>", escape(&self.raw))?;
				}
				lex::Error::InvalidEscape(span, reason) => {
					let (before, within, after) = span.split_three(&self.raw).unwrap();

					writeln!(
						f,
						"<strong><span class=\"error-block__error\">error</span>: invalid string escape</strong>"
					)?;
					writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
					write!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
					writeln!(
						f,
						"{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">{carets} {reason}</span>{}</span>{}",
						escape(before),
						escape(within),
						escape(after),
						carets = Carets(span.len()),
					)?;
				}
				lex::Error::StringNotUtf8(..) => unreachable!(), // we already read (past) it as a string
			},
			parse::Error::ExpectedTagGot(got) => {
				let got_name = match got.as_ref().map(|(_span, ty)| ty) {
					None => "EOF",
					Some(TokenType::And) => "and operator",
					Some(TokenType::Or) => "or operator",
					Some(TokenType::Not) => "not operator",
					Some(TokenType::CloseParen) => "closing parenthesis",
					Some(TokenType::Colon) => "colon",
					Some(TokenType::Error(_)) => unreachable!(),
					Some(TokenType::String | TokenType::OpenParen) => unreachable!(), // these would have been consumed by expression2 as `tag` or the start of `OPEN_PAREN expression0 CLOSE_PAREN` respectively
				};
				writeln!(
					f,
					"<strong><span class=\"error-block__error\">error</span>: expected tag, got {got_name}</strong>"
				)?;
				writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				write!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				if let Some((span, _ty)) = got {
					let (before, within, after) = span.split_three(&self.raw).unwrap();
					writeln!(
						f,
						"{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">{carets} expected tag here</span>{}</span>{}",
						escape(before),
						escape(within),
						escape(after),
						carets = Carets(span.len()),
					)?;
				} else {
					writeln!(f, "{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">^ expected tag here</span>&nbsp;</span>", escape(&self.raw))?;
				}
			}
			parse::Error::UnclosedParenthesis { open_location } => {
				let (before, within, after) = Span::single(*open_location).split_three(&self.raw).unwrap();
				writeln!(
					f,
					"<strong><span class=\"error-block__error\">error</span>: unclosed parenthesis</strong>"
				)?;
				writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				write!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
				writeln!(f, "{}<span class=\"error-block__comment-span\"><span class=\"error-block__comment error-block__error\">^ this opening parenthesis is not closed</span>{}</span>{}",
					escape(before),
					escape(within),
					escape(after),
				)?;
			}
		}
		for _ in 0..2 {
			writeln!(f, "<b class=\"error-block__note\">&nbsp;|&nbsp;</b>")?;
		}
		writeln!(f, "</code></pre>")?;
		Ok(())
	}
}

#[derive(Debug)]
pub struct ViewSpecOrError(pub Result<Ast, Error>);

impl<'de> Deserialize<'de> for ViewSpecOrError {
	fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = <Cow<'de, str>>::deserialize(deserializer)?;
		Ok(Self(match viewspec::lex_and_parse(raw.bytes()) {
			Ok(ast) => Ok(ast),
			Err(error) => Err(Error {
				raw: raw.into_owned(),
				error,
			}),
		}))
	}
}
