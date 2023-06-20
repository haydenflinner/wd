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

use crate::action::{Action, FilterListAction, FilterType, CursorMove};
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
    let dest = self.txt.textarea.lines()[0].clone();
    self.show = false;
    // TODO delete whole line
    self.txt.textarea.delete_line_by_head();
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
            FilterListAction::CloseList => self.show = false,
            FilterListAction::ConfirmNew => {self.confirm(); self.show = false; goto_dest = true; },
            _ => todo!(),
        }
      }
      Action::TextEntry(_) => {
        self.txt.dispatch(action);
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
    let block = Block::default().title("GoTo: (Enter) Confirm. Examples: \"09:44:21\" \"+5m\" (g) beginning (q/enter/escape) Close ").borders(Borders::ALL.difference(Borders::BOTTOM));
    self.txt.textarea.set_block(block);
    self.txt.render(f, rect);
    // f.render_widget(self.txt, rect);
  }
}
