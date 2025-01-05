use bstr::{BStr, ByteSlice};
use rustc_hash::FxHashMap;
use tracing::error;

use crate::drainrs::{ParseTree, DrainState, RecordParsed, RecordsParsedIter, RecordsParsedResult};

use super::home::DispLine;

struct LineData {
    template_id: usize,
}

#[derive(Default)]
pub struct DrainHandler {
    drain_state: DrainState,
    parsed: FxHashMap<(usize, usize), LineData>,
    templates: Vec<String>,
}

impl DrainHandler {
    pub fn new() -> Self {
        Self { drain_state: DrainState::default(), parsed: FxHashMap::default(), templates: Vec::default() }
    }

    pub fn parse(&mut self, logfile: &BStr, lines: &[DispLine]) {
        // Note, cannot assume the display lines are contiguous in the file, may have been a bunch filtered out.
        // let mut tree = ParseTree::default();
        let mut template_names = Vec::new();

        for line in lines {
            let mut handle_parse = |template_names: &[String], rp: &RecordParsed| {
                // let typ = &self.templates[rp.template_id];
                // rp.values  <-- actual values
                self.parsed.insert(line.file_loc, LineData { template_id: (rp.template_id) } );
                true
            };
            // TODO refactor to use &strs instead of start/end indexes within the mmap?
            // Can also just pass the mmap in here.
            // Then the below should just work; probably need better abstraction in the lib to shield users from the virtually guaranteed need
            // to store the templates.
            let str = &logfile[line.file_loc.0..line.file_loc.1].to_str_lossy();
            let mut rpi = RecordsParsedIter::from(&str, &mut self.drain_state.parse_tree);
            loop {
                let handle = |record| {
                    match record {
                        RecordsParsedResult::NewTemplate(template) => {
                            template_names.push(
                                template
                                    .template
                                    .iter()
                                    .map(|t| t.to_string())
                                    .intersperse(" ".to_string())
                                    .collect::<String>(),
                            );
                            // handle_parse(&template_names, &template.first_parse);
                            true
                        }
                        RecordsParsedResult::RecordParsed(rp) => handle_parse(&template_names, &rp),
                        RecordsParsedResult::ParseError(e) => {
                            error!("err: {}", e);
                            false
                        }
                        RecordsParsedResult::UnparsedLine(line) => {
                            error!("unparsed: {}", line);
                            false
                        }
                        RecordsParsedResult::Done => {
                            log::info!("Done!");
                            false
                        }
                    }
                };
                if !rpi.next(handle) {
                    break;
                }
            }
        }
    }
}