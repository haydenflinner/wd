use cursive::align::HAlign;
use cursive::event::EventResult;
use cursive::event::{Event, Key};
use cursive::view::scroll::Scroller;
use cursive::view::*;
use cursive::views::BoxedView;
use cursive::views::TextView;
use cursive::views::*;
use cursive::Cursive;
use cursive::CursiveExt;
use cursive::With;
use log;

use memmap::MmapOptions;
use std::fs::File;
use std::io::Write;

fn main() {
    let mut cursive = Cursive::new();

    /*
    siv.add_layer(TextView::new("Hello World!\nPress q to quit."));

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
    */

    // Read some long text from a file.
    let content = "This is my really long text
    it goes here
    it is reather larg
    l
    o
    ol
    hi lol 1234
    23232
    ";

    // Initialize the cursive logger.
    cursive::logger::init();

    /*
    // Use some logging macros from the `log` crate.
    log::error!("Something serious probably happened!");
    log::warn!("Or did it?");
    log::debug!("Logger initialized.");
    log::info!("Starting!");

    let mut siv = cursive::default();
    siv.add_layer(cursive::views::Dialog::text(
        "Press ~ to open the console.\nPress l to generate logs.\nPress q to quit.",
    ));
    siv.add_global_callback('q', cursive::Cursive::quit);
    siv.add_global_callback('~', cursive::Cursive::toggle_debug_console);

    siv.add_global_callback('l', |_| log::trace!("Wooo"));

    siv.run();
    */

    let mut siv = cursive::default();

    // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    siv.add_fullscreen_layer(
        TextView::new(content)
            .wrap_with(OnEventView::new)
            .on_pre_event_inner(Event::Char('x'), |v, _| {
                v.set_content("new content");
                Some(EventResult::Consumed(None))
            }),
    );

    /*
    // The text is too long to fit on a line, so the view will wrap lines,
    // and will adapt to the terminal size.
    siv.add_fullscreen_layer(
        Dialog::around((TextView::new(content)
            .scrollable()
            .wrap_with(OnEventView::new)
            .on_pre_event_inner(Key::PageUp, |v, _| {
                let scroller = v.get_scroller_mut();
                if scroller.can_scroll_up() {
                    scroller.scroll_up(scroller.last_outer_size().y.saturating_sub(1));
                }
                Some(EventResult::Consumed(None))
            })
            .on_pre_event_inner(Key::PageDown, |v, _| {
                let scroller = v.get_scroller_mut();
                if scroller.can_scroll_down() {
                    scroller.scroll_down(scroller.last_outer_size().y.saturating_sub(1));
                }
                Some(EventResult::Consumed(None))
            })))
        .title("Unicode and wide-character support")
        // This is the alignment for the button
        .h_align(HAlign::Center)
        .button("Quit", |s| s.quit()),
    );
    // Show a popup on top of the view.
    siv.add_layer(Dialog::info(
        "Try resizing the terminal!\n(Press 'q' to \
         quit when you're done.)",
    ));
    */

    siv.run();
}
