use std::env;

mod editor;
mod util;
use editor::Editor;

fn main() {
	Editor::new(env::args().nth(1)).run();
}
