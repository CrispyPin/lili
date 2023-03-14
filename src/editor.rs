use crossterm::{
	cursor::{self, MoveTo},
	event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
	queue,
	terminal::{self, Clear, ClearType},
};
use std::{
	env,
	fs::{self, File},
	io::{stdout, Write},
	ops::Range,
	path::PathBuf,
	vec,
};

use crate::config::Config;
use crate::util::{color_highlight, color_reset, read_line};

const TAB_SIZE: usize = 4;

pub struct Editor {
	text: String,
	lines: Vec<Line>,
	scroll: usize,
	cursor: Cursor,
	marker: Option<usize>,
	path: Option<PathBuf>,
	active: bool,
	unsaved_changes: bool,
	message: Option<String>,
}

#[derive(Debug)]
struct Cursor {
	line: usize,
	column: usize,
	// target_column: usize,
}

type Line = Range<usize>;

impl Editor {
	pub fn open_file(path: PathBuf) -> std::io::Result<Self> {
		let text = fs::read_to_string(&path)?;
		Ok(Editor {
			text,
			lines: Vec::new(),
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			marker: None,
			path: Some(path),
			active: false,
			unsaved_changes: false,
			message: None,
		})
	}

	pub fn new(path: Option<PathBuf>) -> Self {
		Editor {
			text: String::new(),
			lines: vec![0..0],
			scroll: 0,
			cursor: Cursor { line: 0, column: 0 },
			marker: None,
			path,
			active: false,
			unsaved_changes: true,
			message: None,
		}
	}

	pub fn title(&self) -> String {
		if let Some(path) = &self.path {
			if let Some(name) = path.file_name() {
				let decorator = if self.unsaved_changes { "*" } else { " " };
				return format!("{}{}", decorator, name.to_string_lossy());
			}
		}
		"*untitled".into()
	}

	pub fn is_unsaved(&self) -> bool {
		self.unsaved_changes
	}

	pub fn path(&self) -> Option<&PathBuf> {
		self.path.as_ref()
	}

	pub fn enter(&mut self, config: &mut Config) {
		self.active = true;
		self.find_lines();

		while self.active {
			self.draw(config);
			self.message = None;
			self.input(config);
		}
	}

	fn input(&mut self, config: &mut Config) {
		if let Ok(Event::Key(event)) = event::read() {
			if self.input_movement(&event) {
				return;
			}
			match event.modifiers {
				KeyModifiers::NONE => match event.code {
					KeyCode::Esc => self.active = false,
					KeyCode::Char(ch) => self.insert_char(ch),
					KeyCode::Enter => self.insert_char('\n'),
					KeyCode::Tab => self.insert_char('\t'),
					KeyCode::Backspace => self.backspace(),
					KeyCode::Delete => self.delete(),
					_ => (),
				},
				KeyModifiers::SHIFT => match event.code {
					KeyCode::Char(ch) => self.insert_char(ch.to_ascii_uppercase()),
					_ => (),
				},
				KeyModifiers::CONTROL => match event.code {
					KeyCode::Char('s') => self.save(),
					KeyCode::Char('c') => self.copy(config),
					KeyCode::Char('x') => self.cut(config),
					KeyCode::Char('v') => self.paste(config),
					KeyCode::Char('l') => config.line_numbers = !config.line_numbers,
					_ => (),
				},
				_ => (),
			}
		}
	}

	/// Cursor movement logic, returns true if cursor moved (so consider the event consumed in that case)
	fn input_movement(&mut self, event: &KeyEvent) -> bool {
		let prev_pos = self.char_index();
		let height = terminal::size().unwrap().1 as usize;
		match event.code {
			KeyCode::Left => self.move_left(),
			KeyCode::Right => self.move_right(),
			KeyCode::Up => self.move_up(1),
			KeyCode::Down => self.move_down(1),
			KeyCode::PageUp => self.move_up(height),
			KeyCode::PageDown => self.move_down(height),
			KeyCode::Home => self.move_home(),
			KeyCode::End => self.move_end(),
			_ => return false,
		}
		if event.modifiers.contains(KeyModifiers::SHIFT) {
			if self.marker.is_none() {
				self.marker = Some(prev_pos);
			}
		} else {
			self.marker = None;
		}
		true
	}

	fn draw(&self, config: &Config) {
		queue!(stdout(), Clear(ClearType::All)).unwrap();

		let max_rows = terminal::size().unwrap().1 as usize - 1;
		let end = (self.scroll + max_rows).min(self.lines.len());
		let visible_rows = self.scroll..end;

		let selection = self.selection().unwrap_or_default();

		let line_number_width = self.lines.len().to_string().len();

		for (line_index, line) in self.lines[visible_rows].iter().enumerate() {
			let text = &self.text[line.clone()];

			queue!(stdout(), MoveTo(0, line_index as u16)).unwrap();

			if config.line_numbers {
				let line_num = line_index + self.scroll + 1;
				print!("{line_num:line_number_width$} ");
			}

			let mut in_selection = false;
			for (i, char) in text.char_indices() {
				let char_i = line.start + i;
				if selection.contains(&char_i) {
					if !in_selection {
						color_highlight();
						in_selection = true;
					}
				} else if in_selection {
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
		self.status_line();
		let cursor_offset = if config.line_numbers {
			line_number_width + 1
		} else {
			0
		};
		queue!(
			stdout(),
			MoveTo(
				(self.physical_column() + cursor_offset) as u16,
				(self.cursor.line - self.scroll) as u16
			),
			cursor::Show,
			cursor::SetCursorStyle::BlinkingBar
		)
		.unwrap();
		stdout().flush().unwrap();
	}

	fn status_line(&self) {
		queue!(stdout(), MoveTo(0, terminal::size().unwrap().1)).unwrap();

		if let Some(message) = &self.message {
			print!("{message}");
		} else {
			print!(
				"[{}, {}] {}",
				self.cursor.line + 1,
				self.physical_column(),
				self.title(),
			);
		}
	}

	fn set_message(&mut self, text: String) {
		self.message = Some(text);
	}

	fn move_left(&mut self) {
		if self.cursor.column > 0 {
			self.cursor.column = self.prev_char_index() - self.current_line().start;
		} else if self.cursor.line > 0 {
			self.cursor.line -= 1;
			self.cursor.column = self.current_line().len();
		}
		self.scroll_to_cursor();
	}

	fn move_right(&mut self) {
		if self.cursor.column < self.current_line().len() {
			self.cursor.column = self.next_char_index() - self.current_line().start;
		} else if self.cursor.line < self.lines.len() - 1 {
			self.cursor.line += 1;
			self.cursor.column = 0;
		}
		self.scroll_to_cursor();
	}

	fn move_up(&mut self, lines: usize) {
		let physical_column = self.text
			[self.current_line().start..(self.current_line().start + self.cursor.column)]
			.chars()
			.count();
		self.cursor.line = self.cursor.line.saturating_sub(lines);
		self.cursor.column = physical_column.min(self.current_line().len());
		self.ensure_char_boundary();
		self.scroll_to_cursor();
	}

	fn move_down(&mut self, lines: usize) {
		let physical_column = self.text
			[self.current_line().start..(self.current_line().start + self.cursor.column)]
			.chars()
			.count();
		self.cursor.line = (self.cursor.line + lines).min(self.lines.len() - 1);
		self.cursor.column = physical_column.min(self.current_line().len());
		self.ensure_char_boundary();
		self.scroll_to_cursor();
	}

	fn scroll_to_cursor(&mut self) {
		let height = terminal::size().unwrap().1 as usize - 2;
		self.scroll = self
			.scroll
			.clamp(self.cursor.line.saturating_sub(height), self.cursor.line);
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

	fn copy(&mut self, config: &mut Config) {
		let range = self.selection_or_line();
		let mut text = self.text[range].to_owned();
		if self.marker.is_none() {
			text += "\n";
		}
		config.set_clipboard(text);
	}

	fn cut(&mut self, config: &mut Config) {
		let range = self.selection_or_line();
		let start = range.start;
		let mut end = range.end;
		let mut text = self.text[range].to_owned();
		if self.marker.is_none() {
			text += "\n";
			end += 1;
		}
		end = end.min(self.text.len());
		config.set_clipboard(text);
		self.text = self.text[..start].to_owned() + &self.text[end..];
		self.find_lines();
		self.move_to_byte(start);
		self.marker = None;
	}

	fn paste(&mut self, config: &Config) {
		self.unsaved_changes = true;
		let cursor = self.char_index();
		let new_text = config.clipboard();
		let end_pos = cursor + new_text.len();
		self.text.insert_str(cursor, new_text);
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

	/// where the cursor is rendered in the terminal output
	fn physical_column(&self) -> usize {
		let start = self.current_line().start;
		let end = self.char_index();
		let preceding_chars = self.text[start..end].chars().count();
		let preceding_tabs = self.text[start..end].chars().filter(|&c| c == '\t').count();
		preceding_chars + preceding_tabs * (TAB_SIZE - 1)
	}

	fn save(&mut self) {
		let mut filename_new = false;
		if self.path.is_none() {
			self.path = read_line("Enter path: ").map(|s| env::current_dir().unwrap().join(s));
			filename_new = true;
		}
		if let Some(path) = &self.path {
			match File::create(path) {
				Ok(mut file) => {
					self.set_message(format!("Saved file as '{}'", path.display()));
					file.write_all(self.text.as_bytes()).unwrap();
					self.unsaved_changes = false;
				}
				Err(e) => {
					self.set_message(format!("Could not save file as '{}': {e}", path.display()));
					if filename_new {
						self.path = None;
					}
				}
			}
		}
	}
}
