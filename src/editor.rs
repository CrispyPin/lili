use crossterm::{
	cursor::{self, MoveTo},
	event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
	queue,
	style::{Color, Colors, ResetColor, SetColors},
	terminal::{self, Clear, ClearType},
};
use std::{
	fs::{self, File},
	io::{stdout, Write},
	ops::Range,
	path::PathBuf,
	vec,
};

use crate::clipboard::Clipboard;
use crate::util::read_line;

const TAB_SIZE: usize = 4;

pub struct Editor {
	text: String,
	lines: Vec<Line>,
	scroll: usize,
	cursor: Cursor,
	marker: Option<usize>,
	clipboard: Clipboard,
	path: Option<PathBuf>,
	active: bool,
	unsaved_changes: bool,
}

#[derive(Debug)]
struct Cursor {
	line: usize,
	column: usize,
	// target_column: usize,
}

type Line = Range<usize>;

impl Editor {
	pub fn open_file(clipboard: Clipboard, path: PathBuf) -> Option<Self> {
		let text = fs::read_to_string(&path).ok()?;
		Some(Editor {
			text,
			lines: Vec::new(),
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			marker: None,
			clipboard,
			path: Some(path),
			active: false,
			unsaved_changes: false,
		})
	}

	pub fn new_empty(clipboard: Clipboard) -> Self {
		Editor {
			text: String::new(),
			lines: vec![0..0],
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			marker: None,
			clipboard,
			path: None,
			active: false,
			unsaved_changes: true,
		}
	}

	pub fn new_named(clipboard: Clipboard, path: PathBuf) -> Self {
		Editor {
			text: String::new(),
			lines: vec![0..0],
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			marker: None,
			clipboard,
			path: Some(path),
			active: false,
			unsaved_changes: true,
		}
	}

	pub fn name(&self) -> String {
		if let Some(path) = &self.path {
			if let Some(name) = path.file_name() {
				return name.to_string_lossy().to_string();
			}
		}
		"untitled".into()
	}

	pub fn has_unsaved_changes(&self) -> bool {
		self.unsaved_changes
	}

	pub fn enter(&mut self) {
		self.active = true;
		self.find_lines();

		while self.active {
			self.draw();
			self.input();
		}
	}

	fn input(&mut self) {
		if let Ok(Event::Key(event)) = event::read() {
			if self.input_movement(&event) {
				return;
			}
			match event.modifiers {
				KeyModifiers::NONE => match event.code {
					KeyCode::Esc => self.active = false,
					KeyCode::Char(ch) => self.insert_char(ch),
					KeyCode::Enter => self.insert_char('\n'),
					KeyCode::Backspace => self.backspace(),
					KeyCode::Delete => self.delete(),
					_ => (),
				},
				KeyModifiers::CONTROL => match event.code {
					KeyCode::Char('s') => self.save(),
					KeyCode::Char('c') => self.copy(),
					KeyCode::Char('x') => self.cut(),
					KeyCode::Char('v') => self.paste(),
					_ => (),
				},
				_ => (),
			}
		}
	}

	/// Cursor movement logic, returns true if cursor moved (so consider the event consumed in that case)
	fn input_movement(&mut self, event: &KeyEvent) -> bool {
		if let KeyCode::Left
		| KeyCode::Right
		| KeyCode::Up
		| KeyCode::Down
		| KeyCode::Home
		| KeyCode::End = event.code
		{
			if event.modifiers.contains(KeyModifiers::SHIFT) {
				self.set_marker();
			} else {
				self.marker = None;
			}
			match event.code {
				KeyCode::Left => self.move_left(),
				KeyCode::Right => self.move_right(),
				KeyCode::Up => self.move_up(),
				KeyCode::Down => self.move_down(),
				KeyCode::Home => self.move_home(),
				KeyCode::End => self.move_end(),
				_ => (),
			}
			true
		} else {
			false
		}
	}

	fn draw(&self) {
		queue!(stdout(), Clear(ClearType::All)).unwrap();

		let max_rows = terminal::size().unwrap().1 as usize - 1;
		let end = (self.scroll + max_rows).min(self.lines.len());
		let visible_rows = self.scroll..end;

		let cursor = self.char_index();
		let marker = self.marker.unwrap_or(0);
		let selection = (marker.min(cursor))..(marker.max(cursor));

		for (line_index, line) in self.lines[visible_rows].iter().enumerate() {
			let text = &self.text[line.clone()];

			queue!(stdout(), MoveTo(0, line_index as u16)).unwrap();

			if self.marker.is_none() {
				print!("{}", text.replace('\t', &" ".repeat(TAB_SIZE)));
			} else {
				let mut in_selection = false;
				for (i, char) in text.char_indices() {
					let char_i = line.start + i;
					if char_i >= selection.start && char_i <= selection.end && !in_selection {
						color_selection();
						in_selection = true;
					} else if char_i > selection.end && in_selection {
						color_reset();
						in_selection = false;
					}
					if char == '\t' {
						print!("{:1$}", " ", TAB_SIZE);
					} else {
						print!("{char}");
					}
				}
				color_reset();
			}
		}
		self.status_line();
		queue!(
			stdout(),
			MoveTo(
				self.physical_column() as u16,
				(self.cursor.line - self.scroll) as u16
			),
			cursor::Show
		)
		.unwrap();
		stdout().flush().unwrap();
	}

	fn status_line(&self) {
		queue!(stdout(), MoveTo(0, terminal::size().unwrap().1)).unwrap();
		print!(
			"({},{}) {}",
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
			if self.cursor.line > (self.scroll + terminal::size().unwrap().1 as usize - 2) {
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

	fn move_to_byte(&mut self, pos: usize) {
		for (line_index, line) in self.lines.iter().enumerate() {
			if (line.start..=line.end).contains(&pos) {
				self.cursor.line = line_index;
				self.cursor.column = pos - line.start;
			}
		}
	}

	fn set_marker(&mut self) {
		if self.marker.is_none() {
			self.marker = Some(self.char_index());
		}
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
		self.unsaved_changes = true;
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

	fn selection(&self) -> Option<Range<usize>> {
		let cursor = self.char_index();
		self.marker
			.map(|marker| marker.min(cursor)..(marker.max(cursor)))
	}

	fn selection_or_line(&self) -> Range<usize> {
		self.selection().unwrap_or(self.current_line().clone())
	}

	fn copy(&mut self) {
		let range = self.selection_or_line();
		let mut text = self.text[range].to_owned();
		if self.marker.is_none() {
			text += "\n";
		}
		self.clipboard.set(text);
	}

	fn cut(&mut self) {
		let range = self.selection_or_line();
		let start = range.start;
		let mut end = range.end;
		let mut text = self.text[range].to_owned();
		if self.marker.is_none() {
			text += "\n";
			end += 1;
		}
		end = end.min(self.text.len());
		self.clipboard.set(text);
		self.text = self.text[..start].to_owned() + &self.text[end..];
		self.find_lines();
		self.move_to_byte(start);
		self.marker = None;
	}

	fn paste(&mut self) {
		self.unsaved_changes = true;
		let cursor = self.char_index();
		let new_text = self.clipboard.get();
		let end_pos = cursor + new_text.len();
		self.text.insert_str(cursor, &new_text);
		self.find_lines();
		self.move_to_byte(end_pos);
		self.marker = None;
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
			self.path = read_line("Enter path: ").map(PathBuf::from);
			if self.path.is_none() {
				return;
			}
		}
		let mut file = File::create(self.path.as_ref().unwrap()).unwrap();
		file.write_all(self.text.as_bytes()).unwrap();
		self.unsaved_changes = false;
	}
}

fn color_selection() {
	queue!(stdout(), SetColors(Colors::new(Color::Black, Color::White))).unwrap();
}

fn color_reset() {
	queue!(stdout(), ResetColor).unwrap();
}
