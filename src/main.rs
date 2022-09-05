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

use ansi_term::Colour::Red;
use memmap::Mmap;
use memmap::MmapOptions;
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

fn init_page_for_offset(page_table: &mut PageTable, mmap: &Mmap, offset: usize) {
    // let offset = offset - (offset % PG_SIZE);
    let pgno = PageNo::new(offset);
    if page_table.contains_key(&pgno) {
        return;
    }
    let start_offset = find_line_starting_before(mmap, pgno.as_byte_offset());
    let s = std::str::from_utf8(&mmap[start_offset..start_offset + 100]).unwrap();
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
        Ok(dt) => page_table.insert(pgno, Page { start_time: dt }),
        _ => panic!(),
    };
}

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

    // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    /*/
    siv.add_fullscreen_layer(
        TextView::new("hello")
            .wrap_with(OnEventView::new)
            .on_pre_event_inner(Event::Char('x'), move |v, _| {
                v.set_content(Red.paint(std::str::from_utf8(&mmap).unwrap()).to_string());
                Some(EventResult::Consumed(None))
            })
            .on_pre_event_inner(Event::Char('y'), move |v, _| {
                v.set_content("hello");
                Some(EventResult::Consumed(None))
            }),
    );
    */
    // let state = String::from("hello");
    /* */
    let state: String = Red.paint("hello").to_string();
    siv.add_fullscreen_layer(
        Canvas::new(state)
            .with_draw(|text: &String, printer| {
                eprintln!("Printing: {}", text);
                printer.print((0, 0), &Red.paint(text).to_string());
            })
            .with_required_size(|text, _constraints| (text.width(), 1).into()),
    );

    siv.run();
    Ok(())
}
