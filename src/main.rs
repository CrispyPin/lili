use crossterm::{
	cursor::{self, MoveTo},
	event::{self, Event, KeyCode, KeyModifiers},
	execute, queue,
	style::{Color, Colors, ResetColor, SetColors},
	terminal::{
		disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
		LeaveAlternateScreen,
	},
};
use std::{
	env,
	io::{stdout, Write},
	process::exit,
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
}

impl Navigator {
	fn new() -> Self {
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
		}
	}

	fn run(mut self) {
		execute!(stdout(), EnterAlternateScreen, Clear(ClearType::All)).unwrap();
		enable_raw_mode().unwrap();

		loop {
			self.draw();
			self.input();
		}
	}

	fn draw(&self) {
		queue!(stdout(), Clear(ClearType::All), cursor::Hide, MoveTo(0, 0)).unwrap();
		print!("Open editors: {}", self.editors.len());

		for (index, editor) in self.editors.iter().enumerate() {
			if Some(index) == self.selected {
				queue!(stdout(), SetColors(Colors::new(Color::Black, Color::White))).unwrap();
			}
			queue!(stdout(), MoveTo(1, index as u16 + 1)).unwrap();
			print!(
				"{}{}",
				editor.has_unsaved_changes().then_some("*").unwrap_or(" "),
				editor.name()
			);
			queue!(stdout(), ResetColor).unwrap();
		}

		stdout().flush().unwrap();
	}

	fn input(&mut self) {
		if let Ok(Event::Key(event)) = event::read() {
			match event.code {
				KeyCode::Char('q') => self.quit(),
				KeyCode::Up => self.nav_up(),
				KeyCode::Down => self.nav_down(),
				KeyCode::Enter => self.open_selected(),
				KeyCode::Char('n') => {
					if event.modifiers == KeyModifiers::CONTROL {
						self.new_editor();
					}
				}
				_ => (),
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
		disable_raw_mode().unwrap();
		execute!(stdout(), LeaveAlternateScreen, cursor::Show).unwrap();
		exit(0);
	}
}
