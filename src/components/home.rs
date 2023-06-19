use std::{borrow::Cow, cmp::min};

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
use crate::action::{Action, FilterListAction, CursorMove, LineFilter, FilterType};

/// TODO In order:
///  1. Add j and k for scroll down and up.#
///  2. Complete filter in/out
///  3. Add search (/)
///  4. Page up/down?
///  5. Search-caching

// type Line = str;
type Line<'a> = Cow<'a, str>;

#[derive(PartialEq, Eq)]
enum LineFilterResult {
    Include,
    Exclude,
    Indifferent,
}

fn line_allowed(filters: &Vec<LineFilter>, line: &str) -> bool {
  let mut cur = LineFilterResult::Indifferent;
  for filter in filters.iter() {
    match (line.contains(&filter.needle), filter.filter_type) {
      (true, crate::action::FilterType::In) => cur = LineFilterResult::Include,
      (true, crate::action::FilterType::Out) => cur = LineFilterResult::Exclude,
      _ => continue,
      // (false, crate::action::FilterType::In) => LineFilterResult::Indifferent,
      // (false, crate::action::FilterType::Out) => LineFilterResult::Indifferent,
    }
  }
  if cur == LineFilterResult::Indifferent && filters.iter().any(|f| f.filter_type == FilterType::In) {
    // If there are any MUST_INCLUDE lines and none of them matched, we should not print this line.
    return false;
  }
  return cur != LineFilterResult::Exclude;
}

// TODO add coloring and highlighting.
fn fmt_visible_lines(lines: Vec<Line>) -> Vec<Line> { lines }

pub fn get_visible_lines<'a, 'b>(source: &'a BStr, filters: &'b Vec<LineFilter>, rows: u16, cols: u16, offset_into_big: usize)
  -> Vec<DispLine> {
    // info!("{}x{}", rows, cols);
    // Dimensions don't  match what I'd expect, and we run out of text before filling screen.
    // Let's just mult by a constant factor since this "assuming a line may be too big to process at once"
    // is a huge pessimization anyway.
    // assert_eq!(rows, 64);
    let mut displayed_rows = 0;
    let mut rows_for_this_line = 0;
    let mut used_cols = 0;
    let mut line_start = 0;
    let mut lines = Vec::with_capacity(1000);
    let maybe_add_line = |lines: &mut Vec<DispLine>, ending_index: usize, displayed_rows: &mut u16, rows_for_this_line: &u16, line_start: usize| {
        let line = source[line_start..ending_index].to_str_lossy().into_owned();
        if line_allowed(filters, &line) {
            lines.push(DispLine { file_loc: (offset_into_big+line_start, offset_into_big+ending_index), line: line });
            *displayed_rows += rows_for_this_line + 1;
            return;
        }
    };
    // let b = source.as_bytes();
    // Assumes always linewrap, one byte == one visible width char.
    for (start, end, c) in source.char_indices() {
        match c {
            '\n' => {
                maybe_add_line(&mut lines, start, &mut displayed_rows, &rows_for_this_line, line_start);
                line_start = end;
                rows_for_this_line = 0;
                used_cols = 0;
                if displayed_rows == rows {
                  return lines;
                }
            },
            _ => {
                used_cols += 1;
                if used_cols == cols {
                    // No need to complete out the line atm, TODO.
                    // Assumption, user will be filtering on something that at least fits in the screen when scrolling by.
                    rows_for_this_line += 1;
                    used_cols = 0;
                    // Note a line which spans multiple screens will not be fully excluded if the thing we're filtering
                    // doesn't fit on-screen.
                    if displayed_rows+rows_for_this_line == rows {
                        maybe_add_line(&mut lines, end, &mut displayed_rows, &rows_for_this_line, line_start);
                        return lines;
                    }
                }
            }
        }
    }
    // if used_cols > 0 {
    // TODO Skip the newline if it terminates this line?
    maybe_add_line(&mut lines, source.len(), &mut displayed_rows, &rows_for_this_line, line_start);
    // }
    return lines;
}

/// Find first character of line starting at or before byte_offset.
fn find_line_starting_before(s: &[u8], byte_offset: usize) -> usize {
    // mmap[0..byte_offset].iter().rev().find('\n').unwrap_or(0)
    let bytes_before_offset_of_newline = 
        // s[0..byte_offset]
        s[..byte_offset]
        .iter()
        .rev()
        .enumerate()
        .find_map(|(index, val)| match val {
            val if *val == ('\n' as u8) => Some(index),
            _ => None,
        })
        .unwrap_or(byte_offset);
    // 4096 -> 3486 should find byte 10. TODO Unit test.
    let idx = byte_offset - bytes_before_offset_of_newline;
    assert!(idx == 0 || s[idx-1] == ('\n' as u8));
    return idx;
}

fn find_start_line_pct(mmap: &Mmap, pct: usize) -> usize {
    if pct == 0 {
        return 0;
    }
    find_line_starting_before(mmap, mmap.len() / pct)
}


#[cfg(test)]
mod tests {
    use super::*;

    static LINES: &str = "03/22/2022 08:51:06 INFO   :...mylogline
03/22/2022 08:51:08 INFO   :...mylogline";

    #[test]
    fn test_visible() {
      let call = |rows, cols| -> String {
        get_visible_lines("lol".into(), &vec!(), rows, cols, 0).iter().map(|l| l.line.clone()).intersperse("\n".to_string()).collect()
      };
      assert_eq!(call(80, 80), "lol");
    }

    #[test]
    fn test_visible1() {
      let call = |rows, cols| -> String {
        get_visible_lines(LINES.into(), &vec!(), rows, cols, 0).iter().map(|l| l.line.clone()).intersperse("\n".to_string()).collect()
      };
      assert_eq!(call(80, 80), LINES);
    }

    #[test]
    fn test_visible2() {
      let s = "\n\nhi\n\n".into();
      // let res = get_visible_lines(s, &vec!(), 80, 80);
      // let comp: Vec<Cow<'_, str>> = vec!(Cow::Borrowed("\n"));
      // assert_eq!(res, comp);
      // assert_eq!(get_visible_lines(s, &vec!(), 1, 1), comp);
      let call = |rows, cols| -> String {
        get_visible_lines(s, &vec!(), rows, cols, 0).iter().map(|l| l.line.clone()).intersperse("\n".to_string()).collect()
      };
      assert_eq!(call(80, 80), s);
      assert_eq!(call(1, 1), "");
      assert_eq!(call(2, 1), "\n");
      assert_eq!(call(3, 1), "\n\nh");
      assert_eq!(call(4, 1), "\n\nhi");
      assert_eq!(call(3, 2), "\n\nhi");
    }

    #[test]
    fn test_allowed() {
      assert!(
        line_allowed(
          &vec!(),
          "Lol"
        )
      );
      assert!(
        !line_allowed(
          &vec!(LineFilter::new("Lol".to_string(), FilterType::Out)),
          "Lol"
        )
      );
      assert!(
        line_allowed(
          &vec!(LineFilter::new("Lol".to_string(), FilterType::In)),
          "Lol"
        )
      );
      assert!(
        !line_allowed(
          &vec!(
            LineFilter::new("Lol".to_string(), FilterType::In),
            LineFilter::new("Lol".to_string(), FilterType::Out),
          ),
          "Lol"
        )
      );
      assert!(
        line_allowed(
          &vec!(
            LineFilter::new("Lol".to_string(), FilterType::Out),
            LineFilter::new("Lol".to_string(), FilterType::In),
          ),
          "Lol"
        )
      );
    }
}


pub struct DispLine {
  file_loc: (usize, usize),
  line: String,
}

impl Into<String> for DispLine {
  fn into(self) -> String {
    self.line
  }
}

// impl DispLine {
//   fn offset_of(whole_buffer: &BStr) -> (usize, usize) {
//     let start = part.as_ptr() as usize - whole_buffer.as_ptr() as usize;
//     let end = start + part.len();
//     (start, end)
//   }
// }

pub struct Home {
  pub is_running: bool,

  pub show_logger: bool,
  pub logger: Logger,

  pub counter: usize,

  mmap: Mmap,
  byte_cursor: usize,

  g_primed: bool,

  show_filter_screen: bool,
  filter_screen: FilterScreen<'static>,

  /// Pre-filtered based on each action.
  // view: Vec<Cow<'mmap, str>>,
  view: Vec<DispLine>,
  // view: Vec<String>,
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
      show_filter_screen: false,
      filter_screen: FilterScreen::default(),
      g_primed: false,
      view: Vec::default(),
    }
  }

  pub fn tick(&self) {
    // debug!("Tick");
  }

  /// Unfortunately doesn't account for the word-wrapping that ratatui does,
  /// so will skip whole lines sometimes rather than just a screen-line.
  pub fn next_line(&mut self) {
      // let bytes_til_newline = &mmap[cursor..cursor.PG_SIZE*4]
      // let bytes_til_newline = self
      //     .mmap[self.byte_cursor..self.mmap.len()]
      //     .iter()
      //     .enumerate()
      //     .find_map(|(index, val)| match val {
      //         val if *val == ('\n' as u8) => Some(index),
      //         _ => None,
      //     })
      //     .unwrap_or(0);
      // // assert!(self.mmap[byte_til_newline])
      // let byte_after_newline = min(self.mmap.len(), self.byte_cursor + bytes_til_newline + 1);

      self.byte_cursor = self.view[0].file_loc.1+1;
      info!("Set cursor to {}", self.byte_cursor);
  }

  pub fn goto_pct(&mut self, pct: usize) {
      self.byte_cursor = find_start_line_pct(&self.mmap, pct);
  }

  pub fn prev_line(&mut self) {
      self.byte_cursor = find_line_starting_before(&self.mmap, self.byte_cursor.saturating_sub(1));
  }

  pub fn goto_begin(&mut self) {
      self.byte_cursor = 0;
  }

  pub fn goto_end(&mut self) {
      // TODO Just set cursor to mmap.len()? Does cursor really need to be at beginning of valid line always?
      self.byte_cursor = find_line_starting_before(&self.mmap, self.mmap.len());
  }

  // fn get_view(&self, rect: Rect) -> String {
  //     // 4kb, TODO Could minimize this by knowing size of terminal+rows returned.
  //     // Also this re-validates as utf8 each redraw, who cares for now.
  //     // std::str::from_utf8(&self.mmap[self.byte_cursor./.min(PG_SIZE * 4, self.mmap.len())]).unwrap().to_string()
  //     // let lines = 
  //   let s = get_visible_lines(self.mmap[self.byte_cursor..].as_bstr(),
  //   &self.filter_screen.items, rect.height*10, rect.width*10);
  //   s.join("\n")
  //     // std::str::from_utf8(&self.mmap[self.byte_cursor..min(PG_SIZE * 4, self.mmap.len())]).unwrap()
  // }

  fn update_view(&mut self) {
    self.view = get_visible_lines(&self.mmap[self.byte_cursor..].as_bstr(), &self.filter_screen.items, 1000, 1000, self.byte_cursor);
    // &self.filter_screen.items, 1000, 1000).iter_mut().map(|s| s.line.clone()).collect();
  }

}

impl Component for Home {
  fn init(&mut self) -> anyhow::Result<()> {
    self.is_running = true;
    self.update_view();
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
      KeyCode::Char('g') => {
        match self.g_primed {
            true => Action::CursorMove(CursorMove::End(crate::action::Direction::Prev)),
            false => Action::Primeg,
        }
      }
      KeyCode::Char('G') => Action::CursorMove(CursorMove::End(crate::action::Direction::Next)),
      KeyCode::Char('j') => Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Next)),
      KeyCode::Char('k') => Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Prev)),
      KeyCode::Char('f') => Action::FilterListAction(FilterListAction::OpenFilterScreen),
      _ => Action::Tick,
    }
  }

  fn dispatch(&mut self, action: Action) -> Option<Action> {
    match action {
      Action::Quit => self.is_running = false,
      Action::Tick => self.tick(),
      Action::ToggleShowLogger => self.show_logger = !self.show_logger,
      Action::CursorMove(cm) => {
        match cm {
            CursorMove::OneLine(dir) => match dir {
                crate::action::Direction::Prev => self.prev_line(),
                crate::action::Direction::Next => self.next_line(),
            },
            CursorMove::End(dir) => match dir {
                crate::action::Direction::Prev => self.goto_begin(),
                crate::action::Direction::Next => self.goto_end(),
            },
            CursorMove::Timestamp(_) => todo!(),
        }
      }
      Action::Primeg => {
        self.g_primed = true;
      },
      Action::FilterListAction(fa) => {
        match fa {
            crate::action::FilterListAction::CloseList => self.show_filter_screen = false,
            crate::action::FilterListAction::OpenFilterScreen => self.show_filter_screen = true,
            _ => {
              let opt = self.filter_screen.dispatch(action);
              assert_eq!(opt, None);
            },
        }
      }
      Action::TextEntry(_) => {
        if self.show_filter_screen {
          self.filter_screen.dispatch(action);
        } else {
          unimplemented!("invalid text entry time");
        }
      }
      _ => {},
    }
    if action != Action::Tick {
      self.update_view();
    }
    None
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

    // let s = get_visible_lines(self.mmap.as_bstr(), vec!(), rect.height*10, rect.width*10);
    // let s = s.join("\n");
    // info!("{}", s);
    // let os = format!("Press j or k to increment or decrement.\n\nCounter: {}", self.counter);
    // let s = self.get_view(rect);
    let s: String = self.view.iter().map(|l| l.line.clone()).intersperse("\n".to_string()).collect();
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
