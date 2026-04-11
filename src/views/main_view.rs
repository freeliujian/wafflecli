use crate::app::app::ModeType;
use crate::router::route::{ PageStatus};
use crate::router::{
    route::CurrentScreen,
    router::{Action, Router},
    screen::Screen,
};
use indoc::formatdoc;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_input::Input;
use std::env;

pub struct MainScreen {
    version: String,
    dir_name: String,
    mode: ModeType,
    page_status: PageStatus,
    input: Input,
    input_value: String
}

impl MainScreen {
    pub fn new() -> Self {
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
        let footer_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(footer_height), Constraint::Min(3)])
            .split(*chunk);

        self.create_input(f, &footer_area[0]);
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

        let footer_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(3)])
            .split(*chunk);
        f.render_widget(help_content, footer_area[1]);
    }

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
}

impl MainScreen {
    fn render_wrapper(&self, f: &mut Frame) {
        let area_height = f.area().height;
        let header_height = 5;
        let footer_height = 12;
        let content_height = area_height.saturating_sub(header_height + footer_height);
        println!("{:?}",area_height);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                // Constraint::Length(content_height),
                Constraint::Length(footer_height),
            ])
            .split(f.area());
        self.create_header(f, &chunks[0]);
        // self.create_content(f, &chunks[1]);
        // self.create_footer(f, &chunks[2])
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

impl Screen for MainScreen {
    fn render(&self, f: &mut Frame) {
        self.render_wrapper(f);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                if self.page_status == PageStatus::Exiting {
                    self.set_page_status(PageStatus::Normal);
                }
                Action::None
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                match self.page_status {
                    PageStatus::Normal => {
                        self.set_page_status(PageStatus::Exiting);
                        Action::None
                    }
                    PageStatus::Exiting => Action::Quit,
                }
            }
            KeyCode::Enter => {
                // app.save_current_value();
                // if !app.input_value.is_empty() {
                //     request_llm().await.expect("llm is error.");
                // }
                Action::None
            }
            _ => {
                if self.page_status == PageStatus::Exiting {
                    self.page_status = PageStatus::Normal;
                }
                Action::None
            }
        }
    }
}
