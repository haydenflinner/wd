use anyhow::Result;
use crossterm::{event::{KeyCode, KeyEvent}, cursor::MoveToPreviousLine};
use ratatui::{
  layout::Rect,
  style::{Color, Style, Modifier},
  widgets::{Block, Borders, List, ListItem, ListState},
  widgets,
};

use crate::action::Action;
use crate::components::home::LineFilter;

use super::{Component, Frame};

#[derive(Default)]
pub struct FilterScreen {
  // state: TuiWidgetState,
  pub items: Vec<LineFilter>,
  state: ListState,
}

impl Component for FilterScreen {
  fn init(&mut self) -> Result<()> {
    // self.state = TuiWidgetState::new().set_default_display_level(LevelFilter::Debug);
    // self.state = ListState::default();
    Ok(())
  }

  fn on_key_event(&self, key: KeyEvent) -> Action {
    match key.code {
      KeyCode::Char('q') => Action::FilterListAction(crate::action::FilterListAction::CloseList),
      // KeyCode::Char('l') => Action::ToggleShowLogger,
      KeyCode::Char('j') => Action::FilterListAction(crate::action::FilterListAction::PrevItem),
      KeyCode::Char('k') => Action::FilterListAction(crate::action::FilterListAction::NextItem),
      _ => Action::Tick,
  }
}


  fn dispatch(&mut self, action: Action) -> Option<Action> {
    let curr = self.state.selected().unwrap_or_default();
    let prev = Some(if curr == 0 { self.items.len() } else { curr - 1 });
    let next = Some(if curr == self.items.len() { 0 } else { curr + 1});
    match action {
      Action::FilterListAction(fa) => {
        match fa {
            crate::action::FilterListAction::NextItem => self.state.select(next),
            crate::action::FilterListAction::PrevItem => self.state.select(prev),
            _ => { }
        }
      }
      _ => (),
    }
    None
  }

  fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
    let items = [
      // ListItem::new("Item 1").style(Style::default().bg(Color::Blue)),
      ListItem::new("Item 1"),
      ListItem::new("Item 2"),
      ListItem::new("Item 3")];
    let l = List::new(items)
        .block(Block::default().title("List").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
      .highlight_symbol(">>");
    /*let w = TuiFilterScreenWidget::default()
      .block(Block::default().title("Log").borders(Borders::ALL))
      .style_error(Style::default().fg(Color::Red))
      .style_debug(Style::default().fg(Color::Green))
      .style_warn(Style::default().fg(Color::Yellow))
      .style_trace(Style::default().fg(Color::Magenta))
      .style_info(Style::default().fg(Color::Cyan))
      .output_separator(':')
      .output_timestamp(Some("%H:%M:%S".to_string()))
      .output_level(Some(TuiFilterScreenLevelOutput::Long))
      .output_target(false)
      .output_file(true)
      .output_line(true)
      .state(&self.state);*/
    // self.state.select(Some(2));
    // self.state.select(None);
    f.render_stateful_widget(l, rect, &mut self.state);
    // f.render_widget(l, rect);
  }
}
