use crate::action::{Action, CursorMove, FilterListAction, FilterType, LineFilter};

#[derive(PartialEq, Eq)]
pub(crate) enum LineFilterResult {
    Include,
    Exclude,
    Indifferent,
}

pub(crate) fn line_allowed(filters: &Vec<LineFilter>, line: &str) -> (bool, LineFilterResult) {
    let mut cur = LineFilterResult::Indifferent;
    let get_active_filters = || filters.iter().filter(|f| f.enabled);
    for filter in get_active_filters() {
        match (line.contains(&filter.needle), filter.filter_type) {
            (true, crate::action::FilterType::In) => cur = LineFilterResult::Include,
            (true, crate::action::FilterType::Out) => cur = LineFilterResult::Exclude,
            _ => continue,
            // (false, crate::action::FilterType::In) => LineFilterResult::Indifferent,
            // (false, crate::action::FilterType::Out) => LineFilterResult::Indifferent,
        }
    }
    if cur == LineFilterResult::Indifferent
        && get_active_filters().any(|f| f.filter_type == FilterType::In)
    {
        // If there are any MUST_INCLUDE lines and none of them matched, we should not print this line.
        return (false, cur); // cur here is not depended on yet 20230623
    }
    (cur != LineFilterResult::Exclude, cur)
}
