use std::{
	env,
	io::{stdin, stdout, Stdout, Write},
	process::exit,
};
use termion::{
	clear, color,
	cursor::Goto,
	event::{Event, Key},
	input::TermRead,
	raw::{IntoRawMode, RawTerminal},
};

mod editor;
mod util;
use editor::Editor;

fn main() {
	Navigator::new(env::args().nth(1)).run();
}

struct Navigator {
	editors: Vec<Editor>,
	selected: Option<usize>,
	path: String,
	_term: RawTerminal<Stdout>,
}

impl Navigator {
	fn new(immediate_file: Option<String>) -> Self {
		let term = stdout().into_raw_mode().unwrap();
		let editors = vec![Editor::new(immediate_file)];
		Self {
			editors,
			selected: Some(0),
			path: String::new(), // TODO
			_term: term,
		}
	}

	fn run(mut self) {
		print!("{}", clear::All);
		stdout().flush().unwrap();

		loop {
			self.draw();
			self.input();
		}
	}

	fn draw(&self) {
		print!(
			"{}{}Open editors: {}",
			clear::All,
			Goto(1, 1),
			self.editors.len()
		);

		for (index, editor) in self.editors.iter().enumerate() {
			if Some(index) == self.selected {
				print!("{}{}", color::Fg(color::Black), color::Bg(color::White));
			}
			print!("{}{}", Goto(2, index as u16 + 2), editor.name());
			print!("{}{}", color::Fg(color::Reset), color::Bg(color::Reset));
		}

		stdout().flush().unwrap();
	}

	fn input(&mut self) {
		for event in stdin().events().take(1).flatten() {
			if let Event::Key(key) = event {
				match key {
					Key::Esc => self.quit(),
					Key::Char('\n') => self.open_selected(),
					_ => (),
				}
			}
		}
	}

	fn open_selected(&mut self) {
		if let Some(index) = self.selected {
			self.editors[index].open();
		}
	}

	fn quit(&self) {
		print!("{}", clear::All);
		exit(0);
	}
}
