//! From an answer to the place it cites, and back (the Jump board).
//!
//! An inline `[n]` or a source chip opens its meeting on Transcript, its
//! Files text on the Original, or its dictation in History, and leaves a
//! small "came from" on `Ui`. While it is there the pane's header leads
//! with Back to Assistant (a chevron, a small face, the words), and ⌘[
//! (Ctrl+[ elsewhere) does the same: back to the same conversation, scrolled
//! where it was. Choosing another pane clears it. The mouse's back button
//! doesn't yet: Rune hands a button other than the primary only to an element
//! that also takes primary presses, which over the whole pane would take
//! text selection and window dragging from it.
//!
//! The cited place is tinted in the accent, all corners round, with the
//! answer's `[n]` beside the speaker and the words searched for lit inside
//! it, until the person leaves. An item cited more than once gets a bar over
//! its text: "From the answer: …", "1 of 2", arrows between the places, and
//! a cross that clears the light. The other places are tinted fainter.

use rune::{El, Icon, Span, Tx, col, icon, rich, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::sidebar::Pane;
use super::face::face;
use crate::assistant::recordings::{Kind, Ref};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// Where the cited place lands: about a third of the way down the text.
const PLACE_TOP: f32 = 120.0;
/// The current place, which the scroll lands on.
pub(in super::super) const NOW_KEY: &str = "cited-now";

/// The answer a pane was opened from.
#[derive(Clone, Debug)]
pub(in super::super) struct CameFrom {
    conversation: u32,
    /// How far the conversation was scrolled.
    scroll_y: f32,
    /// The places lit, until the cross clears them.
    pub(in super::super) cited: Option<Cited>,
}

/// The places one answer cites in one item.
#[derive(Clone, Debug)]
pub(in super::super) struct Cited {
    kind: Kind,
    id: String,
    /// The question, shortened: what the bar says the places are about.
    topic: String,
    /// In the order they come in the item.
    pub(in super::super) places: Vec<Place>,
    /// The one scrolled to.
    pub(in super::super) current: usize,
}

#[derive(Clone, Debug)]
pub(in super::super) struct Place {
    pub(in super::super) n: u32,
    pub(in super::super) offset_ms: u64,
    words: Vec<String>,
}

/// How one line of an item shows a place cited in it.
#[derive(Clone, Copy)]
pub(in super::super) struct Mark<'a> {
    pub(in super::super) n: u32,
    /// The place scrolled to, rather than another one cited.
    pub(in super::super) now: bool,
    words: &'a [String],
}

impl Cited {
    /// The places in the line starting at each of `starts`: the last line
    /// started by a place's moment holds it.
    pub(in super::super) fn marks(&self, starts: &[u64]) -> Vec<Option<Mark<'_>>> {
        let mut marks = vec![None; starts.len()];
        for (index, place) in self.places.iter().enumerate() {
            let line = starts
                .iter()
                .rposition(|start| *start <= place.offset_ms)
                .unwrap_or(0);
            let Some(slot) = marks.get_mut(line) else {
                continue;
            };
            let now = index == self.current;
            if slot.as_ref().is_none_or(|mark: &Mark| !mark.now) {
                *slot = Some(Mark {
                    n: place.n,
                    now,
                    words: &place.words,
                });
            }
        }
        marks
    }
}

/// The places lit in `id`, of `kind`.
pub(in super::super) fn cited<'a>(ui: &'a Ui, kind: Kind, id: &str) -> Option<&'a Cited> {
    ui.came_from
        .as_ref()
        .and_then(|from| from.cited.as_ref())
        .filter(|cited| cited.kind == kind && cited.id == id)
}

/// Opens place `n` of turn `index` in `conversation`.
pub(super) fn open(ui: &mut Ui, conversation: u32, index: usize, n: u32, tx: &mut Tx) {
    if ui.is_fixture() || ui.assistant.fixture {
        return;
    }
    let Some(turn) = ui
        .assistant
        .store
        .get(conversation)
        .and_then(|conversation| conversation.turns.get(index))
    else {
        return;
    };
    let Some(target) = turn.refs.iter().find(|cited| cited.n == n).cloned() else {
        return;
    };
    let cited = places(&turn.question, &turn.refs, &target);
    let scroll_y = ui.assistant.scroll_y.get();
    match target.kind {
        Kind::Meeting => {
            ui.select_pane(Pane::Meetings, tx);
            ui.meetings.view = super::super::meetings::View::Recordings;
            ui.meetings.select(target.id.clone());
            ui.meetings.tab = Some(super::super::meeting_tabs::MeetingTab::Transcript);
        }
        Kind::File => {
            ui.select_pane(Pane::Files, tx);
            ui.files.view = super::super::files::View::default();
            ui.files.select(target.id.clone());
            let original = ui.files.original(&target.id);
            ui.files.tab = original.map(super::super::files::Tab::Version);
        }
        Kind::Dictation => {
            ui.select_pane(Pane::History, tx);
            ui.history.selected = target.id.parse().ok();
        }
    }
    let lit = !cited.places.is_empty();
    ui.came_from = Some(CameFrom {
        conversation,
        scroll_y,
        cited: lit.then_some(cited),
    });
    if lit {
        tx.scroll_key_to(NOW_KEY, PLACE_TOP);
    }
}

/// Every place `refs` cites in `target`'s item, `target` current.
fn places(question: &str, refs: &[Ref], target: &Ref) -> Cited {
    let mut places: Vec<Place> = refs
        .iter()
        .filter(|cited| cited.kind == target.kind && cited.id == target.id)
        .filter_map(|cited| {
            Some(Place {
                n: cited.n,
                offset_ms: cited.offset_ms?,
                words: cited.words.clone(),
            })
        })
        .collect();
    places.sort_by_key(|place| place.offset_ms);
    places.dedup_by_key(|place| place.offset_ms);
    let current = places
        .iter()
        .position(|place| Some(place.offset_ms) == target.offset_ms)
        .unwrap_or(0);
    Cited {
        kind: target.kind,
        id: target.id.clone(),
        topic: topic(question),
        places,
        current,
    }
}

/// The question as a short topic: its first line, without the question
/// mark, cut at a word near 48 characters.
fn topic(question: &str) -> String {
    let line = question.lines().next().unwrap_or_default().trim();
    let line = line.trim_end_matches(['?', '？', '.', '!']);
    match line.char_indices().nth(48) {
        Some((cut, _)) => {
            let cut = line[..cut].rfind(' ').unwrap_or(cut);
            format!("{}…", &line[..cut])
        }
        None => line.to_owned(),
    }
}

/// A scene's: `item` opened from conversation 1, `places` (`n`, moment,
/// words) lit, the first current.
pub(super) fn scene(
    kind: Kind,
    item: &str,
    topic: &str,
    places: &[(u32, u64, &[&str])],
) -> CameFrom {
    CameFrom {
        conversation: 1,
        scroll_y: 0.0,
        cited: Some(Cited {
            kind,
            id: item.into(),
            topic: topic.into(),
            places: places
                .iter()
                .map(|(n, offset_ms, words)| Place {
                    n: *n,
                    offset_ms: *offset_ms,
                    words: words.iter().map(|word| (*word).into()).collect(),
                })
                .collect(),
            current: 0,
        }),
    }
}

/// Back to the conversation the pane was opened from, where it was.
pub(in super::super) fn back(ui: &mut Ui, tx: &mut Tx) {
    let Some(from) = ui.came_from.take() else {
        return;
    };
    ui.select_pane(Pane::Assistant, tx);
    let pane = &mut ui.assistant;
    if pane.store.get(from.conversation).is_some() {
        pane.open = Some(from.conversation);
        pane.view = super::View::Chat;
        tx.scroll_to_y("assistant-scroll", from.scroll_y);
    }
}

/// ⌘[ (Ctrl+[ off macOS) goes back while there is somewhere to go.
pub(in super::super) fn on_key(ui: &mut Ui, event: &rune::KeyEvent, tx: &mut Tx) -> bool {
    if event.key != rune::Key::Command('[') || ui.came_from.is_none() {
        return false;
    }
    back(ui, tx);
    true
}

static CHEVRON_LEFT: Icon = Icon {
    name: "hex-chevron-left",
    view: 24.0,
    body: r#"<path d="m15 18-6-6 6-6" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

/// Back to Assistant, before the pane's title, while it was opened from
/// an answer.
pub(in super::super) fn back_button(ui: &Ui, p: Paint) -> Option<El> {
    if ui.came_from.is_none() || ui.pane == Pane::Assistant || ui.labs_open {
        return None;
    }
    let c = &p.k.c;
    let accent = p.accent();
    let label = tr("Back to Assistant");
    Some(
        row()
            .key("back-to-assistant")
            .role("button")
            .label(label)
            .focusable(true)
            .focus_ring(2.0, 0.0, c.border_focus)
            .h(28.0)
            .pl(6.0)
            .pr(10.0)
            .gap(6.0)
            .center_y()
            .radius(7.0)
            .fill(p.k.alpha(accent, 0.1))
            .border(p.hair, p.k.alpha(accent, 0.45))
            .hover(p.k.alpha(accent, 0.16))
            .press(p.k.alpha(accent, 0.22))
            .on_press_tx(back)
            .child(icon(&CHEVRON_LEFT, 14.0).key("chevron").color(accent))
            .child(face(
                "face",
                16.0,
                accent,
                accent,
                ui.assistant.face.mood(false, true),
            ))
            .child(
                text(label)
                    .key("label")
                    .font(p.k.ty.ui_medium)
                    .color(c.text_base),
            ),
    )
}

/// One cited line on an accent tint: who and when (`head`) with the
/// answer's `[n]`, or a time beside it (`lead`) with the `[n]` under it,
/// then `words` with the searched-for words lit. It reaches 10 pt past the
/// text each side, so the lines around it are inset as much (`INSET`).
pub(in super::super) fn line(
    p: Paint,
    key: String,
    mark: Mark,
    (head, lead): (Option<El>, Option<El>),
    words: &str,
) -> El {
    let k = p.k;
    let accent = p.accent();
    let strength = if mark.now { 1.0 } else { 0.4 };
    let tint = k.alpha(accent, if k.dark { 0.13 } else { 0.09 } * strength);
    let badge = badge(p, mark);
    let text = rich(words.to_owned(), lit(p, words, mark.words, mark.now))
        .key("text")
        .font(k.ty.body)
        .color(if mark.now { k.c.text_base } else { p.reading() })
        .grow(1.0)
        .shrink()
        .wrap();
    let shown = match lead {
        Some(lead) => row()
            .gap(12.0)
            .child(
                col()
                    .key("lead")
                    .gap(4.0)
                    .items_start()
                    .child(lead)
                    .child(badge),
            )
            .child(text),
        None => col()
            .gap(2.0)
            .child(
                row()
                    .key("head")
                    .gap(6.0)
                    .center_y()
                    .children(head)
                    .child(badge),
            )
            .child(text),
    };
    let body = col()
        .key(key)
        .px(INSET)
        .py(7.0)
        .mx(-INSET)
        .radius(8.0)
        .fill(tint)
        .child(shown.key("shown"));
    if mark.now {
        col().key(NOW_KEY).child(body)
    } else {
        body
    }
}

/// How far a cited line's tint reaches past its text.
pub(in super::super) const INSET: f32 = 10.0;

/// The answer's `[n]`, fainter on a place not shown.
fn badge(p: Paint, mark: Mark) -> El {
    let k = p.k;
    let accent = p.accent();
    let strength = if mark.now { 1.0 } else { 0.4 };
    row()
        .key("mark")
        .h(16.0)
        .min_w(16.0)
        .px(3.0)
        .center()
        .radius(4.0)
        .fill(if mark.now {
            k.alpha(accent, 0.14)
        } else {
            rune::Color::TRANSPARENT
        })
        .border(p.hair, k.alpha(accent, 0.4 * strength + 0.1))
        .child(
            text(mark.n.to_string())
                .key("n")
                .font(rune::TextStyle {
                    size: 10.5,
                    line_height: 14.0,
                    weight: 600.0,
                    ..k.ty.small
                })
                .color(if mark.now {
                    accent
                } else {
                    k.alpha(accent, 0.7)
                }),
        )
}

/// The words in `text` that start with one of `words`, lit.
fn lit(p: Paint, text: &str, words: &[String], now: bool) -> Vec<Span> {
    let k = p.k;
    let folded: Vec<String> = words
        .iter()
        .filter(|word| word.chars().count() >= 3)
        .map(|word| crate::meeting::query::fold(word))
        .collect();
    let mut spans = Vec::new();
    let mut start = None;
    for (at, ch) in text.char_indices().chain([(text.len(), ' ')]) {
        match (ch.is_alphanumeric(), start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                let word = crate::meeting::query::fold(&text[from..at]);
                if folded.iter().any(|term| word.starts_with(term.as_str())) {
                    spans.push(
                        Span::new(from, at)
                            .bg(k.alpha(p.accent(), if now { 0.28 } else { 0.14 }), 2.0)
                            .color(k.c.text_base),
                    );
                }
                start = None;
            }
            _ => {}
        }
    }
    spans
}

/// The bar over an item cited more than once: what the places are about,
/// which one shows, arrows between them, and a cross that clears the light.
pub(in super::super) fn bar(ui: &Ui, p: Paint, kind: Kind, id: &str) -> Option<El> {
    let cited = cited(ui, kind, id).filter(|cited| cited.places.len() > 1)?;
    let k = p.k;
    let accent = p.accent();
    let small = rune::TextStyle {
        size: 12.5,
        ..k.ty.ui
    };
    let step = |key: &'static str, glyph: &'static Icon, label: &str, by: isize| {
        row()
            .key(key)
            .role("button")
            .label(label)
            .focusable(true)
            .focus_ring(2.0, 0.0, k.c.border_focus)
            .size(22.0, 22.0)
            .center()
            .radius(5.0)
            .hover(k.alpha(accent, 0.16))
            .press(k.alpha(accent, 0.24))
            .child(icon(glyph, 14.0).color(accent))
            .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| {
                let cited = ui.came_from.as_mut().and_then(|from| from.cited.as_mut());
                match (by, cited) {
                    (0, _) => {
                        if let Some(from) = ui.came_from.as_mut() {
                            from.cited = None;
                        }
                    }
                    (by, Some(cited)) => {
                        let count = cited.places.len() as isize;
                        cited.current = (cited.current as isize + by).rem_euclid(count) as usize;
                        tx.scroll_key_to(NOW_KEY, PLACE_TOP);
                    }
                    _ => {}
                }
            })
    };
    let count = tr_named(
        "{current} of {count}",
        &[
            ("current", &(cited.current + 1).to_string()),
            ("count", &cited.places.len().to_string()),
        ],
    );
    Some(
        row()
            .key("cited-bar")
            .h(30.0)
            .pl(10.0)
            .pr(6.0)
            .gap(8.0)
            .center_y()
            .radius(8.0)
            .fill(k.alpha(accent, 0.1))
            .border(p.hair, k.alpha(accent, 0.4))
            .child(
                text(tr_fill("From the answer: {}", &cited.topic))
                    .key("topic")
                    .font(small)
                    .color(k.c.text_base)
                    .grow(1.0)
                    .shrink()
                    .max_lines(1),
            )
            .child(text(count).key("count").font(small).color(accent))
            .child(step(
                "previous",
                k.glyph.chevron_up,
                tr("Previous place"),
                -1,
            ))
            .child(step("next", k.glyph.chevron_down, tr("Next place"), 1))
            .child(step("clear", k.glyph.close, tr("Clear highlight"), 0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cited(n: u32, id: &str, offset: Option<u64>) -> Ref {
        Ref {
            n,
            kind: Kind::Meeting,
            id: id.into(),
            offset_ms: offset,
            words: vec!["бюджет".into()],
            ..Ref::default()
        }
    }

    #[test]
    fn an_item_cited_twice_has_two_places_the_one_clicked_current() {
        let refs = [
            cited(1, "a", Some(9_000)),
            cited(2, "b", Some(1_000)),
            cited(3, "a", Some(2_000)),
        ];
        let places = places("Кто ведёт найм?", &refs, &refs[0]);
        assert_eq!(
            places.places.iter().map(|p| p.n).collect::<Vec<_>>(),
            [3, 1]
        );
        assert_eq!(places.current, 1);
        assert_eq!(places.topic, "Кто ведёт найм");
        let marks = places.marks(&[0, 1_500, 8_000, 12_000]);
        assert_eq!(
            marks
                .iter()
                .map(|mark| mark.map(|mark| (mark.n, mark.now)))
                .collect::<Vec<_>>(),
            [None, Some((3, false)), Some((1, true)), None]
        );
    }
}
