use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Layout;
use ratatui::prelude::*;
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::router::route::{CurrentScreen, PageStatus};
use crate::router::router::Action;
use crate::router::screen::Screen;
use crate::router::{route, router::Router};
use crate::views::main_view::MainScreen;

#[derive(Debug, Clone, Copy, Default)]
pub enum ModeType {
    #[default]
    Auto,
    MoonShot,
}

pub struct App {
    pub router: Router,
    main_screen: MainScreen,
}

impl App {
    pub fn new() -> Self {
        Self {
            router: Router::new(route::CurrentScreen::Main(PageStatus::Normal)),
            main_screen: MainScreen::new(),
        }
    }

    pub fn render(&self, f: &mut Frame) {
        match self.router.current() {
            CurrentScreen::Main(PageStatus::Normal) => self.main_screen.render(f),
            _ => {}
        };
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let action = match self.router.current() {
            CurrentScreen::Main(PageStatus::Normal) => self.main_screen.handle_key(key),
            _ => Action::None,
        };
        self.router.dispatch(action);
    }

    pub fn print_json(&self) -> Result<(), ()> {
        println!("结束");
        Ok(())
    }
}
