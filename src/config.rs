pub struct Config {
	clipboard: String,
	pub line_numbers: bool,
}

impl Config {
	pub fn new() -> Self {
		Self {
			clipboard: String::new(),
			line_numbers: true,
		}
	}

	pub fn clipboard(&self) -> &str {
		&self.clipboard
	}

	pub fn set_clipboard(&mut self, text: String) {
		self.clipboard = text;
	}
}
