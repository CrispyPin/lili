use crossterm::{
	cursor,
	event::{self, Event, KeyCode},
	queue, terminal,
};
use std::io::{stdout, Write};

pub fn read_line(prompt: &str) -> Option<String> {
	let mut response = String::new();
	let size = terminal::size().unwrap();
	let start_pos = cursor::MoveTo(0, size.1);

	queue!(stdout(), start_pos).unwrap();
	print!("{prompt}");
	stdout().flush().unwrap();

	loop {
		if let Ok(Event::Key(event)) = event::read() {
			match event.code {
				KeyCode::Enter => break,
				KeyCode::Char(ch) => response.push(ch),
				KeyCode::Backspace => {
					response.pop();
				}
				KeyCode::Esc => return None,
				_ => (),
			}
		}
		queue!(stdout(), start_pos).unwrap();
		print!("{prompt}{response} ");
		stdout().flush().unwrap();
	}
	Some(response.trim().into())
}
