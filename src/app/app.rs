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

struct Messages {
    message: String,
    is_user: bool,
}

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub command: String,
    pub description: String,
}

enum MessagesStateManage {
    Generating,
    Thinking,
    Stream,
    ToolCalling,
    ToolExecuting,
    TollResult,
    WaitingUser,
    Completed,
    Error,
    Cancelled,
}

pub struct App {
    // pub input_value: String,
    // pub input: Input,
    // pub current_screen: CurrentScreen,
    // pub mode: ModeType,

    // pub show_select: bool,
    // pub select_items: Vec<SelectItem>,
    // pub selected_index: usize,
    // pub filtered_items: Vec<SelectItem>,
    pub router: Router,
    main_screen: MainScreen,
}

impl App {
    // pub fn new() -> App {
    //     let input = Input::default();

    //     let select_items = vec![
    //         SelectItem {
    //             command: "/help".to_string(),
    //             description: "显示帮助信息".to_string(),
    //         },
    //         SelectItem {
    //             command: "/agents".to_string(),
    //             description: "查看可用的AI代理".to_string(),
    //         },
    //         SelectItem {
    //             command: "/clear".to_string(),
    //             description: "清除当前对话".to_string(),
    //         },
    //         SelectItem {
    //             command: "/skill".to_string(),
    //             description: "查看技能列表".to_string(),
    //         },
    //     ];
    //     App {
    //         input_value: String::new(),
    //         current_screen: CurrentScreen::Main,
    //         input,
    //         mode: ModeType::Auto,
    //         show_select: false,
    //         selected_index: 0,
    //         select_items: select_items.clone(),
    //         filtered_items: select_items,
    //     }
    // }

    // pub fn save_current_value(&mut self) {
    //     self.input_value = self.input.value_and_reset();
    // }

    // pub fn input_change(&mut self, event: &Event) {
    //     self.input.handle_event(event);
    //     self.check_input_for_select();
    // }

    // pub fn select_pervious(&mut self) {
    //     if !self.filtered_items.is_empty() {
    //         self.selected_index = self.selected_index.saturating_sub(1);
    //         if self.selected_index == 0 && self.filtered_items.len() > 0 {
    //             self.selected_index = self.filtered_items.len() - 1;
    //         }
    //     }
    // }

    // pub fn select_next(&mut self) {
    //     if !self.filtered_items.is_empty() {
    //         self.selected_index = (self.selected_index + 1) % self.filtered_items.len();
    //     }
    // }

    // fn check_input_for_select(&mut self) {
    //     let value = self.input.value().to_string();
    //     if value.starts_with('/') {
    //         self.show_select = true;
    //         self.filter_select_items(&value[..]);
    //     } else {
    //         self.show_select = false;
    //         self.selected_index = 0;
    //     }
    // }

    // fn filter_select_items(&mut self, query: &str) {
    //     self.filtered_items = self
    //         .select_items
    //         .iter()
    //         .filter(|item| item.command.starts_with(query))
    //         .cloned()
    //         .collect();

    //     if self.selected_index >= self.filtered_items.len() && !self.filtered_items.is_empty() {
    //         self.selected_index = 0;
    //     }
    // }

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
