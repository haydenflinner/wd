use std::{
    borrow::Cow,
    cmp::{max, min, Ordering},
    str::pattern::{Pattern, Searcher},
    sync::Arc,
};

use crate::{action::Direction, dateparser::datetime::Parse, drainrs::DrainState};
use bstr::{BStr, ByteSlice};
use chrono::{DateTime, Duration, Local, NaiveDate, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use log::{debug, info, warn};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use regex::Regex;
// use tracing::debug;
use memmap::Mmap;
use tracing::error;
use tui_textarea::TextArea;
// use tracing::info;

use super::{
    filter::{line_allowed, LineFilterResult},
    filter_screen::FilterScreen,
    go_screen::GoScreen,
    logger::Logger,
    text_entry::TextEntry,
    Component, Frame,
};
use crate::action::{Action, CursorMove, FilterListAction, FilterType, LineFilter};

// TODO:
// 7. Search interruption via CTRL+C or asyncness.
// 8. 'h' highlight menu, like search but sticks around
// 5. Search-caching
//     Probably a list of all search results ever shown ought to be fine, no human will view enough results to exhaust memory.
//     This may be implemented as a map of line numbers to the filters which matched on that line, to facilitate quick enable/disable of filters.
// 9. Autoskip based on drain algorithm
// 10. Autohighlight like the newfound tool, things like IPs, numbers, WARN, etc.
// DONE
//  1. Add j and k for scroll down and up.#
//  2. Complete filter in/out
//  3. Add search (/)
//  4. PGUP/DOWN
// 6. go-to by pressing g to bring up go menu. g again auto goes to beginning, everything else gets typed into a text box for goto purposes.
//
// wd's goal is to use minimal resources at all times. Only do what is asked.
// That means no loading the whole file into memory an indexing unless the user asks for it
//   (and probably with setting io priorities to low when we do it!)
// And the computational part of impl is primarily a single non-async thread at the moment.
//
// wd's internal model processes the log file in the following sequence:
//   file mmap -- raw bytes of the underlying file, mmapped into memory
//     |--> record parsing -- a log is a series of records, which are basically new-line delimited strings (with not-permanently-specified-exceptions)
//       |--> record filtering -- filter these records based on their contents and user-provided in/out requirements
//         |--> display -- apply highlighting/custom formatting and then line wrapping for display to the screen.
//
//     |--> search -- right now, uses stdlib searching since we only support raw strings. to support regex, use ripgrep.
//             luckily, most desired regexes so far have been trivially represented as a combo of simple filters.
//             Note that upcoming Drain integration will likely also reduce the need for regex.
//
// There are thus a few levels of abstraction that any given feature could operate at.
// For example, pressing 'j'  operates basically at the display level, since we must know where lines wrapped for
// display to know what the user meant by scrolling to the next line.
// Or, searching works mostly at the file-level, to avoid unnecessary performance overhead of record parsing/filtering.
// To some extent, these things relate to each other though, since once we have parsed enough of the file for display,
// there is no need to continue parsing/filtering.
//
// State stored is:
// mmap - what it says on the tin.
// record parsing -- cache all previously recognized record locations [start_byte, end_byte)
// record filtering -- we should cache the records matches to filters, but we don't.
// display -- represented as a list of displayed lines.
// TODO add coloring and highlighting.
// fn fmt_visible_lines(lines: Vec<Line>) -> Vec<Line> { lines }

struct Screen {
    pub view: Vec<DispLine>,
    /// Used for PGDOWN/UP.
    pub screen_size: Rect,
}

impl Screen {
    pub fn new(screen_size: Rect) -> Self {
        Self {
            view: Vec::new(),
            screen_size,
        }
    }

    pub fn prepend_line(&mut self, line: DispLine) {
        self.view.pop();
        self.view.insert(0, line);
    }

    // This line must already be highlighted. TODO use type system for that.
    pub fn push_line(&mut self, line: DispLine) {
        // TODO don't always remove the first line, see what happens when press G.
        self.view.remove(0); // Technically this causes a shift of the vector but I don't care at the moment :-)
        self.view.push(line);
    }
}

pub fn highlight_line(line: &mut DispLine, needle: &str) {
    if needle.is_empty() {
        return;
    }
    let orig_line = line.line.clone();
    let line_txt = orig_line.spans[0].content.to_owned();
    let mut searcher = needle.into_searcher(&line_txt);
    let mut spans = Vec::new();
    let mut last_plain = 0;
    while let Some((start, end)) = searcher.next_match() {
        // let spans
        spans.push(Span {
            content: line_txt[last_plain..start].to_owned().into(),
            style: Style::default(),
        });
        spans.push(Span {
            content: line_txt[start..end].to_owned().into(),
            style: Style::default().bg(Color::LightGreen),
        });
        last_plain = end;
    }
    if last_plain != line_txt.len() {
        spans.push(Span {
            content: line_txt[last_plain..].to_owned().into(),
            style: Style::default(),
        });
    }
    line.line = Line {
        alignment: None,
        spans,
    };
}

pub fn highlight_lines(lines: &mut [DispLine], needle: &str) {
    if needle.is_empty() {
        return;
    }
    for line in lines {
        highlight_line(line, needle);
    }
}

pub fn get_visible_lines(
    source: &BStr,
    filters: &Vec<LineFilter>,
    rows: u16,
    cols: u16,
    offset_into_big: usize,
) -> Vec<DispLine> {
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
    let mut in_bad_record = false;
    let mut maybe_add_line = |lines: &mut Vec<DispLine>,
                              ending_index: usize,
                              displayed_rows: &mut u16,
                              rows_for_this_line: &u16,
                              line_start: usize| {
        let line = source[line_start..ending_index].to_str_lossy().into_owned();
        // TODO Do we need to allow IN filters which match part of a record to display the whole record?
        // IMO no, you can add a new in filter for the line you're interested in, with higher priority.
        let is_new_record = {
            if line_start != ending_index {
                let first_char = source[line_start];
                first_char != b'\t' && first_char != b' '
            } else {
                false
            }
        };
        if is_new_record {
            in_bad_record = false;
        }
        let matches = line_allowed(filters, &line);
        let should_print = if in_bad_record {
            matches.1 == LineFilterResult::Include // Must have exactly matched an include line if it's a part of an otherwise filtered record.
        } else {
            matches.0 // Otherwise fallback to the general rules for filters.
        };
        if should_print {
            lines.push(DispLine {
                file_loc: (offset_into_big + line_start, offset_into_big + ending_index),
                line: line.into(),
            });
            *displayed_rows += rows_for_this_line + 1;
        } else {
            in_bad_record = true;
        }
        // return false;
    };
    // We need some concept of 'records' rather than lines so that we can filter multi-line log msgs easily.
    // A good generic (if not fast) approach seems to be a newline followed by a timestamp (since we currently assume that timestamps begin the line)
    // Right now parsing a timestamp is kinda slow (50usec) but this can be vastly improved by remembering what format worked last time and using that first (or exclusively).
    // Let's give it a shot and see how useable this approach is. Keep in mind that we only need to fill the screen.
    // As another approximation, we could say that indented lines following a true-start are part of the previous record, as are lines that look like '\n}' or '\n]'.
    // This approach works for me today without having to attempt parsing timestamps over and over :-) We would anyways have checked 'does this line start with a number'
    // before continuing to attempt timestamp parsing.
    // let b = source.as_bytes();
    // Assumes always linewrap, one byte == one visible width char.
    for (start, end, c) in source.char_indices() {
        match c {
            '\n' => {
                maybe_add_line(
                    &mut lines,
                    start,
                    &mut displayed_rows,
                    &rows_for_this_line,
                    line_start,
                );
                line_start = end;
                rows_for_this_line = 0;
                used_cols = 0;
                if displayed_rows == rows {
                    return lines;
                }
            }
            _ => {
                used_cols += 1;
                if used_cols == cols {
                    // No need to complete out the line atm, TODO.
                    // Assumption, user will be filtering on something that at least fits in the screen when scrolling by.
                    rows_for_this_line += 1;
                    used_cols = 0;
                    // Note a line which spans multiple screens will not be fully excluded if the thing we're filtering
                    // doesn't fit on-screen.
                    if displayed_rows + rows_for_this_line == rows {
                        maybe_add_line(
                            &mut lines,
                            end,
                            &mut displayed_rows,
                            &rows_for_this_line,
                            line_start,
                        );
                        return lines;
                    }
                }
            }
        }
    }
    // if used_cols > 0 {
    // TODO Skip the newline if it terminates this line?
    maybe_add_line(
        &mut lines,
        source.len(),
        &mut displayed_rows,
        &rows_for_this_line,
        line_start,
    );
    // }
    lines
}

/// Find first character of line starting at or before byte_offset.
fn find_line_starting_before(s: &[u8], byte_offset: usize) -> usize {
    // mmap[0..byte_offset].iter().rev().find('\n').unwrap_or(0)
    let bytes_before_offset_of_newline = s[..byte_offset] // or s[0..byte_offset]
        .iter()
        .rev()
        .enumerate()
        .find_map(|(index, val)| match val {
            val if *val == b'\n' => Some(index),
            _ => None,
        })
        .unwrap_or(byte_offset);
    // 4096 -> 3486 should find byte 10. TODO Unit test.
    let idx = byte_offset - bytes_before_offset_of_newline;
    assert!(idx == 0 || s[idx - 1] == b'\n');
    idx
}

fn find_start_line_pct(mmap: &Mmap, pct: f64) -> usize {
    let pct = f64::min(f64::max(pct, 0.0), 100.0);
    let going_to = (mmap.len() as f64 * (pct / 100.0)).floor() as usize;
    find_line_starting_before(mmap, going_to)
}

fn parse_date_starting_at(
    s: &[u8],
    start_offset: FileOffset,
    default_date: NaiveDate,
) -> Option<DateTime<Utc>> {
    // TODO .. need to use min or is that implicit like Python?
    let s = std::str::from_utf8(&s[start_offset..min(start_offset + 100, s.len())]).ok()?;
    let second_space_idx = s
        .char_indices()
        .filter_map(|(index, char)| match char == ' ' {
            true => Some(index),
            false => None,
        })
        .nth(1)?;
    let s = &s[0..second_space_idx];
    debug!("Parsing: {}", s);
    // match crate::dateparser::parse(std::str::from_utf8(&s).unwrap()) {
    /*match crate::dateparser::parse(s) {
        Ok(dt) => dt,
        _ => panic!(),
    }*/

    // Parse::new(tz, Utc::now().time()).parse(input)
    // TODO set context with this: https://github.com/waltzofpearls/dateparser/issues/39
    crate::dateparser::parse_with_timezone(s, &Local, Some(default_date)).ok()
}

fn find_date_before(
    s: &[u8],
    mut byte_offset: FileOffset,
    default_date: NaiveDate,
) -> Option<(FileOffset, DateTime<Utc>)> {
    let mut lines_try = 1000;
    while lines_try > 0 {
        let line_start = find_line_starting_before(s, byte_offset);
        let maybe_ts = parse_date_starting_at(s, line_start, default_date);
        match maybe_ts {
            Some(ts) => {
                return Some((line_start, ts));
            }
            None => {
                byte_offset = line_start - 1;
                lines_try -= 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;

    use super::*;

    static LINES: &str = "03/22/2022 08:51:06 INFO   :...mylogline
03/22/2022 08:51:08 INFO   :...mylogline";

    #[test]
    fn test_visible() {
        let call = |rows, cols| -> String {
            get_visible_lines("lol".into(), &vec![], rows, cols, 0)
                .iter()
                .map(|l| l.line.spans[0].content.clone().to_owned())
                .intersperse("\n".to_string().into())
                .collect()
        };
        assert_eq!(call(80, 80), "lol");
    }

    #[test]
    fn test_visible1() {
        let call = |rows, cols| -> String {
            // let s: Vec<_> = self.view.iter().map(|dl| dl.line.clone()).collect();
            get_visible_lines(LINES.into(), &vec![], rows, cols, 0)
                .iter()
                .map(|l| l.line.spans[0].content.clone().to_owned())
                .intersperse("\n".to_string().into())
                .collect()
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
            get_visible_lines(s, &vec![], rows, cols, 0)
                .iter()
                .map(|l| l.line.spans[0].content.clone().to_owned())
                .intersperse("\n".to_string().into())
                .collect()
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
        assert!(line_allowed(&vec!(), "Lol").0);
        assert!(
            !line_allowed(
                &vec!(LineFilter::new("Lol".to_string(), FilterType::Out)),
                "Lol"
            )
            .0
        );
        assert!(
            line_allowed(
                &vec!(LineFilter::new("Lol".to_string(), FilterType::In)),
                "Lol"
            )
            .0
        );
        assert!(
            !line_allowed(
                &vec!(
                    LineFilter::new("Lol".to_string(), FilterType::In),
                    LineFilter::new("Lol".to_string(), FilterType::Out),
                ),
                "Lol"
            )
            .0
        );
        assert!(
            line_allowed(
                &vec!(
                    LineFilter::new("Lol".to_string(), FilterType::Out),
                    LineFilter::new("Lol".to_string(), FilterType::In),
                ),
                "Lol"
            )
            .0
        );
    }

    #[test]
    fn test_first() {
        assert_eq!(
            bin_search(
                LINES.as_bytes(),
                &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:06Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn test_dt() {
        // let dt = crate::dateparser::parse_with_timezone("09:42:21.0090980809 WORD", &Local);
        // TODO switch to qsv-dateparser to get support for fractions on the timestamps?
        // Or just hardcode a filter out of them, because we don't need the tenths?
        assert!(crate::dateparser::parse_with_timezone("09:42:21", &Local, None).is_ok());
        assert!(crate::dateparser::parse_with_timezone("09:42:21.401", &Local, None).is_ok());
        assert!(crate::dateparser::parse_with_timezone("09:42:21.99923", &Local, None).is_ok());
    }

    #[test]
    fn test_second() {
        assert_eq!(
            bin_search(
                LINES.as_bytes(),
                &Local
                    .datetime_from_str("2022-03-22T08:51:08", "%Y-%m-%dT%H:%M:%S")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .unwrap(),
            41
        );
    }

    #[test]
    fn test_between() {
        assert_eq!(
            bin_search(
                LINES.as_bytes(),
                &Local
                    .datetime_from_str("2022-03-22T08:51:07", "%Y-%m-%dT%H:%M:%S")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .unwrap(),
            41
        );
    }

    #[test]
    fn test_after() {
        // env_logger::init();
        assert_eq!(
            bin_search(
                LINES.as_bytes(),
                &Local
                    .datetime_from_str("2022-03-22T08:51:09", "%Y-%m-%dT%H:%M:%S")
                    // &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:09Z")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .unwrap(),
            80
        );
    }

    #[test]
    fn test_before() {
        assert_eq!(
            bin_search(
                LINES.as_bytes(),
                &Local
                    .datetime_from_str("2022-03-22T08:51:00", "%Y-%m-%dT%H:%M:%S")
                    .unwrap()
                    .with_timezone(&Utc)
            )
            .unwrap(),
            0
        );
    }

    #[test]
    fn test_re() {
        // let re = Regex::new("[0-9]{3}-[0-9]{3}-[0-9]{4}").unwrap();
        // let mat = re.find("phone: 111-222-3333").unwrap();
        // TODO Use this to highlight within the match. And to search next instance.
        // assert_eq!((mat.start(), mat.end()), (7, 19));
    }
}

type FileOffset = usize;

#[derive(Debug)]
enum TsBinSearchError {
    FailedParseTs,
}

// fn bin_search_file(s: &[u8], dt: &DateTime<Utc>) {
fn bin_search(s: &[u8], dt: &DateTime<Utc>) -> Result<FileOffset, TsBinSearchError> {
    let mut low: usize = 0;
    let mut high: usize = s.len() - 1;
    let mut middle = 0;

    while low <= high {
        middle = (high + low) / 2;
        info!("Bin search: low: {}, mid: {}, high: {}", low, middle, high);
        match find_date_before(s, middle, dt.date_naive()) {
            Some((line_start, ts)) => {
                info!("Comparing tses {} and {}", ts, dt);
                match ts.cmp(dt) {
                    Ordering::Less => {
                        low = middle + 1;
                    }
                    Ordering::Equal => return Ok(line_start), // lucky guess!
                    Ordering::Greater => {
                        if middle == 0 {
                            return Ok(0);
                        }
                        high = middle - 1;
                    }
                }
            }
            None => {
                return Err(TsBinSearchError::FailedParseTs);
            }
        }
    }
    Ok(middle)
}

#[derive(PartialEq, Eq, Clone)]
pub struct DispLine {
    file_loc: (usize, usize), // <-- [begin, end)
    // line: String,
    line: ratatui::text::Line<'static>,
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

    filename: String,
    mmap: Arc<Mmap>,
    byte_cursor: usize,
    today: Option<NaiveDate>,

    go_screen: GoScreen,

    show_filter_screen: bool,
    filter_screen: FilterScreen<'static>,

    show_search: bool,
    search_screen: TextEntry<'static>,
    last_search: String,
    search_visits: Vec<usize>,

    screen: Screen,

    /// Pre-filtered based on each action.
    // view: Vec<Cow<'mmap, str>>,
    // view: Vec<DispLine>,
    // view: Vec<String>,
    /// Used for PGDOWN/UP.
    // screen_size: Rect,
    drain_state: DrainState,
}

impl Home {
    pub fn new(filename: String, mmap: Arc<Mmap>) -> Self {
        Self {
            is_running: false,
            show_logger: false,
            logger: Logger::default(),
            counter: 0,
            filename,
            mmap,
            byte_cursor: 0,
            today: None,
            show_filter_screen: false,
            filter_screen: FilterScreen::default(),
            go_screen: GoScreen::default(),
            show_search: false,
            search_screen: TextEntry::default(),
            last_search: String::default(),
            search_visits: Vec::default(),
            screen: Screen::new(Rect {
                x: 0,
                y: 0,
                width: 1000,
                height: 1000,
            }),
            drain_state: DrainState::new(),
        }
    }

    pub fn tick(&self) {
        // debug!("Tick");
    }

    // This is a copy paste of next_line but in the opposite direction;
    // it did not seem necessarily simpler to use one impl that constantly switched on direction.
    pub fn prev_line(&mut self) {
        let first_line = self.screen.view.first();
        let first_line = match first_line {
            Some(first_line) => first_line,
            None => {
                info!("Tried to go past beginning of empty file.");
                return;
            }
        };
        // For some reason when scrolling up past the beginning of a file, we end up in a scenario where
        // byte cursor is 1 while file loc is 0. I'm going to disable that ability, but making this assert
        // a little more friendly just in case.
        if first_line.file_loc.0 == 0 {
            // Trying to scroll before beginning of file. I think scrolling before the beginning of lines that are filtered out
            // but not displaying will still scroll depsite this.
            return;
        }
        // assert!(self.byte_cursor == first_line.file_loc.0 || self.byte_cursor - 1 == first_line.file_loc.0);

        // A naive copy paste of next_line to here results in prev_line moving back one file-line and not one visible line.
        // We got lucky with next_line.
        // We have a conundrum here, which is that seeking backwards doesn't have a parallel/inverse with forwards in our lowest function calls
        // For example, get_visible_lines seeks forward rather than backwards, but expects to be started on the start of a line.
        // some of this is just reality; for example our filters must be applied to a 'line' (or a 'record'!), not just an arbitrary byte stream.
        //
        // There are two obvious approaches in front of us.
        //   1. Scroll back file-line by file-line until a line is visible.
        //   2. Write a get_visible_lines_backwards.
        // get_visible_lines is kind of a big gnarly function that I don't want to go near right now,
        // while (1) seems straightforward.
        // One problem lurking is that approach (1) done wrong will approach n^2 runtime with high-percentage filtering.
        // We can avoid this by only asking it to review the freshly under-consideration file-line.
        // We thus treat get_visible_lines as a sort of single-line filter. Perhaps it should be refactored to lift a single-line filter out of it.
        // Also note by taking approach 1, we are totally ignoring our prior concept of 'records', which results in the following bug:
        // When scrolling forward, the lines 'filtered-out-string\n  continued\n  continued' would all be filtered, but when scrolling backwards, not so.
        // The fix for this is to implement something like a RecordIterator, and break get_visible_lines into more distinguishable pieces.
        // For now, this gets the job mostly done, at least it makes scrolling up responsive in the usual simple cases.
        let mut end_search = first_line.file_loc.0;
        loop {
            if self.byte_cursor == 0 {
                return;
            }
            self.byte_cursor =
                find_line_starting_before(&self.mmap, self.byte_cursor.saturating_sub(1));
            let prev_line_starts_at = self.byte_cursor;
            // line_allowed(&self.filter_screen.items, line) 0-thought went into not using this
            let binding = get_visible_lines(
                self.mmap[prev_line_starts_at..end_search].as_bstr(),
                &self.filter_screen.items,
                1,
                600,
                prev_line_starts_at,
            );
            assert!(binding.len() == 1);
            let prev_line = binding.into_iter().next();
            if let Some(mut prev_line) = prev_line {
                // Found a previous line that is actually visible.
                highlight_line(&mut prev_line, &self.last_search);
                self.screen.prepend_line(prev_line);
                return;
            }
            end_search = prev_line_starts_at;
        }
    }

    /// Unfortunately doesn't account for the word-wrapping that ratatui does,
    /// so will skip whole lines sometimes rather than just a screen-line.
    pub fn next_line(&mut self) {
        let last_line = self.screen.view.last();
        let last_line = match last_line {
            Some(last_line) => last_line,
            None => {
                info!("Tried to go past end of visible file!");
                return;
            }
        };
        self.byte_cursor = self.screen.view[0].file_loc.1 + 1;
        if self.byte_cursor >= self.mmap.len() {
            self.byte_cursor = self.mmap.len() - 1;
            info!("Tried to go past end of file!");
        }

        // TODO document some invariants on these values. Do they point at the newline? One before? etc.
        // Intent is for [start, end), i.e. end index points one past the last valid index.
        let next_line_starts_at = last_line.file_loc.1; // + 1;
        let next_lines = get_visible_lines(
            self.mmap[next_line_starts_at..].as_bstr(),
            &self.filter_screen.items,
            1,
            600,
            next_line_starts_at,
        );
        let next_lines_len = next_lines.len();
        let first = next_lines.into_iter().next();
        if first == None {
            return;
        }
        assert!(next_lines_len == 1);
        let mut first = first.unwrap();
        highlight_line(&mut first, &self.last_search);
        self.screen.push_line(first);
        // info!("Set cursor to {}", self.byte_cursor);
    }

    pub fn goto_dt(&mut self, dt: DateTime<Utc>) {
        // If we successfully parsed a
        /*if let Some(file_today) = self.today {
            let secs_off = (dt.date_naive() - file_today).num_seconds();
            match Utc.timestamp_opt(dt.timestamp() - secs_off, 0) {
                chrono::LocalResult::None => {}
                chrono::LocalResult::Single(ts) => dt = ts,
                chrono::LocalResult::Ambiguous(_, _) => {}
            }
        }*/
        let spot = bin_search(self.mmap.as_bstr(), &dt);
        match spot {
            Ok(cursor) => self.byte_cursor = cursor,
            Err(tbe) => {
                error!("{:?}", tbe);
                // spot.unwrap();
            }
        }
    }

    pub fn goto_pct(&mut self, pct: f64) {
        self.byte_cursor = find_start_line_pct(&self.mmap, pct);
    }

    pub fn goto_begin(&mut self) {
        self.byte_cursor = 0;
    }

    pub fn goto_end(&mut self) {
        // TODO Just set cursor to mmap.len()? Does cursor really need to be at beginning of valid line always?
        self.byte_cursor = find_line_starting_before(&self.mmap, self.mmap.len());
    }

    pub fn put_cursor_on_line_search(&mut self, needle: &str) {
        let mut cursor = self.byte_cursor;
        let mut last_cursor = usize::MAX;
        info!("Search starting at {:?}", cursor);
        loop {
            let haystack = &self.mmap[self.byte_cursor..];
            let result = haystack.find(needle);
            self.last_search = needle.to_owned();
            if cursor == last_cursor {
                return;
            }
            last_cursor = cursor;
            match result {
                Some(idx) => {
                    let maybe_cursor = self.byte_cursor + find_line_starting_before(haystack, idx);
                    if !line_allowed(
                        &self.filter_screen.items,
                        // TODO Explicitly search to the next newline only
                        &self.mmap[maybe_cursor..self.mmap.len().max(maybe_cursor + 4096)]
                            .to_str_lossy(),
                    )
                    .0
                    {
                        info!("Skipping line starting at {} due to filter.", maybe_cursor);
                        cursor = maybe_cursor;
                        continue;
                    }
                    self.byte_cursor = maybe_cursor;
                    self.search_visits.push(self.byte_cursor);
                    info!("Found line starting at {}", maybe_cursor);
                    return;
                    // https://github.com/rhysd/tui-textarea/blob/main/src/highlight.rs#L101
                    // Great reference for using Spans to highlight lines.
                    // self.search_screen.textarea.input().th
                    // self.search_screen.textarea.set_search_pattern(needle);
                }
                None => {
                    info!("Nothing found with term: {:?}", needle);
                    return;
                }
            }
        }
    }

    pub fn new_search(&mut self) {
        let needle = &self.search_screen.textarea.lines()[0].clone();
        self.search_screen.textarea.delete_line_by_head();
        self.put_cursor_on_line_search(needle);
        // TODO Store .show within the textentry, and move cursor to end before delete (in case press enter midway thru line)
    }

    pub fn repeat_search(&mut self, direction: crate::action::Direction) {
        match direction {
            crate::action::Direction::Next => {
                self.next_line(); // TODO go nowhere if search doesn't find anything.
                let needle = self.last_search.clone();
                self.put_cursor_on_line_search(&needle);
            }
            crate::action::Direction::Prev => {
                // In less, pressing N runs the search bachwards. This can be quite slow.
                // There is probably a fast way to do it by searching 4k pages at a time
                // and only breaking down into lines after finding a match.
                // For now, let's just let N only go backwards through already visited searches.
                // Would be nice to implement ? search too, probably.
                // TODO this .pop is destructive because then pressing n requires re-searching.
                // best to fix this by replacing the dumb list of results with intelligent caching for
                // 'user searched this term while in this byte range.'
                match self.search_visits.pop() {
                    Some(idx) => self.byte_cursor = idx,
                    None => info!("Can't go back, reverse serach not yet supported!"),
                }
            }
        }
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
        self.screen.view = get_visible_lines(
            self.mmap[self.byte_cursor..].as_bstr(),
            &self.filter_screen.items,
            200,
            600,
            self.byte_cursor,
        );
        highlight_lines(&mut self.screen.view, &self.last_search);
    }

    fn parse_filename_for_date(&self, filename: &str) -> Option<NaiveDate> {
        let re = Regex::new(r"\b\d{8}\b").unwrap();

        for capture in re.captures_iter(filename) {
            let s = capture.get(0).unwrap().as_str();
            match NaiveDate::parse_from_str(s, "%Y%m%d") {
                Ok(nd) => return Some(nd),
                Err(_) => {}
            };
        }
        None
    }

    fn move_screenful(&mut self, dir: Direction) {
        let h = self.screen.screen_size.height;
        for _ in 0..h + 1 {
            match dir {
                Direction::Prev => self.prev_line(),
                Direction::Next => self.next_line(),
            }
        }
    }
}

impl Component for Home {
    fn init(&mut self) -> anyhow::Result<()> {
        self.is_running = true;
        self.update_view();
        let byte_offset = find_line_starting_before(self.mmap.as_bstr(), self.mmap.len() - 1);
        let default_date = self.parse_filename_for_date(&self.filename);
        let maybe_ts = find_date_before(
            self.mmap.as_bstr(),
            byte_offset,
            default_date.unwrap_or(Local::now().date_naive()),
        );
        self.today = match maybe_ts {
            Some((_, ts)) => Some(ts.date_naive()),
            None => default_date,
        };
        self.go_screen.set_today(self.today);
        self.go_screen.init()
    }

    fn on_key_event(&self, key: KeyEvent) -> Action {
        if self.show_filter_screen {
            let caught = self.filter_screen.on_key_event(key);
            if caught != Action::Tick {
                return caught;
            }
        }
        if self.show_search {
            let caught = self.search_screen.on_key_event(key);
            if caught != Action::Tick {
                return caught;
            }
        }
        if self.go_screen.show {
            // if let Action::TextEntry(KeyEvent { code: KeyCode::Char('g'), modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: KeyEventState::NONE }) = action {
            // TODO Let 'g' be handled by the go screen so it can close itself.
            if key.code == KeyCode::Char('g') {
                return Action::CursorMove(CursorMove::End(crate::action::Direction::Prev));
            }
            let caught = self.go_screen.on_key_event(key);
            if caught != Action::Tick {
                return caught;
            }
        }
        match key.code {
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('l') => Action::ToggleShowLogger,
            KeyCode::Char('g') => match self.go_screen.show {
                true => Action::CursorMove(CursorMove::End(crate::action::Direction::Prev)),
                false => Action::OpenGoScreen,
            },
            KeyCode::Char('G') => {
                Action::CursorMove(CursorMove::End(crate::action::Direction::Next))
            }
            KeyCode::Char('j') => {
                Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Next))
            }
            KeyCode::Char('k') => {
                Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Prev))
            }
            KeyCode::PageUp => {
                Action::CursorMove(CursorMove::Screenful(crate::action::Direction::Prev))
            }
            KeyCode::PageDown => {
                Action::CursorMove(CursorMove::Screenful(crate::action::Direction::Next))
            }
            KeyCode::Down => {
                Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Next))
            }
            KeyCode::Up => Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Prev)),
            KeyCode::Char('/') => Action::BeginSearch,
            KeyCode::Char('n') => Action::RepeatSearch(crate::action::Direction::Next),
            KeyCode::Char('N') => Action::RepeatSearch(crate::action::Direction::Prev),
            KeyCode::Char('f') => Action::FilterListAction(FilterListAction::OpenFilterScreen),
            _ => Action::Tick,
        }
    }

    fn dispatch(&mut self, action: Action) -> Option<Action> {
        let mut followup_action = None;
        if self.go_screen.show {
            if action == Action::CursorMove(CursorMove::End(crate::action::Direction::Prev)) {
                self.go_screen
                    .dispatch(Action::FilterListAction(FilterListAction::CloseNew));
                self.goto_begin();
                self.update_view();
                return None;
            }
            let opt = self.go_screen.dispatch(action);
            return opt;
        }
        match action {
            Action::Quit => self.is_running = false,
            Action::Tick => self.tick(),
            Action::ToggleShowLogger => self.show_logger = !self.show_logger,
            Action::CursorMove(cm) => match cm {
                CursorMove::OneLine(dir) => match dir {
                    crate::action::Direction::Prev => self.prev_line(),
                    crate::action::Direction::Next => self.next_line(),
                },
                CursorMove::End(dir) => match dir {
                    crate::action::Direction::Prev => self.goto_begin(),
                    crate::action::Direction::Next => self.goto_end(),
                },
                CursorMove::Timestamp(ts) => {
                    self.goto_dt(ts);
                }
                CursorMove::Percentage(pct) => {
                    self.goto_pct(pct);
                }
                CursorMove::Screenful(dir) => {
                    self.move_screenful(dir);
                }
            },
            Action::OpenGoScreen => {
                self.go_screen.show = true;
            }
            Action::FilterListAction(fa) => {
                match fa {
                    crate::action::FilterListAction::CloseList => self.show_filter_screen = false,
                    crate::action::FilterListAction::OpenFilterScreen => {
                        self.show_filter_screen = true
                    }
                    _ => {
                        let opt = self.filter_screen.dispatch(action);
                        followup_action = opt;
                        // assert_eq!(opt, None);
                    }
                }
            }
            Action::TextEntry(_) => {
                if self.show_filter_screen {
                    self.filter_screen.dispatch(action);
                } else if self.show_search {
                    self.search_screen.dispatch(action);
                } else if self.go_screen.show {
                    self.go_screen.dispatch(action);
                } else {
                    unimplemented!("invalid text entry time");
                }
            }
            Action::Resize(_, _) => {}
            Action::OpenTextEntry => {}
            Action::CloseTextEntry => {
                assert!(self.show_search);
                self.show_search = false;
            }
            Action::ConfirmTextEntry => {
                assert!(self.show_search);
                self.show_search = false;
                self.new_search()
            }
            Action::BeginSearch => {
                self.show_search = true;
            }
            Action::RepeatSearch(dir) => {
                self.repeat_search(dir);
            }
            Action::Noop => {} // _ => {},
        }
        // Hardcoded list of actions which don't require a full redo:
        if action != Action::Tick
            && action != Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Next))
            && action != Action::CursorMove(CursorMove::OneLine(crate::action::Direction::Prev))
        {
            self.update_view();
        }
        followup_action
    }

    fn render(&mut self, f: &mut Frame<'_>, rect: Rect) {
        self.screen.screen_size = rect;

        let rect = if self.show_logger {
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Percentage(10), Constraint::Percentage(90)])
                .split(rect);
            self.logger.render(f, chunks[1]);
            chunks[0]
        } else {
            rect
        };

        let rect = if self.show_filter_screen {
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rect);
            self.filter_screen.render(f, chunks[1]);
            chunks[0]
        } else {
            rect
        };

        let rect = if self.show_search {
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Max(3)])
                .split(rect);
            let block = Block::default()
                .title("Search: (enter) Submit (esc) Cancel")
                .borders(Borders::all());
            self.search_screen.textarea.set_block(block);
            self.search_screen.render(f, chunks[1]);
            chunks[0]
        } else {
            rect
        };

        let rect = if self.go_screen.show {
            let chunks = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([Constraint::Min(3), Constraint::Max(3)])
                .split(rect);
            self.go_screen.render(f, chunks[1]);
            chunks[0]
        } else {
            rect
        };

        let s: Vec<_> = self.screen.view.iter().map(|dl| dl.line.clone()).collect();
        f.render_widget(
            Paragraph::new(s)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false })
                .style(Style::default()),
            rect,
        )
    }
}
