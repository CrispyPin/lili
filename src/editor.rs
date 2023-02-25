use std::{
	io::{stdin, stdout, Write},
	ops::Range,
};
use termion::{
	clear, cursor,
	event::{Event, Key},
	input::TermRead,
	raw::IntoRawMode,
};

pub struct Editor {
	text: String,
	lines: Vec<Line>,
	cursor: Cursor,
	quit: bool,
}

struct Cursor {
	line: usize,
	column: usize,
}

type Line = Range<usize>;

impl Editor {
	pub fn new() -> Self {
		Editor {
			// text: String::new(),
			text: include_str!("editor.rs").into(),
			lines: Vec::new(),
			cursor: Cursor { line: 0, column: 0 },
			quit: false,
		}
	}

	pub fn run(mut self) {
		println!("{}", clear::All);
		stdout().flush().unwrap();
		let _t = stdout().into_raw_mode().unwrap();

		while !self.quit {
			self.find_lines();
			self.draw();
			self.input();
		}
		println!("{}", clear::All);
		stdout().flush().unwrap();
		// self.term.suspend_raw_mode().unwrap();
	}

	fn input(&mut self) {
		for event in stdin().events().take(1).flatten() {
			// dbg!(&event);
			if let Event::Key(key) = event {
				match key {
					Key::Esc => self.quit = true,
					Key::Char(char) => self.insert_char(char),
					Key::Backspace => self.backspace(),
					Key::Delete => self.delete(),
					Key::Left => self.move_left(),
					Key::Right => self.move_right(),
					Key::Up => self.move_up(),
					Key::Down => self.move_down(),
					_ => (),
				}
			}
		}
	}

	fn move_left(&mut self) {
		if self.cursor.column > 0 {
			self.cursor.column -= 1;
		} else if self.cursor.line > 0 {
			self.cursor.line -= 1;
			self.cursor.column = self.current_line().len();
		}
	}

	fn move_right(&mut self) {
		if self.cursor.column < self.current_line().len() {
			self.cursor.column += 1;
		} else if self.cursor.line < self.lines.len() {
			self.cursor.line += 1;
			self.cursor.column = 0;
		}
	}

	fn move_up(&mut self) {
		if self.cursor.line > 0 {
			self.cursor.line -= 1;
			self.cursor.column = self.cursor.column.min(self.current_line().len());
		}
	}

	fn move_down(&mut self) {
		if self.cursor.line < self.lines.len() {
			self.cursor.line += 1;
			self.cursor.column = self.cursor.column.min(self.current_line().len());
		}
	}

	fn current_line(&self) -> &Line {
		self.lines.get(self.cursor.line).unwrap_or(&(0..0))
	}

	fn find_lines(&mut self) {
		self.lines.clear();
		let mut this_line = 0..0;
		for (index, char) in self.text.chars().enumerate() {
			if char == '\n' {
				this_line.end = index;
				self.lines.push(this_line.clone());
				this_line.start = index + 1;
			}
		}
	}

	fn draw(&self) {
		print!("{}", clear::All);

		for (row, line) in self.lines.iter().enumerate() {
			let text = &self.text[line.clone()];
			print!(
				"{}{}",
				cursor::Goto(1, row as u16 + 1),
				text.replace('\t', "    ")
			);
		}
		print!(
			"{}",
			cursor::Goto(self.cursor.column as u16 + 1, self.cursor.line as u16 + 1)
		);
		stdout().flush().unwrap();
	}

	fn insert_char(&mut self, ch: char) {
		self.text.insert(self.index(), ch);
		self.find_lines();
		self.move_right();
	}

	fn backspace(&mut self) {
		self.text.remove(self.index() - 1);
		self.find_lines();
		self.move_left();
	}

	fn delete(&mut self) {
		self.text.remove(self.index());
		self.find_lines();
	}

	fn index(&self) -> usize {
		self.current_line().start + self.cursor.column
	}
}
