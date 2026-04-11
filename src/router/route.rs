#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurrentScreen {
    Main(PageStatus),
    List(PageStatus)
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageStatus {
    #[default]
    Normal,
    Exiting
}

impl Default for CurrentScreen {
    fn default() -> Self {
        CurrentScreen::Main(PageStatus::Normal)
    }
}
