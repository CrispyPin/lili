use crossterm::{
	cursor,
	event::{self, Event, KeyCode},
	queue,
	style::{Color, Colors, ResetColor, SetColors},
	terminal,
};
use std::io::{stdout, Write};

pub fn read_line(prompt: &str) -> Option<String> {
	let mut response = String::new();
	let size = terminal::size().unwrap();
	let start_pos = cursor::MoveTo(0, size.1);
	let width = size.0 as usize;

	queue!(stdout(), start_pos).unwrap();
	print!("{:width$}", " ");
	queue!(stdout(), start_pos).unwrap();
	print!("{prompt}");
	stdout().flush().unwrap();

	loop {
		if let Ok(Event::Key(event)) = event::read() {
			match event.code {
				KeyCode::Enter => break,
				KeyCode::Char(ch) => response.push(ch),
				KeyCode::Backspace => {
					queue!(stdout(), start_pos).unwrap();
					print!("{:width$}", " ");
					response.pop();
				}
				KeyCode::Esc => return None,
				_ => (),
			}
		}
		queue!(stdout(), start_pos).unwrap();
		print!("{prompt}{response}");
		stdout().flush().unwrap();
	}
	Some(response.trim().into())
}

pub fn color_highlight() {
	queue!(stdout(), SetColors(Colors::new(Color::Black, Color::White))).unwrap();
}

pub fn color_reset() {
	queue!(stdout(), ResetColor).unwrap();
}
