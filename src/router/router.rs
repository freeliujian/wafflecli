use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Paragraph},
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Settings,
    UserProfile,
    Help,
    NotFound,
}

#[derive(Debug)]
pub struct RouteHistory {
    stack: Vec<Route>,
    current_index: usize,
}

impl RouteHistory {
    pub fn new(initial: Route) -> Self {
        Self {
            stack: vec![initial],
            current_index: 0,
        }
    }

    pub fn push(&mut self, route: Route) {
        self.stack.truncate(self.current_index + 1);
        self.stack.push(route);
        self.current_index += 1;
    }

    pub fn back(&mut self) -> Option<Route> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(self.stack[self.current_index])
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<Route> {
        if self.current_index < self.stack.len() - 1 {
            self.current_index += 1;
            Some(self.stack[self.current_index])
        } else {
            None
        }
    }

    pub fn current(&self) -> Route {
        self.stack[self.current_index]
    }

    pub fn can_go_back(&self) -> bool {
        self.current_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.current_index < self.stack.len() - 1
    }
}

pub trait Page {
    fn render(&mut self, frame: &mut Frame, area: Rect);
    fn handle_input(&mut self, key: ratatui::crossterm::event::KeyEvent) -> Option<Route>;
    fn on_enter(&mut self) {}
    fn on_leave(&mut self) {}
}

pub struct Router {
    pages: HashMap<Route, Box<dyn Page>>,
    history: RouteHistory,
    current_route: Route,
}

impl Router {
    pub fn new(initial_route: Route) -> Self {
        Self {
            pages: HashMap::new(),
            history: RouteHistory::new(initial_route),
            current_route: initial_route,
        }
    }

    pub fn register<P: Page + 'static>(&mut self, route: Route, page: P) {
        self.pages.insert(route, Box::new(page));
    }

    pub fn navigate_to(&mut self, route: Route) {
        if route != self.current_route {
            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_leave();
            }

            self.history.push(route);
            self.current_route = route;

            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_enter();
            }
        }
    }

    pub fn go_back(&mut self) {
        if let Some(route) = self.history.back() {
            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_leave();
            }
            self.current_route = route;
            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_enter();
            }
        }
    }

    pub fn go_forward(&mut self) {
        if let Some(route) = self.history.forward() {
            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_leave();
            }
            self.current_route = route;
            if let Some(page) = self.pages.get_mut(&self.current_route) {
                page.on_enter();
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        if let Some(page) = self.pages.get_mut(&self.current_route) {
            page.render(frame, area);
        } else {
            let not_found = Paragraph::new("404 - Page Not Found")
                .block(Block::default().borders(Borders::ALL).title("Error"));
            frame.render_widget(not_found, area);
        }
    }

    pub fn handle_input(&mut self, key: ratatui::crossterm::event::KeyEvent) -> bool {
        use ratatui::crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                if self.history.can_go_back() {
                    self.go_back();
                    return true;
                }
            }
            KeyCode::Right
                if key
                    .modifiers
                    .contains(ratatui::crossterm::event::KeyModifiers::ALT) =>
            {
                if self.history.can_go_forward() {
                    self.go_forward();
                    return true;
                }
            }
            _ => {}
        }

        if let Some(page) = self.pages.get_mut(&self.current_route) {
            if let Some(route) = page.handle_input(key) {
                self.navigate_to(route);
                return true;
            }
        }

        false
    }

    pub fn current_route(&self) -> Route {
        self.current_route
    }
}
