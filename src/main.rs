use std::{
	env,
	io::{stdin, stdout, Stdout, Write},
	process::exit,
};
use termion::{
	clear, color,
	cursor::{self, Goto},
	event::{Event, Key},
	input::TermRead,
	raw::{IntoRawMode, RawTerminal},
};

mod clipboard;
mod editor;
mod util;
use clipboard::Clipboard;
use editor::Editor;

fn main() {
	Navigator::new().run();
}

struct Navigator {
	editors: Vec<Editor>,
	selected: Option<usize>,
	clipboard: Clipboard,
	_term: RawTerminal<Stdout>,
}

impl Navigator {
	fn new() -> Self {
		let term = stdout().into_raw_mode().unwrap();
		let clipboard = Clipboard::new();
		let mut editors: Vec<Editor> = env::args()
			.skip(1)
			.map(|path| Editor::new(clipboard.clone(), path))
			.collect();
		if editors.is_empty() {
			editors.push(Editor::new_empty(clipboard.clone()));
		}
		Self {
			editors,
			selected: Some(0),
			clipboard,
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
			"{}{}{}Open editors: {}",
			clear::All,
			cursor::Hide,
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
					Key::Char('q') => self.quit(),
					Key::Char('\n') => self.open_selected(),
					Key::Ctrl('n') => self.new_editor(),
					Key::Up => self.nav_up(),
					Key::Down => self.nav_down(),
					_ => (),
				}
			}
		}
	}

	fn nav_up(&mut self) {
		if self.selected > Some(0) {
			self.selected = Some(self.selected.unwrap() - 1);
		}
	}

	fn nav_down(&mut self) {
		if let Some(index) = self.selected.as_mut() {
			if *index < self.editors.len() - 1 {
				*index += 1;
			}
		}
	}

	fn open_selected(&mut self) {
		if let Some(index) = self.selected {
			self.editors[index].open();
		}
	}

	fn new_editor(&mut self) {
		self.selected = Some(self.editors.len());
		self.editors.push(Editor::new_empty(self.clipboard.clone()));
		self.open_selected();
	}

	fn quit(&self) {
		print!("{}{}", clear::All, cursor::Show);
		exit(0);
	}
}
