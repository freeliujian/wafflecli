use crate::app::app::ModeType;
use crate::router::route::PageStatus;
use crate::router::{
    route::CurrentScreen,
    router::{Action, Router},
    screen::Screen,
};
use indoc::formatdoc;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::symbols::{block, border};
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use std::env;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug, Clone)]
pub struct SelectItem {
    pub command: String,
    pub description: String,
}

pub struct MainScreen {
    version: String,
    dir_name: String,
    mode: ModeType,
    page_status: PageStatus,
    input: Input,
    input_value: String,
    show_select: bool,
    select_items: Vec<SelectItem>,
    selected_index: usize,
    selected_current_item: SelectItem,
    filtered_items: Vec<SelectItem>,
    select_scroll_offset: usize,
    select_visible_count: usize,
}

impl MainScreen {
    pub fn new() -> Self {
        let select_items = vec![
            SelectItem {
                command: "/help".to_string(),
                description: "显示帮助信息".to_string(),
            },
            SelectItem {
                command: "/agents".to_string(),
                description: "查看可用的AI代理".to_string(),
            },
            SelectItem {
                command: "/clear".to_string(),
                description: "清除当前对话".to_string(),
            },
            SelectItem {
                command: "/skill".to_string(),
                description: "查看技能列表".to_string(),
            },
        ];
        let version = env!("CARGO_PKG_VERSION").into();
        let current_dir = env::current_dir().expect("没有找到当前目录");
        let dir_name = current_dir.to_string_lossy().into_owned();
        let init_input_value = String::new();

        let input = Input::new(init_input_value.clone());
        MainScreen {
            version,
            dir_name,
            mode: ModeType::Auto,
            page_status: PageStatus::Normal,
            input,
            input_value: init_input_value,
            show_select: false,
            select_items: select_items.clone(),
            selected_index: 0,
            filtered_items: select_items,
            selected_current_item: SelectItem {
                command: String::new(),
                description: String::new(),
            },
            select_scroll_offset: 0,
            select_visible_count: 5,
        }
    }

    pub fn set_page_status(&mut self, status: PageStatus) {
        self.page_status = status;
    }

    pub fn get_page_status(&mut self) -> PageStatus {
        self.page_status
    }
}

impl MainScreen {
    fn create_footer(&self, f: &mut Frame, chunk: &Rect) {
        let footer_height = chunk.height;
        let input_height = 3;
        let select_height = if self.show_select {
            self.select_visible_count as u16 + 2
        } else {
            0
        };
        let help_height = 3;
        let footer_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(input_height),
                Constraint::Length(select_height),
                Constraint::Length(help_height),
            ])
            .split(*chunk);

        self.create_input(f, &footer_area[0]);

        if self.show_select {
            self.create_select(f, &footer_area[1]);
        }
        self.create_help(f, &footer_area[2]);
    }

    fn create_help(&self, f: &mut Frame, chunk: &Rect) {
        let help_description = if let PageStatus::Exiting = self.page_status {
            formatdoc! {
            "press Ctrl + c or y again to quit
                --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
            Model: Auto · {dir_name}
            ",
            dir_name = self.dir_name
            }
        } else {
            formatdoc! {
            "? for shortcuts, ctrl+j for newline, ctrl+f for images, ctrl+c to exit
                --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
            Model: Auto · {dir_name}
            ",
            dir_name = self.dir_name
            }
        };

        let help_content =
            Paragraph::new(help_description).style(Style::default().fg(Color::DarkGray));

        f.render_widget(help_content, *chunk);
    }
}

impl MainScreen {
    fn render_wrapper(&self, f: &mut Frame) {
        // let area_height = f.area().height;
        let header_height = 5;
        let select_height = if self.show_select {
            self.select_visible_count as u16 + 2
        } else {
            0
        };
        let footer_height = 3 + select_height + 3;
        // let content_height = area_height.saturating_sub(header_height + footer_height);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Min(4),
                Constraint::Length(footer_height),
            ])
            .split(f.area());
        self.create_header(f, &chunks[0]);
        self.create_content(f, &chunks[1]);
        self.create_footer(f, &chunks[2])
    }

    fn create_content(&self, f: &mut Frame, chunk: &Rect) {
        let content_width = 120;
        let description = String::from("这是一个仿照qodercli的命令行工具，这里是描述段落");
        let description_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(content_width), Constraint::Fill(1)])
            .split(*chunk)[0];
        let content_block = Block::default();
        let content = Paragraph::new(description).block(content_block);
        f.render_widget(content, description_area);
    }

    fn create_header(&self, f: &mut Frame, chunk: &Rect) {
        let title_width = 50;
        let welcome = String::from("Welcome to waffle CLI!");

        let block_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(title_width)])
            .split(*chunk)[0];

        let title_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let title_lines = vec![
            Line::from(vec![
                Span::from(format!("{} {}", welcome, self.version)).green(),
            ]),
            Line::from(" "),
            Line::from(vec![
                Span::from(format!("cwd: {}", self.dir_name)).dark_gray(),
            ]),
        ];

        let title = Paragraph::new(title_lines).block(title_block);

        f.render_widget(title, block_area);
    }
}

impl MainScreen {
    fn create_input(&self, f: &mut Frame, chunk: &Rect) {
        let width = chunk.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let input = Paragraph::new(self.input.value())
            .style(Style::default())
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Type your message..."));
        f.render_widget(input, *chunk);

        let x = self.input.visual_cursor().max(scroll) - scroll + 1;
        f.set_cursor_position((chunk.x + x as u16, chunk.y + 1))
    }

    fn save_current_value(&mut self) {
        self.input_value = self.input.value_and_reset();
    }

    fn input_change(&mut self, event: &Event) {
        self.input.handle_event(event);
        self.check_input_for_select();
    }
}

impl MainScreen {
    fn create_select(&self, f: &mut Frame, chunk: &Rect) {
        let total_items = self.filtered_items.len();
        let visible = self.select_visible_count;

        if total_items == 0 {
            let empty_block = Block::new().title("Commands");
            let empty_text = Paragraph::new("No matches").block(empty_block);
            f.render_widget(empty_text, *chunk);
            return;
        }

        let start = self.select_scroll_offset;
        let end = (start + visible).min(total_items);

        let items: Vec<ListItem> = self
            .filtered_items
            .iter()
            .enumerate()
            .skip(start)
            .take(visible)
            .map(|(idx, item)| {
                let global_idx = idx;
                let content = format!(" {} - {}", item.command, item.description);
                let style = if global_idx == self.selected_index {
                    Style::default()
                        .bg(Color::LightGreen)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(content).style(style)
            })
            .collect();

        let list_block = Block::new().title("Commands");

        let select_list = List::new(items).block(list_block);

        f.render_widget(select_list, *chunk);

        if total_items > visible {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(total_items)
                .position(self.selected_index)
                .viewport_content_length(visible);

            f.render_stateful_widget(scrollbar, *chunk, &mut scrollbar_state);
        }
    }

    fn update_scroll(&mut self) {
        let visible = self.select_visible_count;
        let total = self.filtered_items.len();
        println!("visible:{},total:{}", visible, total);
        if total <= visible {
            self.select_scroll_offset = 0;
        } else {
            if self.selected_index < self.select_scroll_offset {
                self.select_scroll_offset = self.selected_index;
            } else if self.selected_index >= self.select_scroll_offset + visible {
                self.select_scroll_offset = self.selected_index.saturating_sub(visible - 1);
            }
        }
    }

    fn filter_select_items(&mut self, query: &str) {
        self.filtered_items = self
            .select_items
            .iter()
            .filter(|item| item.command.starts_with(query))
            .cloned()
            .collect();

        self.selected_index = 0;
        self.select_scroll_offset = 0;
    }

    fn check_input_for_select(&mut self) {
        let value = self.input.value().to_string();
        if value.starts_with('/') {
            self.show_select = true;
            self.filter_select_items(&value);
        } else {
            self.show_select = false;
            self.selected_index = 0;
            self.select_scroll_offset = 0;
        }
    }

    fn select_previous(&mut self) {
        if !self.filtered_items.is_empty() {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(self.filtered_items.len() - 1);
            self.update_scroll();
        }
    }

    fn select_next(&mut self) {
        if !self.filtered_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_items.len();
            self.update_scroll();
        }
    }

    fn current_item(&self) -> Option<&SelectItem> {
        self.filtered_items.get(self.selected_index)
    }
}

impl Screen for MainScreen {
    fn render(&self, f: &mut Frame) {
        self.render_wrapper(f);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        if let KeyCode::Char('c') = key.code {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match self.page_status {
                    PageStatus::Normal => {
                        self.set_page_status(PageStatus::Exiting);
                        return Action::None;
                    }
                    PageStatus::Exiting => return Action::Quit,
                }
            }
        }

        if key.code == KeyCode::Esc {
            if self.page_status == PageStatus::Exiting {
                self.set_page_status(PageStatus::Normal);
            } else if self.show_select {
                self.show_select = false;
            }
            return Action::None;
        }

        if self.show_select {
            match key.code {
                KeyCode::Down | KeyCode::Tab => {
                    self.select_next();
                    return Action::None;
                }
                KeyCode::Up => {
                    self.select_previous();
                    return Action::None;
                }
                KeyCode::Enter => {
                    if let Some(item) = self.current_item() {
                        self.input_value = item.command.clone();
                    }
                    self.show_select = false;
                    return Action::None;
                }
                KeyCode::Esc => {
                    self.show_select = false;
                    return Action::None;
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Enter => {
                self.save_current_value();
                Action::None
            }
            _ => {
                if self.page_status == PageStatus::Exiting {
                    self.page_status = PageStatus::Normal;
                }
                self.input_change(&Event::Key(key));
                Action::None
            }
        }
    }
}
