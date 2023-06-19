use anyhow::Result;
use log::info;
use crossterm::{event::{KeyCode, KeyEvent}, cursor::MoveToPreviousLine};
use ratatui::{
  layout::{Rect, Layout, Direction, Constraint},
  style::{Color, Style, Modifier},
  widgets::{Block, Borders, List, ListItem, ListState},
  widgets,
};

use crate::action::{Action, FilterListAction, FilterType};
use crate::action::LineFilter;

use super::{Component, Frame, text_entry::TextEntry};

#[derive(Default)]
pub struct FilterScreen<'a> {
  // state: TuiWidgetState,
  pub items: Vec<LineFilter>,
  state: ListState,
  // show_new: bool,
  new_filter_type: Option<FilterType>,
  new: TextEntry<'a>,
}

impl FilterScreen<'_> {
  fn show_new(&self) -> bool { self.new_filter_type.is_some() }

  fn confirm_new_filter(&mut self) {
    self.items.push(LineFilter::new(self.new.textarea.lines()[0].clone(), self.new_filter_type.unwrap()));
    self.new_filter_type = None;
    self.new.textarea.delete_line_by_head();
    info!("Added new filter: {:?}", self.items[self.items.len()-1]);
  }
}

impl Component for FilterScreen<'_> {
  fn init(&mut self) -> Result<()> {
    // self.state = TuiWidgetState::new().set_default_display_level(LevelFilter::Debug);
    // self.state = ListState::default();
    Ok(())
  }

  fn on_key_event(&self, key: KeyEvent) -> Action {
    if self.show_new() {
      return match key.code {
        KeyCode::Esc => Action::FilterListAction(FilterListAction::CloseNew),
        KeyCode::Enter => Action::FilterListAction(FilterListAction::ConfirmNew),
        _ => self.new.on_key_event(key),
        // _ => {
        //   match self.new.on_key_event(key) {
        //     Action::TextEntry(_) => Action::FilterListAction(Tex),
        //     x =>  x,
        // }
        // },
      }
    }
    match key.code {
      KeyCode::Char('q') => Action::FilterListAction(crate::action::FilterListAction::CloseList),
      KeyCode::Esc => Action::FilterListAction(crate::action::FilterListAction::CloseList),
      KeyCode::Enter => Action::FilterListAction(crate::action::FilterListAction::CloseList),
      // KeyCode::Char('l') => Action::ToggleShowLogger,
      KeyCode::Char('j') => Action::FilterListAction(crate::action::FilterListAction::PrevItem),
      KeyCode::Char('k') => Action::FilterListAction(crate::action::FilterListAction::NextItem),
      KeyCode::Char('i') => Action::FilterListAction(FilterListAction::New(FilterType::In)),
      KeyCode::Char('o') => Action::FilterListAction(FilterListAction::New(FilterType::Out)),
      _ => Action::Tick,
  }
}


  fn dispatch(&mut self, action: Action) -> Option<Action> {
    let curr = self.state.selected().unwrap_or_default();
    // reverse the direction
    let next = Some(if curr == 0 { self.items.len() } else { curr - 1 });
    let prev = Some(if curr == self.items.len() { 0 } else { curr + 1});
    match action {
      Action::FilterListAction(fa) => {
        match fa {
            FilterListAction::NextItem => self.state.select(next),
            FilterListAction::PrevItem => self.state.select(prev),
            FilterListAction::New(which)=> self.new_filter_type = Some(which),
            FilterListAction::OpenFilterScreen => unimplemented!(),
            FilterListAction::CloseList => unimplemented!(),
            FilterListAction::CloseNew => { self.new_filter_type = None; self.new.textarea.delete_line_by_head(); },
            FilterListAction::ConfirmNew => self.confirm_new_filter(),
            /*FilterListAction::TextEntry(_) => {
              let o = self.new.dispatch(action);
              assert_eq!(o, None);
            },*/
        }
      }
      Action::TextEntry(_) => {
        self.new.dispatch(action);
      }
      _ => (),
    }
    None
  }

  fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
    let rect = if self.show_new() {
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        // .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .constraints([Constraint::Min(3), Constraint::Max(3)])
        .split(rect);
      let s = format!("Filter {:?}", self.new_filter_type.unwrap());
      let block = Block::default().title(s).borders(Borders::all());
      self.new.textarea.set_block(block);
      self.new.render(f, chunks[1]);
      chunks[0]
    } else {
      rect
    };
    let items: Vec<_> = self.items.iter().map(|i| {
      ListItem::new(i.needle.clone()).style(Style::default().bg(if i.filter_type.include() { Color::Green } else { Color::Red } ))
    }).collect();
    /*let items = [
      // ListItem::new("Item 1").style(Style::default().bg(Color::Blue)),

      ListItem::new("Item 1"),
      ListItem::new("Item 2"),
      ListItem::new("Item 3")];*/
    let l = List::new(items)
        .block(Block::default().title("Filters: (i) In (o) Out (space) Toggle Filter (q/enter/escape) Close ").borders(Borders::ALL.difference(Borders::BOTTOM)))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
      .highlight_symbol(">>");
    f.render_stateful_widget(l, rect, &mut self.state);
  }
}
