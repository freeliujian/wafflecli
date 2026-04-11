use ratatui::crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::router::router::Action;

pub trait Screen {
  fn render(&self, frame:&mut Frame);

  fn handle_key(&mut self, key: KeyEvent) -> Action;
}