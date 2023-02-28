use std::{
	fs::{self, File},
	io::{stdin, stdout, Write},
	ops::Range,
	process::exit,
};
use termion::{
	clear, cursor,
	event::{Event, Key},
	input::TermRead,
	terminal_size,
};

use crate::util::read_line;

const TAB_SIZE: usize = 4;

pub struct Editor {
	text: String,
	lines: Vec<Line>,
	scroll: usize,
	cursor: Cursor,
	path: Option<String>,
	active: bool,
}

#[derive(Debug)]
struct Cursor {
	line: usize,
	column: usize,
	// target_column: usize,
}

type Line = Range<usize>;

impl Editor {
	pub fn new(path: Option<String>) -> Self {
		let text = path
			.as_ref()
			.map(|path| {
				fs::read_to_string(path).unwrap_or_else(|err| {
					println!("Error: {err}");
					exit(1);
				})
			})
			.unwrap_or_default();

		let mut this = Editor {
			text,
			lines: Vec::new(),
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			path,
			active: false,
		};
		this.find_lines();
		this
	}

	pub fn name(&self) -> &str {
		self.path.as_ref().map_or("untitled", |s| s)
	}

	pub fn open(&mut self) {
		self.active = true;

		while self.active {
			self.draw();
			self.input();
		}
	}

	fn input(&mut self) {
		for event in stdin().events().take(1).flatten() {
			if let Event::Key(key) = event {
				match key {
					Key::Esc => self.active = false,
					Key::Char(char) => self.insert_char(char),
					Key::Backspace => self.backspace(),
					Key::Delete => self.delete(),
					Key::Left => self.move_left(),
					Key::Right => self.move_right(),
					Key::Up => self.move_up(),
					Key::Down => self.move_down(),
					Key::Home => self.move_home(),
					Key::End => self.move_end(),
					Key::Ctrl('s') => self.save(),
					_ => (),
				}
			}
		}
	}

	fn draw(&self) {
		print!("{}", clear::All);

		let max_rows = terminal_size().unwrap().1 as usize - 1;
		let end = (self.scroll + max_rows).min(self.lines.len());
		let visible_rows = self.scroll..end;

		for (line_index, line) in self.lines[visible_rows].iter().enumerate() {
			let text = &self.text[line.clone()];
			print!(
				"{}{}",
				cursor::Goto(1, line_index as u16 + 1),
				text.replace('\t', &" ".repeat(TAB_SIZE))
			);
		}
		self.status_line();
		print!(
			"{}",
			cursor::Goto(
				self.physical_column() as u16 + 1,
				(self.cursor.line - self.scroll) as u16 + 1
			)
		);
		stdout().flush().unwrap();
	}

	fn status_line(&self) {
		print!(
			"{}({}, {}) {}",
			cursor::Goto(1, terminal_size().unwrap().1),
			self.cursor.line,
			self.physical_column(),
			self.name(),
		);
	}

	fn move_left(&mut self) {
		if self.cursor.column > 0 {
			self.cursor.column = self.prev_char_index() - self.current_line().start;
		} else if self.cursor.line > 0 {
			self.cursor.line -= 1;
			self.cursor.column = self.current_line().len();
		}
	}

	fn move_right(&mut self) {
		if self.cursor.column < self.current_line().len() {
			self.cursor.column = self.next_char_index() - self.current_line().start;
		} else if self.cursor.line < self.lines.len() - 1 {
			self.cursor.line += 1;
			self.cursor.column = 0;
		}
	}

	fn move_up(&mut self) {
		if self.cursor.line > 0 {
			let physical_column = self.text
				[self.current_line().start..(self.current_line().start + self.cursor.column)]
				.chars()
				.count();
			self.cursor.line -= 1;
			self.cursor.column = physical_column.min(self.current_line().len());
			self.ensure_char_boundary();
			if self.cursor.line < self.scroll {
				self.scroll -= 1;
			}
		}
	}

	fn move_down(&mut self) {
		if self.cursor.line < self.lines.len() - 1 {
			let physical_column = self.text
				[self.current_line().start..(self.current_line().start + self.cursor.column)]
				.chars()
				.count();
			self.cursor.line += 1;
			self.cursor.column = physical_column.min(self.current_line().len());
			self.ensure_char_boundary();
			if self.cursor.line > (self.scroll + terminal_size().unwrap().1 as usize - 2) {
				self.scroll += 1;
			}
		}
	}

	fn move_home(&mut self) {
		self.cursor.column = 0;
	}

	fn move_end(&mut self) {
		self.cursor.column = self.current_line().len();
		self.ensure_char_boundary();
	}

	/// Moves cursor left until it is on a character (in case it was in the middle of a multi-byte character)
	fn ensure_char_boundary(&mut self) {
		while !self
			.text
			.is_char_boundary(self.current_line().start + self.cursor.column)
		{
			self.cursor.column -= 1;
		}
	}

	fn current_line(&self) -> &Line {
		self.lines.get(self.cursor.line).unwrap()
	}

	fn find_lines(&mut self) {
		self.lines.clear();
		let mut this_line = 0..0;
		for (index, char) in self.text.char_indices() {
			if char == '\n' {
				this_line.end = index;
				self.lines.push(this_line.clone());
				this_line.start = index + 1;
			}
		}
		this_line.end = self.text.len();
		self.lines.push(this_line);
	}

	fn insert_char(&mut self, ch: char) {
		// eprintln!("inserting {ch} at {}", self.index());
		self.text.insert(self.char_index(), ch);
		self.find_lines();
		self.move_right();
	}

	fn backspace(&mut self) {
		if self.char_index() > 0 {
			self.move_left();
			self.text.remove(self.char_index());
			self.find_lines();
		}
	}

	fn delete(&mut self) {
		if self.char_index() < self.text.len() {
			self.text.remove(self.char_index());
			self.find_lines();
		}
	}

	/// Byte position of current character. May be text.len if cursor is at the end of the file
	fn char_index(&self) -> usize {
		self.current_line().start + self.cursor.column
	}

	/// Byte position of next character.
	/// Returns text.len if cursor is on the last character
	fn next_char_index(&self) -> usize {
		self.text[self.char_index()..]
			.char_indices()
			.nth(1)
			.map_or(self.text.len(), |(byte, _char)| byte + self.char_index())
	}

	/// Byte position of preceding character.
	/// Panics if cursor is at index 0
	fn prev_char_index(&self) -> usize {
		self.text[..self.char_index()]
			.char_indices()
			.last()
			.map(|(byte, _char)| byte)
			.unwrap()
	}

	fn physical_column(&self) -> usize {
		let start = self.current_line().start;
		let end = self.char_index();
		let preceding_chars = self.text[start..end].chars().count();
		let preceding_tabs = self.text[start..end].chars().filter(|&c| c == '\t').count();
		preceding_chars + preceding_tabs * (TAB_SIZE - 1)
	}

	fn save(&mut self) {
		if self.path.is_none() {
			self.path = read_line("Enter path: ");
			if self.path.is_none() {
				return;
			}
		}
		let mut file = File::create(self.path.as_ref().unwrap()).unwrap();
		file.write_all(self.text.as_bytes()).unwrap();
	}
}
