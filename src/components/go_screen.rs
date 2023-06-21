use std::panic;

use anyhow::Result;
use chrono::Local;
use log::info;
use crossterm::{event::{KeyCode, KeyEvent}, cursor::MoveToPreviousLine};
use ratatui::{
  layout::{Rect, Layout, Direction, Constraint},
  style::{Color, Style, Modifier},
  widgets::{Block, Borders, List, ListItem, ListState},
  widgets,
};

use crate::{action::{Action, FilterListAction, FilterType, CursorMove}, utils::initialize_panic_handler};
use crate::action::LineFilter;

use super::{Component, Frame, text_entry::TextEntry};

#[derive(Default)]
pub struct GoScreen {
  pub show: bool,
  txt: TextEntry<'static>,
  destination: Option<CursorMove>,
}

impl GoScreen {
  fn confirm(&mut self) {
    self.show = false;
    let dest = self.txt.pop();
    self.destination = self.parse_dest(&dest);
  }

  fn parse_dest(&self, dest: &str) -> Option<CursorMove> {
    // TODO Color red if not parseable.
    if dest.contains('%') {
        return Some(CursorMove::Percentage(dest[..dest.len()-1].parse().ok()?));
    }

    let res = dateparser::parse_with_timezone(dest, &Local).ok();
    if res.is_some() {
      return Some(CursorMove::Timestamp(res.unwrap()));
    }

    return None;
  }
  
  fn validate_and_store(&mut self) {
    self.destination = self.validate_entry();
  }

  fn validate_entry(&self) -> Option<CursorMove> {
    let contents = self.txt.contents();
    if contents.len() == 0 {
      None
    } else {
        // I wish it didn't have to be this way, Windows...
        // https://github.com/chronotope/chrono/issues/1150
      panic::set_hook(Box::new(|panic_info| {}));
      match panic::catch_unwind(|| {
        let ret = self.parse_dest(self.txt.contents());
        initialize_panic_handler();
        ret
      }) {
          Ok(res) => {
            initialize_panic_handler();
            res
          },
          Err(_) => {
            initialize_panic_handler();
            None
          }
      }
    }
  }

  fn valid_entry(&self) -> bool {
    if self.txt.contents().len() == 0 {
      true
    } else {
      self.destination.is_some()
    }
  }
}

impl Component for GoScreen {
  fn init(&mut self) -> Result<()> {
    Ok(())
  }

  fn on_key_event(&self, key: KeyEvent) -> Action {
      match key.code {
        KeyCode::Esc => Action::FilterListAction(FilterListAction::CloseNew),
        KeyCode::Enter => Action::FilterListAction(FilterListAction::ConfirmNew),
        _ => self.txt.on_key_event(key),
      }
  }

  fn dispatch(&mut self, action: Action) -> Option<Action> {
    let mut goto_dest = false;
    match action {
      Action::FilterListAction(fa) => {
        match fa {
            // FilterListAction::OpenGoScreen => self.show = true,
            FilterListAction::CloseNew => self.show = false,
            FilterListAction::ConfirmNew => {self.confirm(); self.show = false; goto_dest = true; },
            _ => todo!("{:?}", fa),
        }
      }
      Action::TextEntry(_) => {
        self.txt.dispatch(action);
        self.validate_and_store();
      }
      _ => (),
    }
    if goto_dest {
      Some(Action::CursorMove(self.destination?))
    } else {
      None
    }
  }

  fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
    /*let chunks = Layout::default()
      .direction(Direction::Vertical)
      // .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
      .constraints([Constraint::Min(3), Constraint::Max(3)])
      .split(rect);*/
    // let s = format!("Filter {:?}", self.new_filter_type.unwrap());
    let block = Block::default()
    .title("GoTo: (Enter) Confirm. Examples: \"09:44:21\" \"+5m\" (g) beginning (q/enter/escape) Close ")
    .borders(Borders::ALL.difference(Borders::BOTTOM))
    .style(Style::default().fg(if self.valid_entry() { Color::Green } else { Color::Red }));
    self.txt.textarea.set_block(block);
    self.txt.render(f, rect);
    // f.render_widget(self.txt, rect);
  }
}
