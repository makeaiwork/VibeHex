//! No card stands in for what a model doesn't know. A switch that only
//! changes what a list shows (a filter, a sort order, a search field, a
//! count) is no answer to "which models do I have?": offered there, it is
//! a way to go and look, in place of the answer. Such a switch is a card
//! only when the question asks for a change.

/// Switches that change only what the window shows of what is there.
const VIEWS: [&str; 6] = [
    "labs.picker_downloaded",
    "labs.picker_sort_filter",
    "labs.picker_active_first",
    "labs.settings_search",
    "labs.history_stats",
    "labs.benchmark_details",
];

/// Words that ask for a change, as a question starts or holds them, in
/// lower case: English, then Russian.
const CHANGE: [&str; 34] = [
    "turn on",
    "turn off",
    "turn it",
    "switch on",
    "switch off",
    "enable",
    "disable",
    "show only",
    "show me only",
    "hide",
    "filter",
    "sort",
    "set up",
    "set it",
    "i want",
    "i'd like",
    "let me",
    "make it",
    "can you add",
    "включи",
    "выключи",
    "отключи",
    "поставь",
    "сделай",
    "настрой",
    "покажи только",
    "показывай только",
    "показывать только",
    "скрой",
    "спрячь",
    "фильтр",
    "сортир",
    "хочу",
    "добавь",
];

/// Whether `question` asks for something to change.
pub fn wants_change(question: &str) -> bool {
    let lower = question.to_lowercase();
    CHANGE.iter().any(|word| lower.contains(word))
}

/// Whether a card for `key` would only be a way to look something up:
/// a view switch, for a question that asks for no change.
pub fn only(key: &str, question: &str) -> bool {
    VIEWS.contains(&key) && !question.trim().is_empty() && !wants_change(question)
}
