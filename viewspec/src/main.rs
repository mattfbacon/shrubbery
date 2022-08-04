use std::io::{self, BufRead, Write};

use viewspec::{lex, parse};

fn main() {
	eprintln!("Input a viewspec and we will lex and parse it.");
	loop {
		eprint!("> ");
		io::stderr().flush().unwrap();
		let mut input = String::new();
		if io::stdin().lock().read_line(&mut input).unwrap() == 0 {
			break;
		}
		let lexed: Vec<_> = lex::lex(input.trim().bytes()).collect();
		println!("Lexed: {lexed:#?}");
		let parsed = parse::parse(lexed.into_iter());
		match parsed {
			Ok(parsed) => {
				println!("Parsed: {parsed:#?}");
			}
			Err(err) => {
				println!("Parsing failed: {err:?}");
				continue;
			}
		}
	}
}
