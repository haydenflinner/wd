use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, Utc};
// use crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEvent;
use env_logger::filter::Filter;
use futures::{FutureExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    // It might be cool to have an ! button on filters,
    // so we can say !In => if line includes this, it will show, no further filters apply.
    In,
    Out,
}

impl FilterType {
    pub fn include(&self) -> bool {
        match self {
            FilterType::In => true,
            FilterType::Out => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineFilter {
    pub needle: String,
    pub filter_type: FilterType,
    pub enabled: bool,
}
impl LineFilter {
    pub fn new(needle: String, filter_type: FilterType) -> Self {
        Self {
            needle,
            filter_type,
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterListAction {
    OpenFilterScreen,
    NextItem,
    PrevItem,
    CloseList,

    // TextEntry(KeyEvent),
    New(FilterType),
    CloseNew,
    ConfirmNew,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Prev,
    Next,
}

pub struct TimeDelta {
    num_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CursorMove {
    /// 'j' or 'k' in less
    OneLine(Direction),
    /// 'G' or 'gg' in less
    End(Direction),
    /// Percentage through the file. '54%' in less => 0.54
    Percentage(f64),
    // /// Absolute lineno, ':54' in less.
    // LineNo(u64),
    /// Put the cursor at the beginning of the first line with a timestamp >= this timestamp.
    Timestamp(DateTime<Utc>),
    // /// E.g. "+1s, +5m". TODO
    // TimeDelta(TimeDelta),
    Screenful(Direction),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Quit,
    Tick,
    Resize(u16, u16),

    CursorMove(CursorMove),
    /// Pressed 'g' which means may soon press 'g' again for 'gg' beg-file cmd.
    OpenGoScreen,

    ToggleShowLogger,
    BeginSearch,
    RepeatSearch(Direction),

    FilterListAction(FilterListAction),

    OpenTextEntry,
    // TODO Move these and the .show to pub members of the sub-component?
    CloseTextEntry,
    ConfirmTextEntry,
    TextEntry(KeyEvent),

    /// Use drainrs to autoskip what's on screen.
    AutoSkip,

    Noop,
}

#[derive(Debug)]
pub struct ActionHandler {
    pub sender: mpsc::UnboundedSender<Action>,
    pub receiver: mpsc::UnboundedReceiver<Action>,
}

impl ActionHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self { sender, receiver }
    }

    pub async fn recv(&mut self) -> Action {
        let action = self.receiver.recv().await;
        debug!("Received action {:?}", action);
        action.unwrap_or(Action::Quit)
    }

    pub async fn send(&self, action: Action) -> Result<()> {
        debug!("Sending action {:?}", action);
        self.sender.send(action)?;
        Ok(())
    }
}
