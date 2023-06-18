use std::borrow::Cow;

use bstr::{ByteSlice, BStr};
use crossterm::event::{KeyCode, KeyEvent};
use log::{warn, debug, info};
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Style},
  widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
// use tracing::debug;
use memmap::Mmap;
// use tracing::info;

use super::{logger::Logger, Component, Frame, filter_screen::FilterScreen};
use crate::action::Action;


pub struct LineFilter {
    needle: String,
    include: bool,
}

// type Line = str;
type Line<'a> = Cow<'a, str>;

enum LineFilterResult {
    Include,
    Exclude,
    Indifferent,
}

fn line_allowed(filter: &LineFilter, line: &str) -> LineFilterResult {
    if line.contains(&filter.needle) {
        match filter.include {
            true => return LineFilterResult::Include,
            false => return LineFilterResult::Exclude
        }
    }
    LineFilterResult::Indifferent
}

// TODO add coloring and highlighting.
fn fmt_visible_lines(lines: Vec<Line>) -> Vec<Line> { lines }


fn get_visible_lines<'a>(source: &'a BStr, filters: Vec<LineFilter>, rows: u16, cols: u16)
  -> Vec<Line> {
    // info!("{}x{}", rows, cols);
    // Dimensions don't  match what I'd expect, and we run out of text before filling screen.
    // Let's just mult by a constant factor since this "assuming a line may be too big to process at once"
    // is a huge pessimization anyway.
    // assert_eq!(rows, 64);
    let mut used_rows = 0;
    let mut used_cols = 0;
    let mut line_start = 0;
    let mut lines = Vec::with_capacity(1000);
    let maybe_add_line = |lines: &mut Vec<Line<'a>>, line: &'a BStr, used_rows: &mut u16| {
        let line = line.to_str_lossy();
        for filter in filters.iter() {
            match line_allowed(filter, &line) {
                LineFilterResult::Exclude => {
                    return;
                }
                LineFilterResult::Include => {
                    lines.push(line);
                    *used_rows += 1;
                    return;
                }
                LineFilterResult::Indifferent => {}
            }
        }
        // debug!("Pushed line: {}", line);
        lines.push(line);
        *used_rows += 1;
    };
    // let b = source.as_bytes();
    // Assumes always linewrap, one byte == one visible width char.
    // while used_rows < rows && i < source.len() {
    for (start, end, c) in source.char_indices() {
        // debug!("{} {} {}", start, end, c);
        match c {
            '\n' => {
                maybe_add_line(&mut lines, &source[line_start..end-1], &mut used_rows);
                line_start = end;
            },
            _ => {
                used_cols += 1;
                if used_cols == cols {
                    // No need to complete out the line atm, TODO.
                    // Assumption, user will be filtering on something that at least fits in the screen when scrolling by.
                    used_cols = 0;
                    used_rows += 1;
                }
            }
        }
        if used_rows == rows {
            maybe_add_line(&mut lines, &source[line_start..end], &mut used_rows);
            return lines;
        }
    }
    maybe_add_line(&mut lines, &source[line_start..source.len()], &mut used_rows);
    return lines;
}


#[cfg(test)]
mod tests {
    use super::*;

    static LINES: &str = "03/22/2022 08:51:06 INFO   :...mylogline
03/22/2022 08:51:08 INFO   :...mylogline";

    #[test]
    fn test_visible() {
      assert_eq!(get_visible_lines("lol".into(), vec!(), 80, 80).join("\n"), "lol");
    }

    #[test]
    fn test_visible1() {
      assert_eq!(get_visible_lines(LINES.into(), vec!(), 80, 80).join("\n"), LINES);
    }

    #[test]
    fn test_visible2() {
      let s = "\n\nhi\n\n".into();
      assert_eq!(get_visible_lines(s, vec!(), 80, 80).join("\n"), s);
      assert_eq!(get_visible_lines(s, vec!(), 1, 1).join("\n"), "\n");
      assert_eq!(get_visible_lines(s, vec!(), 2, 1).join("\n"), "\n\n");
      assert_eq!(get_visible_lines(s, vec!(), 3, 1).join("\n"), "\n\nh");
      assert_eq!(get_visible_lines(s, vec!(), 4, 1).join("\n"), "\n\nhi");
      assert_eq!(get_visible_lines(s, vec!(), 3, 2).join("\n"), "\n\nhi");
    }
}


pub struct Home {
  pub is_running: bool,

  pub show_logger: bool,
  pub logger: Logger,

  pub counter: usize,

  mmap: Mmap,
  byte_cursor: usize,
  filters: Vec<LineFilter>,

  show_filter_screen: bool,
  filter_screen: FilterScreen,
}

impl Home {
  pub fn new(mmap: Mmap) -> Self {
    Self {
      is_running: false,
      show_logger: false,
      logger: Logger::default(),
      counter: 0,
      mmap,
      byte_cursor: 0,
      filters: vec!(),
      show_filter_screen: true,
      filter_screen: FilterScreen::default(),
    }
  }

  pub fn tick(&self) {
    // debug!("Tick");
  }

  pub fn increment(&mut self, i: usize) {
    self.counter = self.counter.saturating_add(i);
  }

  pub fn decrement(&mut self, i: usize) {
    self.counter = self.counter.saturating_sub(i);
  }
}

impl Component for Home {
  fn init(&mut self) -> anyhow::Result<()> {
    self.is_running = true;
    Ok(())
  }

  fn on_key_event(&self, key: KeyEvent) -> Action {
    if self.show_filter_screen {
      let caught = self.filter_screen.on_key_event(key);
      if caught != Action::Tick {
        return caught;
      }
    }
    match key.code {
      KeyCode::Char('q') => Action::Quit,
      KeyCode::Char('l') => Action::ToggleShowLogger,
      KeyCode::Char('j') => Action::IncrementCounter,
      KeyCode::Char('k') => Action::DecrementCounter,
      _ => Action::Tick,
    }
  }

  fn dispatch(&mut self, action: Action) -> Option<Action> {
    match action {
      Action::Quit => { self.is_running = false; None },
      Action::Tick => { self.tick(); None },
      Action::ToggleShowLogger => { self.show_logger = !self.show_logger; None },
      Action::IncrementCounter => { self.increment(1); None },
      Action::DecrementCounter => { self.decrement(1); None },
      Action::FilterListAction(fa) => {
        match fa {
            crate::action::FilterListAction::CloseList => { self.show_filter_screen = false; None },
            _ => self.filter_screen.dispatch(action),
        }
      }
      _ => None,
    }
    // None
  }

  fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
    let rect = if self.show_logger {
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rect);
      self.logger.render(f, chunks[1]);
      chunks[0]
    } else {
      rect
    };

    let rect = if self.show_filter_screen {
      let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rect);
      self.filter_screen.render(f, chunks[1]);
      chunks[0]
    } else {
      rect
    };

    let s = get_visible_lines(self.mmap.as_bstr(), vec!(), rect.height*10, rect.width*10);
    let s = s.join("\n");
    // info!("{}", s);
    // let os = format!("Press j or k to increment or decrement.\n\nCounter: {}", self.counter);
    f.render_widget(
      Paragraph::new(s)
      .alignment(Alignment::Left)
      .wrap(Wrap { trim: false })
      // .wrap(Wrap::None)
        // .block(
        //   Block::default()
        //     .title("Template")
        //     .title_alignment(Alignment::Center)
        //     .borders(Borders::ALL)
        //     .border_type(BorderType::Rounded),
        // )
        .style(Style::default().fg(Color::White)),
        // .alignment(Alignment::Center),
      rect,
    )
  }
}
