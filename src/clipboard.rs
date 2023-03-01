use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct Clipboard {
	clipboard: Rc<RefCell<Internal>>,
}

impl Clipboard {
	pub fn new() -> Self {
		Self {
			clipboard: Rc::new(RefCell::new(Internal::new())),
		}
	}

	pub fn get(&self) -> String {
		self.clipboard.borrow().get().to_owned()
	}

	pub fn set(&mut self, text: String) {
		self.clipboard.borrow_mut().set(text);
	}
}

struct Internal {
	contents: String,
}

impl Internal {
	fn new() -> Self {
		Self {
			contents: String::new(),
		}
	}

	fn get(&self) -> &str {
		&self.contents
	}

	fn set(&mut self, text: String) {
		self.contents = text;
	}
}
