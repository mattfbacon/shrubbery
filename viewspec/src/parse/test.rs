use crate::lex::span::Span;
use crate::lex::token::Token;
use crate::parse::ast::{Ast, Node};
use crate::parse::parse;
use crate::parse::tag::Tag;

fn test_parse(tokens: impl IntoIterator<Item = Token>) -> Ast {
	parse(
		tokens
			.into_iter()
			.map(|token| token.with_span(Span::null())),
	)
	.expect("parsing failed")
}

#[test]
fn basic_name() {
	let ast = test_parse([Token::String {
		content: "abc".into(),
		bare: true,
	}]);
	assert_eq!(ast.root(), &Node::Tag(Tag::name("abc")));
}

#[test]
fn basic_category() {
	let ast = test_parse([
		Token::String {
			content: "abc".into(),
			bare: true,
		},
		Token::Colon,
	]);
	assert_eq!(ast.root(), &Node::Tag(Tag::category("abc")));
}

#[test]
fn basic_both() {
	let ast = test_parse([
		Token::String {
			content: "abc".into(),
			bare: true,
		},
		Token::Colon,
		Token::String {
			content: "def".into(),
			bare: true,
		},
	]);
	assert_eq!(ast.root(), &Node::Tag(Tag::both("abc", "def").unwrap()));
}

#[test]
fn simple_operator() {
	let ast = test_parse([
		Token::String {
			content: "abc".into(),
			bare: true,
		},
		Token::Colon,
		Token::And,
		Token::String {
			content: "def".into(),
			bare: true,
		},
		Token::Colon,
		Token::String {
			content: "ghi".into(),
			bare: true,
		},
	]);
	match ast.root() {
		Node::And(left, right) => {
			assert_eq!(ast.resolve_key(*left), &Node::Tag(Tag::category("abc")));
			assert_eq!(
				ast.resolve_key(*right),
				&Node::Tag(Tag::both("def", "ghi").unwrap())
			);
		}
		_ => panic!("expected And node"),
	};
}

#[test]
fn nesting() {
	let ast = test_parse([
		Token::OpenParen,
		Token::String {
			content: "abc".into(),
			bare: true,
		},
		Token::And,
		Token::String {
			content: "def".into(),
			bare: true,
		},
		Token::CloseParen,
		Token::Or,
		Token::Not,
		Token::String {
			content: "ghi".into(),
			bare: true,
		},
	]);
	match ast.root() {
		Node::Or(left, right) => {
			match ast.resolve_key(*left) {
				Node::And(left, right) => {
					assert_eq!(ast.resolve_key(*left), &Node::Tag(Tag::name("abc")));
					assert_eq!(ast.resolve_key(*right), &Node::Tag(Tag::name("def")));
				}
				_ => panic!("expected And node"),
			};
			match ast.resolve_key(*right) {
				Node::Not(child) => {
					assert_eq!(ast.resolve_key(*child), &Node::Tag(Tag::name("ghi")));
				}
				_ => panic!("expected Not node"),
			}
		}
		_ => panic!("expected Or node"),
	};
}

#[test]
fn associativity() {
	let ast = test_parse([
		Token::String {
			content: "abc".into(),
			bare: true,
		},
		Token::And,
		Token::String {
			content: "def".into(),
			bare: true,
		},
		Token::Or,
		Token::Not,
		Token::String {
			content: "ghi".into(),
			bare: true,
		},
	]);

	match ast.root() {
		Node::Or(left, right) => {
			match ast.resolve_key(*left) {
				Node::And(left, right) => {
					assert_eq!(ast.resolve_key(*left), &Node::Tag(Tag::name("abc")));
					assert_eq!(ast.resolve_key(*right), &Node::Tag(Tag::name("def")));
				}
				_ => panic!("expected And node"),
			};
			match ast.resolve_key(*right) {
				Node::Not(child) => {
					assert_eq!(ast.resolve_key(*child), &Node::Tag(Tag::name("ghi")));
				}
				_ => panic!("expected Not node"),
			}
		}
		_ => panic!("expected Or node"),
	}
}
