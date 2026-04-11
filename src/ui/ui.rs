use indoc::formatdoc;
use ratatui::{
    crossterm::terminal,
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use std::{env, thread::sleep};
use tui_input::Input;

use crate::app::app::{App, ModeType};
use crate::router::route::CurrentScreen;

#[derive(Default)]
pub struct DrawUI {
    pub state: String,
    pub input: Input,
    version: String,
    dir_name: String,
    current_screen: CurrentScreen,
    mode: ModeType,
    show_select: bool,
    select_items: Vec<(String, String)>,
    selected_index: usize,
}

impl DrawUI {
    pub fn new(app: &mut App) -> Self {
        let version = env!("CARGO_PKG_VERSION").into();
        let current_dir = env::current_dir().expect("没有找到当前目录");
        let dir_name = current_dir.to_string_lossy().into_owned();
        let select_items: Vec<(String, String)> = app
            .filtered_items
            .iter()
            .map(|item| (item.command.clone(), item.description.clone()))
            .collect();

        Self {
            state: app.input_value.clone(),
            input: app.input.clone(),
            version,
            dir_name,
            current_screen: app.current_screen.clone(),
            mode: app.mode.clone(),
            select_items,
            show_select: app.show_select,
            selected_index: app.selected_index,
        }
    }

    pub fn draw(&self, frame: &mut Frame) {
        let area_height = frame.area().height;
        let header_height = 5;
        let footer_height = if self.show_select { 12 } else { 6 };
        let content_height = area_height.saturating_sub(header_height + footer_height);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(header_height),
                Constraint::Length(content_height),
                Constraint::Length(footer_height),
            ])
            .split(frame.area());
        self.create_header(frame, &chunks[0]);
        self.create_content(frame, &chunks[1]);
        self.create_footer(frame, &chunks[2]);
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

    fn create_select(&self, f: &mut Frame, chunk: &Rect) {
        if !self.show_select || self.select_items.is_empty() {
            return;
        }

        let select_area = Rect {
            x: chunk.x,
            y: chunk
                .y
                .saturating_sub((self.select_items.len() as u16).min(6) + 2),
            width: chunk.width.min(60),
            height: (self.select_items.len() as u16).min(6) + 2,
        };

        f.render_widget(Clear, select_area);

        let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

        let rows: Vec<Row> = self
            .select_items
            .iter()
            .enumerate()
            .map(|(idx, (command, description))| {
                let is_selected = idx == self.selected_index;

                let style = if is_selected {
                    Style::default()
                        .fg(Color::LightGreen)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                Row::new(vec![
                    Cell::from(command.as_str()).style(style),
                    Cell::from(description.as_str()).style(if is_selected {
                        Style::default().fg(Color::LightGreen)
                    } else {
                        Style::default().fg(Color::Gray)
                    }),
                ])
            })
            .collect();

        let table = Table::new(rows, widths)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("Commands"),
            )
            .column_spacing(2)
            .row_highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        let mut table_state = TableState::default();
        table_state.select(Some(self.selected_index));

        f.render_stateful_widget(table, select_area, &mut table_state);
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

    fn create_footer(&self, f: &mut Frame, chunk: &Rect) {
        let footer_height = chunk.height;
        let footer_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(footer_height), Constraint::Min(3)])
            .split(*chunk);
        if self.show_select {
            let input_area = Rect {
                x: chunk.x,
                y: chunk.y + chunk.height - 3,
                width: chunk.width,
                height: 3,
            };

            let select_area = Rect {
                x: chunk.x,
                y: chunk.y,
                width: chunk.width,
                height: chunk.height - 3,
            };


            self.create_input(f, &input_area);
            self.create_select(f, &select_area);
        } else {
            self.create_input(f, &footer_area[0]);
            let help_description = if let CurrentScreen::Exiting = self.current_screen {
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
            let help_area = if self.show_select {
                Rect {
                    x: chunk.x,
                    y: chunk.y + chunk.height - 1,
                    width: chunk.width,
                    height: 1,
                }
            } else {
                let footer_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(3)])
                    .split(*chunk);
                footer_area[1]
            };
            f.render_widget(help_content, help_area);
        }
    }

    fn create_header(&self, f: &mut Frame, chunk: &Rect) {
        let title_width = 50;
        let welcome = String::from("Welcome to Qoder CLI!");

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
