use std::{
	io::{stdin, stdout, Write},
	ops::Range,
};
use termion::{
	cursor,
	event::{Event, Key},
	input::TermRead,
	terminal_size,
};

pub fn read_line(prompt: &str) -> Option<String> {
	let mut response = String::new();
	let size = terminal_size().unwrap();
	let start_pos = cursor::Goto(1, size.1);
	let width = size.0 as usize;

	print!("{start_pos}{prompt}{response}",);
	stdout().flush().unwrap();

	for event in stdin().events() {
		if let Ok(Event::Key(key)) = event {
			match key {
				Key::Char('\n') => break,
				Key::Char(ch) => response.push(ch),
				Key::Backspace => {
					response.pop();
					print!("{start_pos}{:width$}", " ");
				}
				Key::Esc => return None,
				_ => (),
			}
		}
		print!("{start_pos}{prompt}{response}",);
		stdout().flush().unwrap();
	}
	Some(response.trim().into())
}

pub trait RangeConverter {
	fn as_inclusive(&self) -> Range<usize>;
}

impl RangeConverter for Range<usize> {
	fn as_inclusive(&self) -> Range<usize> {
		self.start..(self.end + 1)
	}
}
