use nom::branch::alt;
use nom::bytes::complete::take_until;
use nom::character::complete::{char as one_char, multispace0, none_of, one_of};
use nom::combinator::{all_consuming, map, opt, recognize};
use nom::error::VerboseError;
use nom::multi::{many0, many0_count};
use nom::sequence::{delimited, preceded, terminated, tuple};
use nom::Parser;

use super::{Tag, ViewSpec};

type IResult<'a, O = ViewSpec> = nom::IResult<&'a str, O, VerboseError<&'a str>>;

pub type Error = VerboseError<String>;

static OPERATOR_CHARS: &[char] = &['&', '|', '!', '(', ')'];
static OPERATOR_CHARS_AND_COLON: &[char] = &['&', '|', '!', '(', ')', ':'];

pub fn parse(input: &str) -> Result<ViewSpec, Error> {
	match all_consuming::<_, _, VerboseError<&str>, _>(pad_space(binary_operators))(input) {
		Ok(("", viewspec)) => Ok(viewspec),
		Ok((_remaining, _viewspec)) => unreachable!(), // all_consuming used
		Err(nom::Err::Error(err) | nom::Err::Failure(err)) => Err(VerboseError {
			errors: err
				.errors
				.into_iter()
				.map(|(input, kind)| (input.into(), kind))
				.collect(),
		}),
		Err(nom::Err::Incomplete(..)) => unreachable!(), // no streaming parsers used
	}
}

fn if_then_else<'a, F, G, H, U, O>(
	mut if_: F,
	mut then: G,
	mut else_: H,
) -> impl FnMut(&'a str) -> IResult<'a, O>
where
	F: Parser<&'a str, U, VerboseError<&'a str>>,
	G: Parser<&'a str, O, VerboseError<&'a str>>,
	H: Parser<&'a str, O, VerboseError<&'a str>>,
{
	move |input| {
		if let Ok((input, _)) = if_.parse(input) {
			then.parse(input)
		} else {
			else_.parse(input)
		}
	}
}

fn pad_space<'a, O, F>(inner: F) -> impl FnMut(&'a str) -> IResult<'a, O>
where
	F: Parser<&'a str, O, VerboseError<&'a str>>,
{
	delimited(multispace0, inner, multispace0)
}

fn token<'a>(token: char) -> impl FnMut(&'a str) -> IResult<'a, char> {
	pad_space(one_char(token))
}

fn tag(input: &str) -> IResult<'_> {
	let quote = one_char('"');
	let unquoted_tag = |first| {
		map(
			recognize(many0_count(none_of(if first {
				OPERATOR_CHARS_AND_COLON
			} else {
				OPERATOR_CHARS
			}))),
			|unquoted: &str| unquoted.trim(),
		)
	};
	let tag_part = |first| {
		if_then_else(
			&quote,
			terminated(take_until("\""), &quote),
			unquoted_tag(first),
		)
	};
	let (input, first_str) = tag_part(true)(input)?;
	let (input, second_str) = opt(preceded(token(':'), tag_part(false)))(input)?;
	let ret = match second_str {
		Some("") => ViewSpec::Tag(Tag::Category(first_str.into())),
		Some(tag_name) => ViewSpec::Tag(Tag::Both {
			category: first_str.into(),
			tag: tag_name.into(),
		}),
		None => ViewSpec::Tag(Tag::Tag(first_str.into())),
	};
	Ok((input, ret))
}

fn inner_expression(input: &str) -> IResult<'_> {
	alt((delimited(token('('), binary_operators, token(')')), tag))(input)
}

fn unary_operators(input: &str) -> IResult<'_> {
	let (input, mut num_nots) = many0_count(token('!'))(input)?;
	let (input, mut inner_expr) = inner_expression(input)?;
	if let ViewSpec::Not(not_inner) = inner_expr {
		inner_expr = *not_inner;
		num_nots += 1;
	}
	let expr = if num_nots % 2 == 0 {
		inner_expr
	} else {
		ViewSpec::Not(Box::new(inner_expr))
	};
	Ok((input, expr))
}

#[derive(Clone, Copy)]
enum BinaryOperator {
	And,
	Or,
}

fn binary_operator(input: &str) -> IResult<'_, BinaryOperator> {
	map(pad_space(one_of("&|")), |token| match token {
		'&' => BinaryOperator::And,
		'|' => BinaryOperator::Or,
		_ => unreachable!(),
	})(input)
}

fn binary_operators(input: &str) -> IResult<'_> {
	// have to collect into Vec to do left-associative operations
	let (input, exprs) = many0(tuple((unary_operators, binary_operator)))(input)?;
	let (input, last_term) = unary_operators(input)?;

	let expr = exprs
		.into_iter()
		.rfold(
			last_term,
			|right_side, (left_side, binary_operator)| match binary_operator {
				BinaryOperator::And => ViewSpec::And(Box::new(left_side), Box::new(right_side)),
				BinaryOperator::Or => ViewSpec::Or(Box::new(left_side), Box::new(right_side)),
			},
		);
	Ok((input, expr))
}

#[cfg(test)]
mod test {
	use super::{parse, Tag, ViewSpec};

	fn make_tag(category: Option<&str>, name: &str) -> ViewSpec {
		ViewSpec::Tag(match category {
			Some(category) => Tag::Both {
				category: category.into(),
				tag: name.into(),
			},
			None => Tag::Tag(name.into()),
		})
	}

	#[test]
	fn tag() {
		// name unquoted, category None
		assert_eq!(parse("abc").unwrap(), make_tag(None, "abc"));
		// name unquoted, category Some(unquoted)
		assert_eq!(parse("abc:def").unwrap(), make_tag(Some("abc"), "def"));
		// name unquoted, category Some(unquoted)
		assert_eq!(
			parse("abc:def:ghi").unwrap(),
			make_tag(Some("abc"), "def:ghi")
		);
		// name quoted, category None
		assert_eq!(parse(r#""abc""#).unwrap(), make_tag(None, "abc"));
		// name quoted, category Some(quoted)
		assert_eq!(
			parse(r#""cat":"abc""#).unwrap(),
			make_tag(Some("cat"), "abc")
		);
		// name quoted, category Some(unquoted)
		assert_eq!(parse(r#"cat:"abc""#).unwrap(), make_tag(Some("cat"), "abc"));
		// name unquoted, category Some(quoted with outer spaces)
		assert_eq!(
			parse(r#"" cat ":abc"#).unwrap(),
			make_tag(Some(" cat "), "abc")
		);
		// name unquoted, category Some(quoted with spaces)
		assert_eq!(
			parse(r#""cat cat cat":abc"#).unwrap(),
			make_tag(Some("cat cat cat"), "abc")
		);
		// name unquoted with spaces, category Some(unquoted with spaces)
		assert_eq!(
			parse(r#"cat cat cat:abc def ghi"#).unwrap(),
			make_tag(Some("cat cat cat"), "abc def ghi")
		);
		// spaces around colon
		assert_eq!(parse(r#"abc : def"#).unwrap(), make_tag(Some("abc"), "def"));
		// operators inside quotes
		assert_eq!(
			parse(r#""abc & def | ghi ! jkl""#).unwrap(),
			make_tag(None, "abc & def | ghi ! jkl")
		);
	}

	#[test]
	fn basic() {
		assert_eq!(
			parse("abc & def").unwrap(),
			ViewSpec::And(
				Box::new(make_tag(None, "abc")),
				Box::new(make_tag(None, "def"))
			)
		);
	}

	#[test]
	fn nots() {
		assert_eq!(
			parse("!abc & !def").unwrap(),
			ViewSpec::And(
				Box::new(ViewSpec::Not(Box::new(make_tag(None, "abc")))),
				Box::new(ViewSpec::Not(Box::new(make_tag(None, "def"))))
			)
		);
	}

	#[test]
	fn parens() {
		assert_eq!(
			parse("!(abc & def) | ghi").unwrap(),
			ViewSpec::Or(
				Box::new(ViewSpec::Not(Box::new(ViewSpec::And(
					Box::new(make_tag(None, "abc")),
					Box::new(make_tag(None, "def"))
				)))),
				Box::new(make_tag(None, "ghi"))
			)
		);
	}

	#[test]
	fn mix_styles() {
		let expected = ViewSpec::And(
			Box::new(make_tag(Some("abc"), "def")),
			Box::new(make_tag(None, "ghi")),
		);
		assert_eq!(parse("abc:def & ghi").unwrap(), expected);
		assert_eq!(parse(r#""abc":def & "ghi""#).unwrap(), expected);
		assert_eq!(parse(r#"abc:"def" & "ghi""#).unwrap(), expected);
	}

	#[test]
	fn just_category() {
		assert_eq!(
			parse("abc:").unwrap(),
			ViewSpec::Tag(Tag::Category("abc".into()))
		);
	}
}
