use crate::router::route::CurrentScreen;

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Push(CurrentScreen),
    Pop,
    Replace(CurrentScreen),
    Quit,
    None,
}

pub struct Router {
  history: Vec<CurrentScreen>,
  pub should_quit: bool,
}

impl Router {
    pub fn new(init: CurrentScreen) -> Self {
      Self { history: vec![init], should_quit: false }
    }

    pub fn current(&self) -> &CurrentScreen {
      self.history.last().expect("导航栈不能为空")
    }

    pub fn dispatch(&mut self, action: Action) {
      match action {
          Action::Push(route) => {
            self.history.push(route);
          }
          Action::Pop => {
            if self.history.len() > 1{
              self.history.pop();
            }
          }
          Action::Replace(route) => {
            if let Some(last) = self.history.last_mut() {
              *last = route;
            }
          }

          Action::Quit => {
            self.should_quit = true;
          }

          Action::None => {}
      }
    }

    pub  fn can_go_back(&self) -> bool {
      self.history.len() > 1
    }

    pub fn stack_depth(&self) -> usize {
      self.history.len()
    }
}
