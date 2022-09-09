use cursive::event::Event;
use cursive::event::EventResult;
use cursive::reexports::time::Time;

use cursive::views::TextView;
use cursive::views::*;
use cursive::Cursive;
use cursive::With;

use chrono::prelude::*;
use std::collections::BTreeMap;
use std::error::Error;

use ansi_term::Colour::Purple;
use memmap::Mmap;
use memmap::MmapOptions;
use std::cmp::min;
use std::fs::File;
use unicode_width::UnicodeWidthStr; // To get the width of some text.

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LineNo(i64);

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

fn find_line_starting_before(mmap: &Mmap, byte_offset: usize) -> usize {
    // mmap[0..byte_offset].iter().rev().find('\n').unwrap_or(0)
    let bytes_before_offset_of_newline = mmap[0..byte_offset]
        .iter()
        .rev()
        .enumerate()
        .find_map(|(index, val)| match val {
            val if *val == ('\n' as u8) => Some(index),
            _ => None,
        })
        .unwrap_or(byte_offset);
    return byte_offset - bytes_before_offset_of_newline;
}

fn parse_date_starting_at(s: &[u8], start_offset: usize) -> DateTime<Utc> {
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
    eprintln!("Parsing: {}", s);
    // match dateparser::parse(std::str::from_utf8(&s).unwrap()) {
    match dateparser::parse(s) {
        Ok(dt) => dt,
        _ => panic!(),
    }
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
            start_time: parse_date_starting_at(mmap, start_offset),
        },
    );
}

/*
fn get_view(mmap: &Mmap, offset: usize) {
    //

}
*/

struct App<'a> {
    mmap: &'a Mmap,
    // current byte offset into mmapped file.
    cursor: usize,
}

type FileOffset = usize;

impl App<'_> {
    fn get_view(&self) -> &str {
        std::str::from_utf8(&self.mmap[self.cursor..PG_SIZE * 4]).unwrap() // 4kb, TODO Could minimize this by knowing size of terminal+rows returned.
    }

    fn next_line(&mut self) {
        // let bytes_til_newline = &mmap[cursor..cursor.PG_SIZE*4]
        let bytes_til_newline = self
            .mmap
            .iter()
            .enumerate()
            .find_map(|(index, val)| match val {
                val if *val == ('\n' as u8) => Some(index),
                _ => None,
            })
            .unwrap_or(0);
        let byte_after_newline = min(self.mmap.len(), self.cursor + bytes_til_newline + 1);
        self.cursor = byte_after_newline;
    }

    fn prev_line(&mut self) {
        self.cursor = find_line_starting_before(self.mmap, self.cursor);
    }

    fn goto_begin(&mut self) {
        self.cursor = 0;
    }

    fn goto_end(&mut self) {
        // TODO Just set cursor to mmap.len()? Does cursor really need to be at beginning of valid line always?
        self.cursor = find_line_starting_before(self.mmap, self.mmap.len());
    }
}

struct NavHandler<'a> {
    mmap: &'a Mmap,
    pt: PageTable,
}

impl NavHandler<'_> {
    fn new(mmap: &Mmap) -> NavHandler {
        let nh = NavHandler {
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
        let spot = bin_search_file(self.mmap, spec, s.len() / 2);
    }
}

enum TsBinSearchError {
    FailedParseTs,
}

// fn bin_search_file(s: &[u8], dt: &DateTime<Utc>) {
fn bin_search_file_helper(
    s: &[u8],
    dt: &DateTime<Utc>,
    byte_offset: usize,
) -> Result<FileOffset, TsBinSearchError> {
    let line_start = find_line_starting_before(&s, byte_offset);
    let found_dt = parse_date_starting_at(s, line_start);
    if line_start == 0 {
        return Ok(byte_offset);
    }
    // TODO assumption here that every newline starts with a timestamp.
    // Should be resilient to multiline-logs, maybe search back a whole few mb or so looking for the prev timestamp.
    // Algo: go left/right until we can't anymore, by comparing the byte idx of ideal move point vs prev newline.

    match cmp.compare(dt, found_dt) {
        Less => x,    // Found is after, go left
        Equal => y,   // Wow, what a guess lol
        Greater => z, // Found is before, go right.
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
    let mut cursor: usize = 0;
    // assert_eq!(b"# memmap", &mmap[0..8]);

    // Initialize the cursive logger.
    cursive::logger::init();
    let mut page_table = BTreeMap::new();
    load_initial_pages(&mut page_table, &mmap);

    // TODO Parse first page, parse last page.
    // page_table.insert(0, Page {start_time : NaiveDateTime::new()});

    /*
    // Use some logging macros from the `log` crate.
    log::error!("Something serious probably happened!");
    log::warn!("Or did it?");
    log::debug!("Logger initialized.");
    log::info!("Starting!");
    */

    let mut siv = cursive::default();

    let viewsize = PG_SIZE * 4; // 16kb
                                // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    siv.add_fullscreen_layer(
        TextView::new(cursive::utils::markup::ansi::parse(
            Purple.paint("hello").to_string(),
        ))
        .wrap_with(OnEventView::new)
        .on_pre_event_inner(Event::Char('x'), move |v, _| {
            v.set_content(cursive::utils::markup::ansi::parse(
                Purple
                    .paint(std::str::from_utf8(&mmap[byte_offset..byte_offset + viewsize]).unwrap())
                    .to_string(),
            ));
            Some(EventResult::Consumed(None))
        })
        .on_pre_event_inner(Event::Char('y'), move |v, _| {
            v.set_content("hello");
            Some(EventResult::Consumed(None))
        }),
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
