// #[derive(Debug)]
// enum Command {
//     LineMove(LineMove),
//     /// Can move the cursor within lines. Cache results for searches unlike `less`.
//     SearchTerm(SearchTerm),
//     FilterCmd(FilterCmd),
//     /// Looks at all log-templates that are on-screen. Skips to next log-line that is from a different template.
//     /// Note that this isn't strictly optimal if you have a couple of templates that alternate more than a screenful apart.
//     /// Maybe we could be smarter and recognize that the autoskip cmd keeps getting issue while a majority of the
//     /// templates present haven't changed. Or we could expose some controls for which are going to be skipped past.
//     /// Press 't' to pull up template menu, then can filter in and out on these?
//     /// Need to integrate with rdrain.
//     AutoSkip,
// }

// /// The primary AppView will be the view of the logfile.
// /// AppViews may also be seen over something like a filtered output, which is independent of the main file.
// trait AppView {
//     /// Returns un-formatted snippet of the logfile.
//     /// Line-wrapping, highlighting, etc to happen in python/textual side?
//     fn get_view(&self) -> &str;
//     /// Offset within get_view.
//     fn get_cursor_pos(&self) -> u64;
//
//     fn take_cmd(&mut self, c: &Command);
// }
//
// struct App {
//     mmap: Mmap,
//
//     /// current byte offset into mmapped file of cursor.
//     cursor: usize,
// }
//
// impl App {}
