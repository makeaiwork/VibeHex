//! Which language `tr` answers in: the window's, or, on one thread for a
//! while, another. The Assistant speaks the dictation language while the
//! rest of the window keeps its own (`crate::assistant::language`), so its
//! strings are made inside [`speaking`]; a place in the window it names
//! stays in the window's words through [`tr_window`].

use std::cell::Cell;
#[cfg(not(test))]
use std::sync::atomic::{AtomicU8, Ordering};

use super::ENGLISH;

/// The window's column into the tables; 255 means English.
#[cfg(not(test))]
static WINDOW: AtomicU8 = AtomicU8::new(ENGLISH);

// Tests render scenes in different languages concurrently, so each test
// thread has its own language.
#[cfg(test)]
thread_local! {
    static WINDOW: Cell<u8> = const { Cell::new(ENGLISH) };
}

thread_local! {
    /// Another column for this thread, while [`speaking`] runs.
    static SPEAKING: Cell<Option<u8>> = const { Cell::new(None) };
}

#[cfg(not(test))]
fn window() -> u8 {
    WINDOW.load(Ordering::Relaxed)
}

#[cfg(test)]
fn window() -> u8 {
    WINDOW.get()
}

/// The column `tr` uses now.
pub(super) fn current() -> u8 {
    SPEAKING.get().unwrap_or_else(window)
}

#[cfg(not(test))]
pub(super) fn set_current(column: u8) {
    WINDOW.store(column, Ordering::Relaxed);
}

#[cfg(test)]
pub(super) fn set_current(column: u8) {
    WINDOW.set(column);
}

/// The column for a speech language code: one with a table, else English
/// (Italian's fixed strings read in English). Cantonese is written in
/// Traditional characters.
fn column_for_speech(code: &str) -> u8 {
    match code {
        "yue" => super::TRADITIONAL_CHINESE_COLUMN,
        code => super::column_for_code(code),
    }
}

/// While it lives, `tr` on this thread speaks another language; dropped,
/// the one before comes back.
#[must_use = "the language goes back when this is dropped"]
pub(crate) struct Speaking(Option<u8>);

impl Drop for Speaking {
    fn drop(&mut self) {
        SPEAKING.set(self.0);
    }
}

fn with_column<T>(column: Option<u8>, f: impl FnOnce() -> T) -> T {
    let _restore = Speaking(SPEAKING.replace(column));
    f()
}

/// `tr` on this thread in the language `code` names (a speech language
/// code: `ru`, `de`, `it`) until the guard drops; `None` keeps the
/// window's (or the language already spoken).
pub(crate) fn speak(code: Option<&str>) -> Speaking {
    let before = SPEAKING.get();
    if let Some(code) = code {
        SPEAKING.set(Some(column_for_speech(code)));
    }
    Speaking(before)
}

/// Runs `f` with `tr` on this thread in the language `code` names; `None`
/// keeps the window's.
pub(crate) fn speaking<T>(code: Option<&str>, f: impl FnOnce() -> T) -> T {
    let _speaking = speak(code);
    f()
}

/// `english` in the language `code` names, from the same tables.
#[cfg(test)]
fn tr_in(code: &str, english: &str) -> String {
    speaking(Some(code), || super::tr(english).to_owned())
}

/// `english` in the window's language, inside [`speaking`] too: the name
/// of a place in the window, which links to it.
pub(crate) fn tr_window(english: &str) -> &str {
    with_column(None, || super::tr(english))
}

/// The window's language code (`en`, `ru`, `zh-Hant`…), whatever
/// [`speaking`] says.
pub(crate) fn window_code() -> &'static str {
    match window() {
        0 => "pl",
        1 => "zh",
        2 => "ja",
        3 => "de",
        4 => "es",
        5 => "uk",
        super::PORTUGUESE_COLUMN => "pt",
        super::FRENCH_COLUMN => "fr",
        super::TRADITIONAL_CHINESE_COLUMN => "zh-Hant",
        super::RUSSIAN_COLUMN => "ru",
        _ => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaking_translates_on_this_thread_and_the_window_keeps_its_own() {
        set_current(ENGLISH);
        assert_eq!(super::super::tr("Don't switch"), "Don't switch");
        speaking(Some("ru"), || {
            assert_eq!(super::super::tr("Don't switch"), "Не переключать");
            assert_eq!(tr_window("Don't switch"), "Don't switch");
            assert_eq!(window_code(), "en");
        });
        assert_eq!(super::super::tr("Don't switch"), "Don't switch");
        // A language without a table reads English; none keeps the window's.
        assert_eq!(tr_in("it", "Don't switch"), "Don't switch");
        assert_eq!(tr_in("de", "Don't switch"), "Nicht wechseln");
        set_current(super::super::RUSSIAN_COLUMN);
        assert_eq!(
            speaking(None, || super::super::tr("Don't switch").to_owned()),
            "Не переключать"
        );
        assert_eq!(window_code(), "ru");
        set_current(ENGLISH);
    }
}
