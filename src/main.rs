mod app;
mod llm;
// mod ui;
mod router;
mod views;

use ratatui::Terminal;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::{Backend, CrosstermBackend};
use std::error::Error;
use std::io;

use crate::app::app::App;
use crate::llm::request_llm::request_llm;
use crate::router::route::CurrentScreen;
// use crate::ui::ui::DrawUI;

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool>
where
    io::Error: From<B::Error>,
{
    loop {
        terminal.draw(|f| app.render(f))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            app.handle_key(key);
            // if app.show_select {
            //     match key.code {
            //         KeyCode::Down | KeyCode::Tab => {
            //             app.select_next();
            //             continue;
            //         }
            //         KeyCode::Up => {
            //             app.select_pervious();
            //             continue;
            //         }
            //         KeyCode::Enter => {
            //             app.current_screen = CurrentScreen::Main;
            //         },
            //         KeyCode::Esc => {
            //             app.current_screen = CurrentScreen::Main;
            //             app.input_value = String::new();
            //         }
            //         KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            //             app.current_screen = CurrentScreen::Main;
            //             app.input_value = String::new();
            //         }
            //         _ => {}
            //     }
            // }
            // match app.current_screen {
            //     CurrentScreen::Main => match key.code {
            //         KeyCode::Esc => {
            //             app.current_screen = CurrentScreen::Exiting;
            //             app.input_value = String::new();
            //         }
            //         KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            //             app.current_screen = CurrentScreen::Exiting;
            //             app.input_value = String::new();
            //         }
            //         KeyCode::Enter => {
            //             app.save_current_value();
            //             if !app.input_value.is_empty() {
            //                 request_llm().await.expect("llm is error.");
            //             }
            //         }
            //         _ => {
            //             app.input_change(&Event::Key(key));
            //         }
            //     },
            //     CurrentScreen::Exiting => match key.code {
            //         KeyCode::Char('y') => {
            //             return Ok(true);
            //         }
            //         KeyCode::Char('n') | KeyCode::Char('q') => {}
            //         KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            //             return Ok(false);
            //         }
            //         _ => {}
            //     },
            //     _ => {}
            // }
        }

        if app.router.should_quit {
            return Ok(false);
        }
        
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Ok(do_print) = res {
        if do_print {
            let _ = app.print_json();
        }
    } else if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
