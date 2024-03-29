#![allow(unused_imports)]
#![allow(dead_code)]

use cursive::align::HAlign;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::event::Key;
use cursive::menu;
use cursive::reexports::time::Time;

use cursive::view::Nameable;
use cursive::view::Resizable;
use cursive::views::TextView;
use cursive::views::*;
use cursive::Cursive;
use cursive::With;
use cursive::theme::{BaseColor, BorderStyle, Color, ColorStyle, Palette};

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
use unicode_width::UnicodeWidthStr; // To get the width of some text.
use cursive::utils::markup::{StyledString,StyledIndexedSpan};
use cursive::utils::span::IndexedCow;

use regex::Regex;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct LineNo(i64);

#[cfg(not(test))]
use log::{info, warn}; // Use log crate when building application

#[cfg(test)]
use std::{println as info, println as warn}; // Workaround to use prinltn! for logs.


struct Page {
    start_time: DateTime<Utc>,
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


fn parse_date_starting_at(s: &[u8], start_offset: usize) -> Option<DateTime<Local>> {
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
    dateparser::parse_with_timezone(s, &Local).ok()
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

/// TODO Convert this to parse logs. Let's just hardcode how to do it for now.
/// One, highlight WARN/INFO/etc with colors.
/// Two, highlight search results background.
/// Take spans of search results in buffer for use?
/// Parse text using a syntect highlighter.
// For finding matches:
// https://docs.rs/grep-searcher/latest/grep_searcher/struct.SinkMatch.html
// https://docs.rs/grep-searcher/0.1.8/grep_searcher/index.html
// ONly thing to check is what happens with wrapping/overlapping spans. Anything?
/*/
pub fn parse<S: Into<String>>(
    input: S,
    highlighter: &mut syntect::easy::HighlightLines,
    syntax_set: &syntect::parsing::SyntaxSet,
) -> Result<StyledString, syntect::Error> {
    let input = input.into();
    let mut spans = Vec::new();

    for line in input.split_inclusive('\n') {
        for (style, text) in highlighter.highlight_line(line, syntax_set)? {
            spans.push(StyledIndexedSpan {
                content: IndexedCow::from_str(text, &input),
                attr: translate_style(style),
                width: text.width(),
            });
        }
    }

    Ok(StyledString::with_spans(input, spans))
}
*/

struct App {
    mmap: Mmap,
}


impl App {
}

// Is this actually necessary vs the single page fault caused by actually pulling in the page?
// Seems like a micro-optimizaito at this point.
// Cache what it makes sense to cache ONCE WE HVE A WORKING APP.
struct NavHandler<'a> {
    mmap: &'a Mmap,
    // pt: PageTable,
    /// current byte offset into mmapped file.
    byte_cursor: usize,
    // virtual offset from byte cursor? or all apply raw to it?
    filters: Vec<Filter>,
}

impl NavHandler<'_> {
    fn new(mmap: &Mmap) -> NavHandler {
        let nh = NavHandler {
            mmap: mmap,
            // pt: PageTable::new(),
            byte_cursor: 0,
            filters: vec!(),
        };
        // load_initial_pages(&mut nh.pt, mmap);
        nh
    }

    // fn goto(self, spec: &str) {
    //     todo!();
    // }

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
        // TODO mvp: just need goto time! And maye a little bit of polish on the color scheme! Then can add everything else over time!
        // TODO highlighting logs with regex / capturing groups.
        let _spot = bin_search(self.mmap, spec);
    }

    fn goto_pct(&mut self, pct: usize) {
        self.byte_cursor = find_start_line_pct(&self.mmap, pct);
    }

    fn prev_line(&mut self) {
        self.byte_cursor = find_line_starting_before(&self.mmap, self.byte_cursor.saturating_sub(1));
    }

    fn goto_begin(&mut self) {
        self.byte_cursor = 0;
    }

    fn goto_end(&mut self) {
        // TODO Just set cursor to mmap.len()? Does cursor really need to be at beginning of valid line always?
        self.byte_cursor = find_line_starting_before(&self.mmap, self.mmap.len());
    }

    fn goto(&mut self, line_idx: usize) {
        self.byte_cursor = find_line_starting_before(&self.mmap, line_idx);
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
    let filename = std::env::args().nth(1).unwrap();

    let file = File::open(filename).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    let _nh = NavHandler::new(&mmap);
    Ok(())
}
/*
    // let _cursive = Cursive::new();

    // let mut cursor: usize = 0;
    // let mut app = App{ mmap: &mmap, cursor: 0};
    let app = Rc::new(RefCell::new(App { mmap: mmap, cursor: 0}));
    // let bapp = App { mmap: mmap, cursor: 0};
    // assert_eq!(b"# memmap", &mmap[0..8]);

    // Initialize the cursive logger.
    cursive::logger::init();
    // let mut page_table = BTreeMap::new();
    // load_initial_pages(&mut page_table, mmap);

    let mut siv = cursive::default();

    //     siv.set_theme(cursive::theme::Theme {
    //     shadow: true,
    //     borders: BorderStyle::Simple,
    //     palette: Palette::default().with(|palette| {
    //         use cursive::theme::BaseColor::*;
    //         use cursive::theme::Color::*;
    //         use cursive::theme::PaletteColor::*;

    //         palette[Background] = TerminalDefault;
    //         palette[View] = TerminalDefault;
    //         palette[Primary] = White.dark();
    //         palette[TitlePrimary] = Blue.light();
    //         palette[Secondary] = Blue.light();
    //         palette[Highlight] = Blue.dark();
    //     }),
    // });

    siv.add_global_callback('q', |s| s.quit());
    let text = "tv";

    let j_app = app.clone();
    let b_app = app.clone();
    let a_app = app.clone();
    let g_app = app.clone();
    let gg_app = app.clone();
    let app3 = app.clone();

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
    // siv.set_user_data(bapp);
    /*fn ok (s: &mut Cursive, name: &str) {
        s.call_on_name("tv", |t: &mut TextView| {

            // app3.borrow_mut().goto_str(name);
            s.with_user_data(|app| {
                app.goto_str(name);
            });
            // t.set_content(app3.borrow_mut().get_view());
            t.set_content(s.with_user_data(|app| { app.get_view() })); //.get_view());
        });
        s.pop_layer();
    };
    let ok_ref = Rc::new(RefCell::new(ok));
    let ok1 = ok_ref.clone();
    let ok2 = ok_ref.clone();
    */
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
        .on_event(Event::Char('g'), move |siv| {
            // g_app.borrow_mut().goto_begin();
            // siv.call_on_name(text, |t: &mut TextView| { t.set_content(g_app.borrow_mut().get_view()); });
            // g for goto. Type in exact or relative?
            let app3 = app3.clone();
            let app4 = app3.clone();
            let app5 = g_app.clone(); // holy cow talk about annoying.
            // but using set and with_user_data doesn't seem to be any better.
            // these lifetimes SUCK
            siv.add_layer(
                Dialog::around(
                LinearLayout::vertical()
                    .child(DummyView.fixed_height(1))
                    .child(TextView::new("Enter goto-spec").h_align(HAlign::Center))
                    .child(EditView::new()
                           // .on_submit(Into::<Fn(&mut Cursive, &str) >::into(ok_ref.borrow_mut()))
                           .on_submit(move |s: &mut Cursive, name: &str| {
                               s.call_on_name("tv", |t: &mut TextView| {
                                   app5.borrow_mut().goto_str(name);
                                   //s.with_user_data(|app: &mut App| {
                                       //app.goto_str(name);
                                   //});
                                   // t.set_content(s.with_user_data(|app: &mut App| { app.get_view() }).unwrap()); //.get_view());
                                   t.set_content(app5.borrow_mut().get_view()); //.get_view());
                               });
                               s.pop_layer();
                           })
                           // ok1.borrow_mut())
                           // .on_submit(ok)
                    .with_name("username").fixed_width(20)
                    )
                    .child(TextView::new("example: 16:44:21"))
                    .child(TextView::new("example: +1s"))
                    .child(TextView::new("example: 40%"))
                    .child(DummyView.fixed_height(1))
                    // .child(TextView::new("Enter Channel").h_align(HAlign::Center))
                    // .child(EditView::new().with_name("channel").fixed_width(20)),
                ).title("goto")
                .button("enter", move |s| {
                    let app4 = app4.clone();
                    s.call_on_name("tv", |t: &mut TextView| {
                        /*
                        s.with_user_data(|app: &mut App| {
                            app.goto_str("TODOgetview");
                        });
                        t.set_content(s.with_user_data(|app: &mut App| { app.get_view() }).unwrap()); //.get_view());
                        lifetime issues, guh.
                        */
                        app4.borrow_mut().goto_str("TODOgetview");
                        t.set_content(app3.borrow_mut().get_view());
                    });
                    s.pop_layer();
                    // ok2.borrow_mut()(s, "dummy");
                    // ok(s, "dummy");
                })
                .wrap_with(OnEventView::new).on_event(Event::Key(Key::Enter), move|siv| { siv.pop_layer(); })
            );
        })
        .on_event(Event::Char('/'), move|siv| {
            // g_app.borrow_mut().goto_begin();
            // siv.call_on_name(text, |t: &mut TextView| { t.set_content(g_app.borrow_mut().get_view()); });
            // g for goto. Type in exact or relative?
            siv.add_layer(
                Dialog::around(
                LinearLayout::vertical()
                    .child(DummyView.fixed_height(1))
                    .child(TextView::new("search term").h_align(HAlign::Center))
                    .child(EditView::new()
                    .with_name("username").fixed_width(20))
                    .child(TextView::new(r"example: xyz12\d+"))
                    // .child(TextView::new("example: +1s"))
                    // .child(TextView::new("example: 40%"))
                    .child(DummyView.fixed_height(1))
                    // .child(TextView::new("Enter Channel").h_align(HAlign::Center))
                    // .child(EditView::new().with_name("channel").fixed_width(20)),
                ).title("search / find")
                // TODO Make these not actually buttons but just text telling you what to do, same for all menues.
                .button("cancel (esc)", |s| { s.pop_layer(); })
                .button("enter", |s| { s.pop_layer(); })
                .wrap_with(OnEventView::new).on_event(Event::Key(Key::Enter), move|siv| { siv.pop_layer(); })
            );
        })
        .on_event(Event::Char('f'), move|siv| {
            // g_app.borrow_mut().goto_begin();
            // siv.call_on_name(text, |t: &mut TextView| { t.set_content(g_app.borrow_mut().get_view()); });
            // g for goto. Type in exact or relative?
            // TOML and yaml would allow writing regexes without escaping. As would a .py config file, actually...
            // auto add last searched term to highlights unless ends with '.*'?
            siv.add_layer(
                Dialog::around(
                    // TODO select within menu by pressing i or o (in / out), then skips to term-entry dialog if press those 2.
                    // otherwise, this is a menu of the current filters, and can enable/disable by pressing space/enter?
                    // Change-color of them by pressing c on them.
                LinearLayout::vertical()
                    .child(DummyView.fixed_height(1))
                    .child(TextView::new("filter regex").h_align(HAlign::Center))
                    .child(EditView::new()
                    .with_name("username").fixed_width(20))
                    .child(TextView::new(r"example: xyz12\d+"))
                    // .child(TextView::new("example: +1s"))
                    // .child(TextView::new("example: 40%"))
                    .child(DummyView.fixed_height(1))
                    // .child(TextView::new("Enter Channel").h_align(HAlign::Center))
                    // .child(EditView::new().with_name("channel").fixed_width(20)),
                ).title("search / find")
                // TODO Make these not actually buttons but just text telling you what to do, same for all menues.
                .button("cancel (esc)", |s| { s.pop_layer(); })
                .button("enter", |s| { s.pop_layer(); })
                .wrap_with(OnEventView::new).on_event(Event::Key(Key::Enter), move|siv| { siv.pop_layer(); })
            );
        })
        // todo 'u' and 'd' half-screen scroll cmds from less? Definitely pgup and pgdown buttons.
        .on_event(Event::Char('G'), move|siv| {
            gg_app.borrow_mut().goto_end();
            siv.call_on_name(text, |t: &mut TextView| { t.set_content(gg_app.borrow_mut().get_view()); });
        })
        .on_event(Event::Char('h'), |siv| {
            // TODO h for highlight
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
}*/
