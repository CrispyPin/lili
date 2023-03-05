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
	env, fs,
	io::{stdout, Write},
	path::PathBuf,
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
	clipboard: Clipboard,
	editors: Vec<Editor>,
	files: Vec<PathBuf>,
	selected: usize,
	path: PathBuf,
	immediate_open: bool,
}

impl Navigator {
	fn new() -> Self {
		let clipboard = Clipboard::new();
		let mut editors = Vec::new();

		let args: Vec<String> = env::args().skip(1).collect();

		let mut path = env::current_dir().unwrap();

		for arg in args.iter().map(PathBuf::from) {
			if arg.is_dir() {
				path = arg.canonicalize().unwrap();
				break;
			} else if arg.is_file() {
				if let Some(editor) = Editor::open_file(clipboard.clone(), arg) {
					editors.push(editor);
				}
			} else {
				editors.push(Editor::new_named(clipboard.clone(), arg))
			}
		}
		if args.is_empty() {
			editors.push(Editor::new_empty(clipboard.clone()));
		}
		let immediate_open = editors.len() == 1;
		Self {
			clipboard,
			editors,
			selected: 0,
			files: Vec::new(),
			path,
			immediate_open,
		}
	}

	fn run(mut self) {
		execute!(stdout(), EnterAlternateScreen, Clear(ClearType::All)).unwrap();
		enable_raw_mode().unwrap();

		if self.immediate_open {
			self.enter();
		}

		loop {
			self.get_files();
			self.draw();
			self.input();
		}
	}

	fn draw(&self) {
		queue!(stdout(), Clear(ClearType::All), cursor::Hide, MoveTo(0, 0)).unwrap();
		print!("Open editors: {}", self.editors.len());

		for (index, editor) in self.editors.iter().enumerate() {
			if index == self.selected {
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

		let offset = self.editors.len() as u16 + 2;
		queue!(stdout(), MoveTo(0, offset)).unwrap();

		print!("Current dir: {}", self.path.to_string_lossy());
		for (index, path) in self.files.iter().enumerate() {
			if index == self.selected.wrapping_sub(self.editors.len()) {
				queue!(stdout(), SetColors(Colors::new(Color::Black, Color::White))).unwrap();
			}
			queue!(stdout(), MoveTo(1, index as u16 + 1 + offset)).unwrap();
			if let Some(name) = path.file_name() {
				print!("{}", name.to_string_lossy());
			} else {
				print!("{}", path.to_string_lossy());
			}
			if path.is_dir() {
				print!("/");
			}
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
				KeyCode::Enter => self.enter(),
				KeyCode::Home => self.path = env::current_dir().unwrap(),
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
		self.selected = self.selected.saturating_sub(1);
	}

	fn nav_down(&mut self) {
		self.selected = (self.selected + 1).min(self.editors.len() + self.files.len() - 1);
	}

	fn enter(&mut self) {
		if self.selected < self.editors.len() {
			self.editors[self.selected].enter();
		} else {
			let i = self.selected - self.editors.len();
			if i == 0 {
				if let Some(parent) = self.path.parent() {
					self.path = parent.to_owned()
				}
			} else {
				let path = &self.files[i];
				if path.is_dir() {
					self.path = self.path.join(path);
					self.selected = self.editors.len();
				} else if path.is_file() {
					if let Some(editor) =
						Editor::open_file(self.clipboard.clone(), path.canonicalize().unwrap())
					{
						self.selected = self.editors.len();
						self.editors.push(editor);
						self.open_selected()
					}
				}
			}
		}
	}

	fn open_selected(&mut self) {
		if self.selected < self.editors.len() {
			self.editors[self.selected].enter();
		}
	}

	fn new_editor(&mut self) {
		self.selected = self.editors.len();
		self.editors.push(Editor::new_empty(self.clipboard.clone()));
		self.open_selected();
	}

	fn get_files(&mut self) {
		self.files.clear();
		self.files.push(PathBuf::from(".."));
		for file in fs::read_dir(&self.path).unwrap().flatten() {
			self.files.push(file.path());
		}
	}

	fn quit(&self) {
		disable_raw_mode().unwrap();
		execute!(stdout(), LeaveAlternateScreen, cursor::Show).unwrap();
		exit(0);
	}
}
