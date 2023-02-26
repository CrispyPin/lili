use std::env;

mod editor;
use editor::Editor;

fn main() {
	Editor::new(env::args().nth(1)).run();
}
