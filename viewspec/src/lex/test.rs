use std::fmt::Write as _;

use itertools::Itertools;
use rand::distributions::Distribution as _;
use rand::thread_rng as r;

use crate::lex::span::{Location, Span};
use crate::lex::token::{SpannedToken, Token};

fn lex_to_vec(input: &str) -> Vec<SpannedToken> {
	super::lex(input.bytes()).collect()
}

#[test]
fn bare_string() {
	assert_eq!(
		lex_to_vec(" ab c "),
		[SpannedToken {
			span: Span { start: 1, end: 4 },
			token: Token::String {
				content: "ab c".into(),
				bare: true,
			}
		}]
	);
}

struct GenerationCtx {
	current_location: Location,
	parsed: Vec<SpannedToken>,
	raw: String,
}

impl super::token::Type {
	fn random_string_len() -> usize {
		rand::Rng::gen_range(&mut r(), 0..=20)
	}

	fn random_char_iter() -> impl Iterator<Item = char> {
		rand::distributions::Uniform::from('\0'..'\u{ffff}').sample_iter(r())
	}

	fn random() -> Self {
		match rand::Rng::gen_range(&mut r(), 0..8) {
			0 | 1 => Self::String,
			2 => Self::And,
			3 => Self::Or,
			4 => Self::Not,
			5 => Self::OpenParen,
			6 => Self::CloseParen,
			7 => Self::Colon,
			_ => unreachable!("random token type index out of range"),
		}
	}

	fn random_iter() -> impl Iterator<Item = Self> {
		std::iter::repeat_with(Self::random)
	}

	fn generate(
		self,
		GenerationCtx {
			current_location,
			parsed,
			raw,
		}: &mut GenerationCtx,
	) {
		let (single_char, token) = match self {
			Self::String => {
				if rand::Rng::gen_bool(&mut r(), 0.5) {
					// quoted string
					let start_location = *current_location;
					let generated_string: String = Self::random_char_iter()
						.take(Self::random_string_len())
						.collect();
					let old_len = raw.len();
					write!(raw, "{generated_string:?}").unwrap();
					*current_location += super::Location::try_from(raw.len() - old_len).unwrap();

					parsed.push(
						super::Token::String {
							content: generated_string.into_boxed_str(),
							bare: false,
						}
						.with_span(Span {
							start: start_location,
							end: *current_location,
						}),
					);
				} else {
					let start_location = *current_location;
					let mut generated_string: String = Self::random_char_iter()
						.filter(|&ch| u8::try_from(ch).map_or(true, |ch_byte| !super::char_is_special(ch_byte))) // always allow Unicode
						.take(Self::random_string_len() + 1)
						.collect();
					// trim whitespace in place
					generated_string.truncate(generated_string.trim_end().len());
					generated_string.drain(..(generated_string.len() - generated_string.trim_start().len()));
					if generated_string.is_empty() {
						generated_string.push_str("abc"); // use a dummy rather than writing an empty string. use `push_str` to reuse the String's allocation
					}
					raw.push_str(&generated_string);
					*current_location += super::Location::try_from(generated_string.len()).unwrap();

					parsed.push(
						super::Token::String {
							content: generated_string.into_boxed_str(),
							bare: true,
						}
						.with_span(Span {
							start: start_location,
							end: *current_location,
						}),
					);
				}
				return;
			}
			Self::And => ('&', Token::And),
			Self::Or => ('|', Token::Or),
			Self::Not => ('!', Token::Not),
			Self::OpenParen => ('(', Token::OpenParen),
			Self::CloseParen => (')', Token::CloseParen),
			Self::Colon => (':', Token::Colon),
			Self::Error(_) => unreachable!("test token generator will never produce error tokens"),
		};
		raw.push(single_char);
		parsed.push(SpannedToken {
			span: Span::single(*current_location),
			token,
		});
		*current_location += 1;
	}
}

fn generate_random(num_components: usize) -> (String, Vec<SpannedToken>) {
	let whitespace_dist = rand::distributions::Uniform::from(0..=3);
	let start_whitespace = whitespace_dist.sample(&mut r());

	let raw = String::with_capacity(num_components * 7 + start_whitespace);
	let parsed = Vec::with_capacity(num_components);
	let current_location: Location = start_whitespace.try_into().unwrap();
	let mut generation_ctx = GenerationCtx {
		current_location,
		parsed,
		raw,
	};

	generation_ctx
		.raw
		.extend(std::iter::repeat(' ').take(start_whitespace));

	for token_type in super::token::Type::random_iter()
		.dedup()
		.take(num_components)
	{
		token_type.generate(&mut generation_ctx);
		let this_whitespace_len = whitespace_dist.sample(&mut r());
		generation_ctx
			.raw
			.extend(std::iter::repeat(' ').take(this_whitespace_len));
		generation_ctx.current_location += super::Location::try_from(this_whitespace_len).unwrap();
	}

	(generation_ctx.raw, generation_ctx.parsed)
}

#[test]
fn instrumented_random() {
	let components_dist = rand::distributions::Uniform::from(5..=10);
	for num_components in components_dist.sample_iter(r()).take(20_000) {
		let (raw_string, expected_output) = generate_random(num_components);
		let lexed = lex_to_vec(&raw_string);
		if lexed != expected_output {
			dbg!(raw_string);
			assert_eq!(lexed, expected_output);
		}
	}
}
