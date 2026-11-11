//! The [`Prompt`] of a real terminal, on `inquire`. Escape goes back, and so
//! does Ctrl-C: nothing here is worth killing the session for.

use inquire::{Confirm, Select, Text};

use crate::menu::Prompt;

pub struct Terminal;

impl Prompt for Terminal {
    fn select(&mut self, message: &str, options: &[String]) -> Option<usize> {
        Select::new(message, options.to_vec())
            .with_help_message("↑↓ to move, enter to choose, esc to go back")
            .raw_prompt()
            .ok()
            .map(|chosen| chosen.index)
    }

    fn text(&mut self, message: &str) -> Option<String> {
        Text::new(message)
            .with_help_message("esc to go back")
            .prompt()
            .ok()
            .filter(|text| !text.trim().is_empty())
    }

    fn confirm(&mut self, message: &str) -> bool {
        Confirm::new(message).with_default(false).prompt().unwrap_or(false)
    }

    fn show(&mut self, text: &str) {
        println!("\n{text}\n");
    }
}
