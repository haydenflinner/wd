#![allow(unused_imports)]
#![allow(dead_code)]

use cursive::event::Event;
use cursive::event::EventResult;
use cursive::menu;
use cursive::reexports::time::Time;

use cursive::view::Nameable;
use cursive::views::TextView;
use cursive::views::*;
use cursive::Cursive;
use cursive::With;

use chrono::prelude::*;
use log::debug;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::error::Error;
use std::rc::Rc;

use ansi_term::Colour::Purple;
use memmap::Mmap;
use memmap::MmapOptions;
use std::cmp::min;
use std::fs::File;
use unicode_width::UnicodeWidthStr; // To get the width of some text.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LineNo(i64);

#[cfg(not(test))] 
use log::{info, warn}; // Use log crate when building application
 
#[cfg(test)]
use std::{println as info, println as warn}; // Workaround to use prinltn! for logs.


struct Page {
    start_time: DateTime<Utc>,
}

fn find_start_line_pct(mmap: &Mmap, pct: usize) -> usize {
    if pct == 0 {
        return 0;
    }
    find_line_starting_before(mmap, mmap.len() / pct)
}

const PG_SIZE: usize = 4096;
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct PageNo(pub usize);

impl PageNo {
    pub fn new(byte_offset: usize) -> Self {
        Self(byte_offset / PG_SIZE)
    }
    fn as_byte_offset(&self) -> usize {
        self.0 * PG_SIZE
    }
}

type PageTable = BTreeMap<PageNo, Page>;

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

fn parse_date_starting_at(s: &[u8], start_offset: usize) -> Option<DateTime<Utc>> {
    // TODO .. need to use min or is that implicit like Python?
    let s = std::str::from_utf8(&s[start_offset..min(start_offset + 100, s.len())]).unwrap();
    let second_space_idx = s
        .char_indices()
        .filter_map(|(index, char)| match char == ' ' {
            true => Some(index),
            false => None,
        })
        .nth(1)
        .unwrap();
    let s = &s[0..second_space_idx];
    debug!("Parsing: {}", s);
    // match dateparser::parse(std::str::from_utf8(&s).unwrap()) {
    /*match dateparser::parse(s) {
        Ok(dt) => dt,
        _ => panic!(),
    }*/
    dateparser::parse_with_timezone(s, &Utc).ok()
}

fn init_page_for_offset(page_table: &mut PageTable, mmap: &Mmap, offset: usize) {
    // let offset = offset - (offset % PG_SIZE);
    let pgno = PageNo::new(offset);
    if page_table.contains_key(&pgno) {
        return;
    }
    let start_offset = find_line_starting_before(mmap, pgno.as_byte_offset());
    page_table.insert(
        pgno,
        Page {
            start_time: parse_date_starting_at(mmap, start_offset).expect("Should be able to parse datetime starting line"),
        },
    );
}

/*
fn get_view(mmap: &Mmap, offset: usize) {
    //

}
*/

struct App {
    mmap: Mmap,
    /// current byte offset into mmapped file.
    cursor: usize,
}

type FileOffset = usize;

impl App {
    fn get_view(&self) -> &str {
        // 4kb, TODO Could minimize this by knowing size of terminal+rows returned.
        std::str::from_utf8(&self.mmap[self.cursor..min(PG_SIZE * 4, self.mmap.len())]).unwrap()
    }

    fn next_line(&mut self) {
        // let bytes_til_newline = &mmap[cursor..cursor.PG_SIZE*4]
        let bytes_til_newline = self
            .mmap[self.cursor..self.mmap.len()]
            .iter()
            .enumerate()
            .find_map(|(index, val)| match val {
                val if *val == ('\n' as u8) => Some(index),
                _ => None,
            })
            .unwrap_or(0);
        // assert!(self.mmap[byte_til_newline])
        let byte_after_newline = min(self.mmap.len(), self.cursor + bytes_til_newline + 1);
        self.cursor = byte_after_newline;
    }

    fn prev_line(&mut self) {
        self.cursor = find_line_starting_before(&self.mmap, self.cursor.saturating_sub(1));
    }

    fn goto_begin(&mut self) {
        self.cursor = 0;
    }

    fn goto_end(&mut self) {
        // TODO Just set cursor to mmap.len()? Does cursor really need to be at beginning of valid line always?
        self.cursor = find_line_starting_before(&self.mmap, self.mmap.len());
    }

    fn goto(&mut self, line_idx: usize) {
        self.cursor = find_line_starting_before(&self.mmap, line_idx);
    }
}

struct NavHandler<'a> {
    mmap: &'a Mmap,
    pt: PageTable,
}

impl NavHandler<'_> {
    fn new(mmap: &Mmap) -> NavHandler {
        let mut nh = NavHandler {
            mmap,
            pt: PageTable::new(),
        };
        load_initial_pages(&mut nh.pt, mmap);
        nh
    }

    fn goto(self, spec: &str) {
        todo!();
    }

    fn goto_time(self, spec: &DateTime<Utc>) {
        // Walk the page table, populating entries as needed to find the right page.
        // Better abstractino would probably be a pagetable holder that populates itself.
        // Perhaps a vector of PageInfos, and just use stdlib binary search?
        // This is dumb, and a better approach would be to do binary search, and where we see a gap between two pages,
        // (a page that we haven't checked the timestamp of), go and fetch that page and find ts, effectively a lazy search through the text.
        // Unfortunately we can't take mutable reference to self in cmp/partialcmp, fair enough.
        // What we can do is, given the start and end times of each page, look for the given ts.
        // If we don't find it, then we can add a few missing pages into the page table to look for it.
        // If

        // Assume we know nothing. What we must do is binary search through the text file, that's not that bad.
        // Then, to save this work from being done all of the time (even though it's more or less intant, it's not quite interactive)
        // Cache the start/end times of each page that we loaded in the process of finding this timestamp, for future finding.

        // TODO Check pt cache here. Store the spot that we should insert page table entries that we find.
        // Or maybe even a starting point, although frankly it's pointless.
        let spot = bin_search(self.mmap, spec);
    }
}

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
        match find_line_starting_before(&s, middle) {
            // i if i <= low => return Ok(i), // Previous line begins before our search area.
            line_i => {
                let ts = parse_date_starting_at(s, line_i);
                match ts {
                    Some(ts) => {
                        info!("Comparing tses {} and {}", ts, dt);
                        match ts.cmp(dt) {
                            Ordering::Less => low = middle + 1,
                            Ordering::Equal => return Ok(line_i), // lucky guess!
                            Ordering::Greater => {
                                if middle == 0 {
                                    return Ok(0);
                                }
                                high = middle - 1
                            }
                        }
                    }
                    None => return Err(TsBinSearchError::FailedParseTs), // TODO head to previous line and look for timestamp!
                }
            }
        }
    }
    return Ok(middle);
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
        static LINES: &str = "03/22/2022 08:51:06 INFO   :...mylogline
03/22/2022 08:51:08 INFO   :...mylogline";
 
    #[test]
    fn test_first() {
        assert_eq!(bin_search(LINES.as_bytes(), 
            &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:06Z").unwrap().with_timezone(&Utc)).unwrap(),
        0);
    }

    #[test]
    fn test_second() {
        assert_eq!(bin_search(LINES.as_bytes(), 
            &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:08Z").unwrap().with_timezone(&Utc)).unwrap(),
        41);
    }

    #[test]
    fn test_between() {
        assert_eq!(bin_search(LINES.as_bytes(), 
            &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:07Z").unwrap().with_timezone(&Utc)).unwrap(),
        41);
    }

    #[test]
    fn test_after() {
        assert_eq!(bin_search(LINES.as_bytes(), 
            &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:09Z").unwrap().with_timezone(&Utc)).unwrap(),
        80);
    }

    #[test]
    fn test_before() {
        assert_eq!(bin_search(LINES.as_bytes(), 
            &DateTime::<FixedOffset>::parse_from_rfc3339("2022-03-22T08:51:00Z").unwrap().with_timezone(&Utc)).unwrap(),
        0);
    }
}


/*/
#[derive(Default)]
struct PageTimeInfo {
    start_time: Optional<DateTime<Utc>>,
}
*/

fn load_initial_pages(page_table: &mut PageTable, mmap: &Mmap) {
    init_page_for_offset(page_table, mmap, 0);
    init_page_for_offset(page_table, mmap, mmap.len());
}

fn main() -> Result<(), Box<dyn Error>> {
    let _cursive = Cursive::new();

    let filename = std::env::args().nth(1).unwrap();

    let file = File::open(filename).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    // let mut cursor: usize = 0;
    // let mut app = App{ mmap: &mmap, cursor: 0};
    let app = Rc::new(RefCell::new(App { mmap: mmap, cursor: 0}));
    // assert_eq!(b"# memmap", &mmap[0..8]);

    // Initialize the cursive logger.
    cursive::logger::init();
    // let mut page_table = BTreeMap::new();
    // load_initial_pages(&mut page_table, mmap);

    let mut siv = cursive::default();

    siv.add_global_callback('q', |s| s.quit());
    let text = "tv";

    let j_app = app.clone();
    let b_app = app.clone();
    let a_app = app.clone();
    let g_app = app.clone();
    let G_app = app.clone();

    // https://github.com/gyscos/cursive/blob/main/doc/tutorial_3.md
    // Maybe can use with_user_data now! Write a tutorial_4 for this once done?
    // Next steps:
    //   0. Fix out the UI that I want, then can hook it up to functionality later?
    //   1. menu for go-to-time
    //   2. Out-filters with menu to edit them. See lnav for fast impl;
    //      Guessing we can just only apply the filters to the screen view;
    //      TODO this will go in app and require support for no results!
    //   2. someday, handle wraparound / running multiple days.
    //   3. Time view, i.e. 
    //   4. "." to repeat last cmd
    //   5. Named regex capture => color-coding / ts parsing
    //   6. Multi-line log entries, or just skip lines that we can't parse timestamps from.

    siv.menubar()
        .add_subtree(
            "File",
            menu::Tree::new()
                .leaf("New", |s| {
                    s.add_layer(Dialog::info("New file screen!"));
                })
        );
    siv.set_autohide_menu(false);
    siv.add_fullscreen_layer(
        TextView::new(a_app.borrow().get_view().clone()).with_name(text)
        .wrap_with(OnEventView::new)
        .on_event(Event::Char('j'), move|siv| {
            j_app.borrow_mut().next_line();
            siv.call_on_name(text, |t: &mut TextView| { t.set_content(j_app.borrow_mut().get_view()); });
        })
        .on_event(Event::Char('k'), move|siv| {
            b_app.borrow_mut().prev_line();
            siv.call_on_name(text, |t: &mut TextView| { t.set_content(b_app.borrow_mut().get_view()); });
        })
        .on_event(Event::Char('g'), move|siv| {
            g_app.borrow_mut().goto_begin();
            siv.call_on_name(text, |t: &mut TextView| { t.set_content(g_app.borrow_mut().get_view()); });
        })
        .on_event(Event::Char('G'), move|siv| {
            G_app.borrow_mut().goto_end();
            siv.call_on_name(text, |t: &mut TextView| { t.set_content(G_app.borrow_mut().get_view()); });
        })
        .on_event(Event::Char('h'), |siv| {
            siv.add_layer(
                Dialog::around(TextView::new("hello!")).title("lmao") // .with_name("di").get_mut()
                    .button("custom", |s| {s.pop_layer();})
                    .dismiss_button("Done")
                    // .button("Back", |s| s.call_on_name("di", |x| { x.cl})
                    .wrap_with(CircularFocus::new)
                    .wrap_tab());
        })
        // .on_pre_event_inner(Event::Char('x'), |v, c| {
        //     v.set_content(cursive::utils::markup::ansi::parse(
        //         Purple
        //             .paint(std::str::from_utf8(&mmap[..]).unwrap())
        //             .to_string(),
        //     ));
        //     // Some(EventResult::Consumed(None))
        //     return Some(EventResult::with_cb(|cb: &mut Cursive | { cb.with_user_data(
        //         |state: &mut App| { state.next_line(); }
        //     ); }));
        // })
        // .on_pre_event_inner(Event::Char('y'), |v, _| {
        //     v.set_content("hello");
        //     Some(EventResult::Consumed(None))
        // }),
    );
    // let state = String::from("hello");
    /* */
    /*
    let state: String = Red.paint("hello").to_string();
    siv.add_fullscreen_layer(
        Canvas::new(state)
            .with_draw(|text: &String, printer| {
                eprintln!("Printing: {}", text);
                printer.print((0, 0), &Red.paint(text).to_string());
            })
            .with_required_size(|text, _constraints| (text.width(), 1).into()),
    );
    */

    siv.run();
    Ok(())
}
