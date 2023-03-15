use crossterm::{
	cursor::{self, MoveTo},
	event::{self, Event, KeyCode, KeyModifiers},
	execute, queue,
	terminal::{
		self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
		LeaveAlternateScreen,
	},
};
use std::{
	env, fs,
	io::{stdout, Write},
	path::PathBuf,
	process::exit,
};

mod config;
mod editor;
mod util;
use config::Config;
use editor::Editor;
use util::{ask_yes_no, color_highlight, color_reset};

fn main() {
	Navigator::new().run();
}

struct Navigator {
	config: Config,
	editors: Vec<Editor>,
	files: Vec<PathBuf>,
	selected: usize,
	path: PathBuf,
	init_path: PathBuf,
	immediate_open: bool,
	message: Option<String>,
	scroll: usize,
}

impl Navigator {
	fn new() -> Self {
		let mut editors = Vec::new();
		let args: Vec<String> = env::args().skip(1).collect();
		let mut path = env::current_dir().unwrap();

		for arg in args.iter().map(PathBuf::from) {
			if arg.is_dir() {
				path = arg.canonicalize().unwrap();
				break;
			} else if arg.is_file() {
				if let Ok(editor) = Editor::open_file(arg) {
					editors.push(editor);
				}
			} else {
				editors.push(Editor::new(Some(arg)));
			}
		}
		if args.is_empty() {
			editors.push(Editor::new(None));
		}
		let immediate_open = editors.len() == 1;
		Self {
			config: Config::new(),
			editors,
			selected: 0,
			files: Vec::new(),
			init_path: path.clone(),
			path,
			immediate_open,
			message: None,
			scroll: 0,
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
			self.message = None;
			self.input();
		}
	}

	fn draw(&self) {
		queue!(stdout(), Clear(ClearType::All), cursor::Hide, MoveTo(0, 0)).unwrap();
		print!("Open editors: {}", self.editors.len());

		for (index, editor) in self.editors.iter().enumerate() {
			if index == self.selected {
				color_highlight();
			}
			queue!(stdout(), MoveTo(1, index as u16 + 1)).unwrap();
			print!("{}", editor.title());
			color_reset();
		}

		let offset = self.editors.len() as u16 + 2;
		queue!(stdout(), MoveTo(0, offset)).unwrap();
		print!("Current dir: {}", self.path.to_string_lossy());

		let height = terminal::size().unwrap().1;
		let max_rows = height as usize - self.editors.len() - 4;
		let end = (self.scroll + max_rows).min(self.files.len());
		let visible_rows = self.scroll..end;

		for (index, path) in self.files[visible_rows].iter().enumerate() {
			if index + self.scroll == self.selected.wrapping_sub(self.editors.len()) {
				color_highlight();
			}
			queue!(stdout(), MoveTo(1, index as u16 + 1 + offset)).unwrap();
			if let Some(name) = path.file_name() {
				print!("{}", name.to_string_lossy());
			} else {
				print!("..");
			}
			if path.is_dir() {
				print!("/");
			}
			color_reset();
		}

		if let Some(text) = &self.message {
			queue!(stdout(), MoveTo(0, height)).unwrap();
			print!("{text}");
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
				KeyCode::Home => self.path = self.init_path.clone(),
				KeyCode::Char('n') => {
					if event.modifiers == KeyModifiers::CONTROL {
						self.new_editor();
					}
				}
				_ => (),
			}
		}
	}

	fn message(&mut self, text: String) {
		self.message = Some(text);
	}

	fn nav_up(&mut self) {
		if self.selected > 0 {
			self.selected -= 1;
		} else {
			let selected_max = self.editors.len() + self.files.len();
			self.selected = selected_max - 1;
		}
		self.update_scroll();
	}

	fn nav_down(&mut self) {
		let selected_max = self.editors.len() + self.files.len();
		self.selected = (self.selected + 1) % selected_max;
		self.update_scroll();
	}

	fn update_scroll(&mut self) {
		let height = terminal::size().unwrap().1 as usize - self.editors.len() - 5;
		let selected_file = self.selected.saturating_sub(self.editors.len());
		self.scroll = self
			.scroll
			.clamp(selected_file.saturating_sub(height), selected_file);
	}

	fn enter(&mut self) {
		if self.selected < self.editors.len() {
			self.open_selected();
			return;
		}

		let i = self.selected - self.editors.len();
		// top entry is hardcoded to be ../
		if i == 0 {
			if let Some(parent) = self.path.parent() {
				self.set_path(self.path.join(parent));
			}
			return;
		}

		let path = &self.files[i];
		if path.is_dir() {
			self.set_path(self.path.join(path));
			return;
		}
		if path.is_file() {
			let path = path.canonicalize().unwrap();
			let mut selected = self.editors.len();
			for (i, editor) in self.editors.iter().enumerate() {
				if editor.path() == Some(&path) {
					selected = i;
					break;
				}
			}
			// no editor exists with this path
			if selected == self.editors.len() {
				match Editor::open_file(path) {
					Ok(editor) => self.editors.push(editor),
					Err(err) => {
						self.message(format!("Could not open file: {err}"));
						return;
					}
				}
			}
			self.selected = selected;
			self.open_selected();
		}
	}

	fn set_path(&mut self, new_path: PathBuf) {
		match env::set_current_dir(&new_path) {
			Ok(()) => {
				self.path = new_path;
				self.selected = self.editors.len();
			}
			Err(err) => self.message(format!("Could not navigate to directory: {err}")),
		}
	}

	fn open_selected(&mut self) {
		if self.selected < self.editors.len() {
			self.scroll = 0;
			self.editors[self.selected].enter(&mut self.config);
		}
	}

	fn new_editor(&mut self) {
		self.selected = self.editors.len();
		self.editors.push(Editor::new(None));
		self.open_selected();
	}

	fn get_files(&mut self) {
		self.files.clear();
		self.files.push(PathBuf::from(".."));
		for file in fs::read_dir(&self.path).unwrap().flatten() {
			self.files.push(file.path());
		}
		self.files[1..].sort_unstable_by(|path, other| {
			let by_type = path.is_file().cmp(&other.is_file());
			let by_name = path.cmp(other);
			by_type.then(by_name)
		});
	}

	fn any_unsaved(&self) -> bool {
		self.editors.iter().any(Editor::is_unsaved)
	}

	fn quit(&self) {
		if self.any_unsaved() && !ask_yes_no("Unsaved changes, quit anyway?", false) {
			return;
		}
		disable_raw_mode().unwrap();
		execute!(stdout(), LeaveAlternateScreen, cursor::Show).unwrap();
		exit(0);
	}
}
