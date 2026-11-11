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
//! The cited passage is tinted in the accent, all corners round, with the
//! answer's `[n]` beside the speaker, and the very lines the answer cited
//! (`[2.3]`, kept with the place as their time and bytes) lit inside it,
//! until the person leaves; a citation that named no line it has tints the
//! passage alone. The place lands about a third of the way down the pane.
//! An item cited in more than one place gets a bar over its text: "From the
//! answer: …", "1 of 2", arrows between the places, and a cross that clears
//! the light. The other places are tinted fainter. A dictation's row in
//! History is tinted and marked the same way.

use std::ops::Range;

use rune::{El, Icon, TextHighlight, Tx, col, icon, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::sidebar::Pane;
use super::face::face;
use crate::assistant::recordings::{Kind, Ref};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// Where the cited place lands before the pane was ever measured.
const PLACE_TOP: f32 = 120.0;
/// The pane's header, over the scrolling text.
const HEADER: f32 = 52.0;
/// What the scroll lands on: the current place's first lit words, else
/// the line holding it.
pub(in super::super) const NOW_KEY: &str = "cited-now";
/// The line holding the current place, when its words take [`NOW_KEY`].
const LINE_KEY: &str = "cited-line";

/// The answer a pane was opened from.
#[derive(Clone, Debug)]
pub(in super::super) struct CameFrom {
    conversation: u32,
    /// How far the conversation was scrolled.
    scroll_y: f32,
    /// The places lit, until the cross clears them.
    pub(in super::super) cited: Option<Cited>,
    /// The current place still has to land, once its item shows.
    land: bool,
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

#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Place {
    pub(in super::super) n: u32,
    /// Which of the answer's `[n]` named it; none when it was cited whole.
    mark: Option<u32>,
    /// When its lines start, or the passage's moment; none for a
    /// dictation, which is lit whole.
    start_ms: Option<u64>,
    /// The cited lines' bytes in the item's text; none: the passage tints
    /// alone.
    at: Option<Range<usize>>,
}

/// How one line of an item shows the places cited in it.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Mark {
    pub(in super::super) n: u32,
    /// It holds the place scrolled to, rather than only others cited.
    pub(in super::super) now: bool,
    /// The place scrolled to starts here.
    lands: bool,
    /// The cited lines' bytes in it.
    lit: Vec<Lit>,
}

#[derive(Clone, Debug, PartialEq)]
struct Lit {
    at: Range<usize>,
    now: bool,
    /// The first lit words of the place scrolled to.
    lands: bool,
}

impl Cited {
    /// How each of an item's lines shows the places in it. `lines`: each
    /// line's start and words, in the item's text one after another with
    /// `gap` bytes between (a meeting's turns, a file's paragraphs). Lines
    /// cited by their words light them; a place whose words aren't there,
    /// or that named none, tints the last line started by its moment.
    pub(in super::super) fn marks(&self, lines: &[(u64, &str)], gap: usize) -> Vec<Option<Mark>> {
        let mut marks: Vec<Option<Mark>> = vec![None; lines.len()];
        let mut starts = Vec::with_capacity(lines.len());
        let mut offset = 0;
        for (_, words) in lines {
            starts.push(offset);
            offset += words.len() + gap;
        }
        for (index, place) in self.places.iter().enumerate() {
            let now = index == self.current;
            let mut landed = false;
            for (line, ((_, words), start)) in lines.iter().zip(&starts).enumerate() {
                let Some(at) = &place.at else { break };
                let from = at.start.max(*start) - start;
                let to = at.end.min(start + words.len()).saturating_sub(*start);
                if from >= to || !words.is_char_boundary(from) || !words.is_char_boundary(to) {
                    continue;
                }
                let lands = now && !landed;
                landed = true;
                let mark = mark(&mut marks[line], place.n, now, lands);
                mark.lit.push(Lit {
                    at: from..to,
                    now,
                    lands,
                });
            }
            if !landed && let Some(moment) = place.start_ms {
                let line = lines.iter().rposition(|(start, _)| *start <= moment);
                if let Some(slot) = marks.get_mut(line.unwrap_or(0)) {
                    mark(slot, place.n, now, now);
                }
            }
        }
        marks
    }
}

/// The line's mark, its `[n]` the current place's once it holds it.
fn mark(slot: &mut Option<Mark>, n: u32, now: bool, lands: bool) -> &mut Mark {
    let mark = slot.get_or_insert(Mark {
        n,
        now,
        lands: false,
        lit: Vec::new(),
    });
    if now && !mark.now {
        mark.n = n;
        mark.now = true;
    }
    mark.lands |= lands;
    mark
}

/// The places lit in `id`, of `kind`.
pub(in super::super) fn cited<'a>(ui: &'a Ui, kind: Kind, id: &str) -> Option<&'a Cited> {
    ui.came_from
        .as_ref()
        .and_then(|from| from.cited.as_ref())
        .filter(|cited| cited.kind == kind && cited.id == id)
}

/// Opens place `n` of turn `index` in `conversation`: the lines its
/// `mark`-th `[n]` cites, or its first place for a chip.
pub(super) fn open(
    ui: &mut Ui,
    (conversation, index): (u32, usize),
    n: u32,
    mark: Option<u32>,
    tx: &mut Tx,
) {
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
    let cited = places(&turn.question, &turn.refs, &target, mark);
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
            if let Ok(id) = target.id.parse() {
                ui.history.reveal(id);
            }
        }
    }
    let lit = !cited.places.is_empty();
    let lands = landing(&cited);
    ui.came_from = Some(CameFrom {
        conversation,
        scroll_y,
        cited: lit.then_some(cited),
        land: lit,
    });
    if let Some(key) = lands {
        tx.scroll_key_to(key, place_top(ui));
    }
}

/// Every place `refs` cite in `target`'s item, the one its `mark`-th `[n]`
/// names current, else its first.
fn places(question: &str, refs: &[Ref], target: &Ref, mark: Option<u32>) -> Cited {
    let mut places: Vec<Place> = Vec::new();
    for cited in refs
        .iter()
        .filter(|cited| cited.kind == target.kind && cited.id == target.id)
    {
        // Cited whole, or only by lines it hasn't: the passage alone.
        if cited.lines.is_empty() && (cited.offset_ms.is_some() || cited.kind == Kind::Dictation) {
            places.push(Place {
                n: cited.n,
                mark: None,
                start_ms: cited.offset_ms,
                at: None,
            });
        }
        places.extend(cited.lines.iter().map(|lines| Place {
            n: cited.n,
            mark: Some(lines.mark),
            start_ms: Some(lines.start_ms),
            at: Some(lines.at.clone()),
        }));
    }
    places.sort_by_key(|place| (place.start_ms, place.at.as_ref().map(|at| at.start)));
    places.dedup_by(|later, earlier| later.start_ms == earlier.start_ms && later.at == earlier.at);
    let of = |place: &Place| place.n == target.n;
    let current = places
        .iter()
        .position(|place| of(place) && mark.is_some() && place.mark == mark)
        .or_else(|| places.iter().position(of))
        .unwrap_or(0);
    Cited {
        kind: target.kind,
        id: target.id.clone(),
        topic: topic(question),
        places,
        current,
    }
}

/// What the scroll lands on for the current place: [`NOW_KEY`], or a
/// dictation's row.
fn landing(cited: &Cited) -> Option<String> {
    cited.places.get(cited.current)?;
    Some(match cited.kind {
        Kind::Dictation => row_key(&cited.id),
        Kind::Meeting | Kind::File => NOW_KEY.to_owned(),
    })
}

/// A History row's key.
fn row_key(id: &str) -> String {
    format!("history-entry{id}")
}

/// How far below the top of the pane's text the place lands: a third of
/// the way down what shows, whatever the window's height.
pub(in super::super) fn place_top(ui: &Ui) -> f32 {
    let panel = ui.assistant.panel_h;
    if panel > HEADER * 2.0 {
        ((panel - HEADER) / 3.0).round()
    } else {
        PLACE_TOP
    }
}

/// Keeps the pane's height for [`place_top`].
pub(in super::super) fn panel_laid_out(ui: &mut Ui, rect: [f32; 4], tx: &mut Tx) {
    ui.assistant.panel_h = rect[3];
    tx.live_only();
}

/// Lands the current place once its item shows: opening scrolls at once,
/// and this catches a pane that loads its item a moment later (and a
/// scene's first frame). True when it scrolled.
pub(in super::super) fn poll(ui: &mut Ui, tx: &mut Tx) -> bool {
    let Some(cited) = ui
        .came_from
        .as_ref()
        .filter(|from| from.land)
        .and_then(|from| from.cited.as_ref())
        .filter(|cited| shows(ui, cited))
    else {
        return false;
    };
    let key = landing(cited);
    let top = place_top(ui);
    if let Some(from) = ui.came_from.as_mut() {
        from.land = false;
    }
    if let Some(key) = key {
        tx.scroll_key_to(key, top);
    }
    true
}

/// Whether the cited item shows in its pane, so its place is laid out.
fn shows(ui: &Ui, cited: &Cited) -> bool {
    let id = cited.id.as_str();
    match cited.kind {
        Kind::Meeting => {
            ui.pane == Pane::Meetings
                && ui.meetings.selected.as_deref() == Some(id)
                && (ui.meetings.meetings.iter())
                    .any(|meeting| meeting.id == id && !meeting.turns().is_empty())
        }
        Kind::File => ui.pane == Pane::Files && ui.files.record(id).is_some(),
        Kind::Dictation => {
            ui.pane == Pane::History
                && (ui.history.entries.iter()).any(|entry| entry.id.to_string() == id)
        }
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

/// A scene's place: `n`, its moment, and the cited words' bytes.
pub(super) type ScenePlace = (u32, Option<u64>, Option<Range<usize>>);

/// A scene's: `item` opened from conversation 1, `places` lit, the
/// `current`-th scrolled to.
pub(super) fn scene(
    kind: Kind,
    item: &str,
    topic: &str,
    places: &[ScenePlace],
    current: usize,
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
                .map(|(n, start_ms, at)| Place {
                    n: *n,
                    mark: Some(0),
                    start_ms: *start_ms,
                    at: at.clone(),
                })
                .collect(),
            current,
        }),
        land: true,
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
/// then `words`, the cited lines lit in them. It reaches 10 pt past the text
/// each side, so the lines around it are inset as much (`INSET`).
pub(in super::super) fn line(
    p: Paint,
    key: String,
    mark: &Mark,
    (head, lead): (Option<El>, Option<El>),
    words: El,
) -> El {
    let k = p.k;
    let badge = badge(p, mark.n, mark.now);
    let text = words
        .text_highlights(mark.lit.iter().map(|lit| highlight(p, lit)))
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
        .fill(tint(p, mark.now))
        .child(shown.key("shown"));
    match (mark.lands, mark.lit.iter().any(|lit| lit.lands)) {
        (true, true) => col().key(LINE_KEY).child(body),
        (true, false) => col().key(NOW_KEY).child(body),
        _ => body,
    }
}

/// How far a cited line's tint reaches past its text.
pub(in super::super) const INSET: f32 = 10.0;

/// A cited passage's tint, fainter on a place not shown.
fn tint(p: Paint, now: bool) -> rune::Color {
    let strength = if now { 1.0 } else { 0.4 };
    p.k.alpha(p.accent(), if p.k.dark { 0.13 } else { 0.09 } * strength)
}

/// Cited lines, lit like a search's matches; the scroll lands on the
/// current place's first.
fn highlight(p: Paint, lit: &Lit) -> TextHighlight {
    let k = p.k;
    let shown = TextHighlight::new(
        lit.at.clone(),
        k.alpha(p.accent(), if lit.now { 0.26 } else { 0.13 }),
    )
    .color(k.c.text_base)
    .pad(2.0);
    if lit.lands { shown.key(NOW_KEY) } else { shown }
}

/// The answer's `[n]`, fainter on a place not shown.
fn badge(p: Paint, n: u32, now: bool) -> El {
    let k = p.k;
    let accent = p.accent();
    let strength = if now { 1.0 } else { 0.4 };
    row()
        .key("mark")
        .h(16.0)
        .min_w(16.0)
        .px(3.0)
        .center()
        .radius(4.0)
        .fill(if now {
            k.alpha(accent, 0.14)
        } else {
            rune::Color::TRANSPARENT
        })
        .border(p.hair, k.alpha(accent, 0.4 * strength + 0.1))
        .child(
            text(n.to_string())
                .key("n")
                .font(rune::TextStyle {
                    size: 10.5,
                    line_height: 14.0,
                    weight: 600.0,
                    ..k.ty.small
                })
                .color(if now { accent } else { k.alpha(accent, 0.7) }),
        )
}

/// A dictation History lists, cited by the answer: its row's tint and the
/// answer's `[n]`.
pub(in super::super) fn row_mark(ui: &Ui, p: Paint, id: &str) -> Option<(rune::Color, El)> {
    let cited = cited(ui, Kind::Dictation, id)?;
    let place = cited.places.get(cited.current)?;
    Some((tint(p, true), badge(p, place.n, true)))
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
                        if let Some(key) = landing(cited) {
                            tx.scroll_key_to(key, place_top(ui));
                        }
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
    use crate::assistant::recordings::Lines;

    fn cited(n: u32, id: &str, offset: Option<u64>, lines: &[(u32, u64, Range<usize>)]) -> Ref {
        Ref {
            n,
            kind: Kind::Meeting,
            id: id.into(),
            offset_ms: offset,
            lines: lines
                .iter()
                .map(|(mark, start_ms, at)| Lines {
                    mark: *mark,
                    start_ms: *start_ms,
                    at: at.clone(),
                    ..Lines::default()
                })
                .collect(),
            ..Ref::default()
        }
    }

    /// Where each line's mark lights words: `(n, now, lit bytes)`.
    type Shown = Option<(u32, bool, Vec<Range<usize>>)>;

    /// One lit range, as [`shown`] lists them.
    fn lit(at: Range<usize>) -> Vec<Range<usize>> {
        vec![at]
    }

    fn shown(marks: &[Option<Mark>]) -> Vec<Shown> {
        marks
            .iter()
            .map(|mark| {
                mark.as_ref().map(|mark| {
                    (
                        mark.n,
                        mark.now,
                        mark.lit.iter().map(|l| l.at.clone()).collect(),
                    )
                })
            })
            .collect()
    }

    #[test]
    fn an_item_cited_in_two_places_lights_each_the_one_clicked_current() {
        // Turns "Ab. Cd." (0 s) and "Ef. Gh." (8 s), one per line.
        let lines = [(0, "Ab. Cd."), (8_000, "Ef. Gh.")];
        let refs = [
            cited(1, "a", Some(8_000), &[(0, 8_000, 12..15)]),
            cited(2, "b", Some(1_000), &[]),
            cited(3, "a", Some(0), &[(0, 0, 4..7), (1, 9_000, 8..11)]),
        ];
        let first = places("Кто ведёт найм?", &refs, &refs[0], Some(0));
        assert_eq!(first.topic, "Кто ведёт найм");
        let order: Vec<(u32, Option<u32>)> = first
            .places
            .iter()
            .map(|place| (place.n, place.mark))
            .collect();
        assert_eq!(order, [(3, Some(0)), (1, Some(0)), (3, Some(1))]);
        assert_eq!(first.current, 1);
        let marks = first.marks(&lines, 1);
        assert_eq!(
            shown(&marks),
            [
                Some((3, false, lit(4..7))),
                Some((1, true, vec![4..7, 0..3]))
            ]
        );
        assert!(marks[1].as_ref().unwrap().lit[0].lands);
        // The second [3] in the answer opens its own lines.
        let second = places("q", &refs, &refs[2], Some(1));
        assert_eq!(second.places[second.current].mark, Some(1));
        // A chip opens the first.
        let chip = places("q", &refs, &refs[2], None);
        assert_eq!(chip.current, 0);
    }

    /// A citation that named no line it has tints its passage with no
    /// words lit, and so does one whose words the transcript no longer
    /// holds.
    #[test]
    fn without_lines_only_the_passage_tints() {
        let lines = [(0, "Ab. Cd."), (8_000, "Ef. Gh.")];
        let refs = [
            cited(1, "a", Some(9_000), &[]),
            cited(2, "a", Some(0), &[(0, 1_000, 40..60)]),
        ];
        let marks = places("q", &refs, &refs[0], None).marks(&lines, 1);
        assert_eq!(
            shown(&marks),
            [Some((2, false, vec![])), Some((1, true, vec![]))]
        );
        assert!(marks[1].as_ref().unwrap().lands);
        // Notes, cited whole, have no place in the transcript.
        let notes = [cited(1, "a", None, &[])];
        assert!(places("q", &notes, &notes[0], None).places.is_empty());
    }

    #[test]
    fn lines_across_turns_light_in_each_and_land_on_the_first() {
        let lines = [(0, "Ab. Cd."), (8_000, "Ef. Gh.")];
        let refs = [cited(1, "a", Some(0), &[(0, 0, 4..11)])];
        let marks = places("q", &refs, &refs[0], None).marks(&lines, 1);
        assert_eq!(
            shown(&marks),
            [Some((1, true, lit(4..7))), Some((1, true, lit(0..3)))]
        );
        assert!(marks[0].as_ref().unwrap().lands && !marks[1].as_ref().unwrap().lands);
        // Bytes that split a character are no place to light.
        let word = [(0, "Ёж.")];
        let split = [cited(1, "a", Some(0), &[(0, 0, 1..4)])];
        let marks = places("q", &split, &split[0], None).marks(&word, 1);
        assert_eq!(shown(&marks), [Some((1, true, vec![]))]);
    }

    #[test]
    fn a_dictation_is_one_place_landed_on_by_its_row() {
        let mut dictation = cited(4, "7", None, &[]);
        dictation.kind = Kind::Dictation;
        let places = places("q", std::slice::from_ref(&dictation), &dictation, Some(0));
        assert_eq!(places.places.len(), 1);
        assert_eq!(landing(&places).as_deref(), Some("history-entry7"));
    }

    #[test]
    fn the_place_lands_a_third_of_the_way_down_any_pane() {
        let mut ui = Ui::fixture_base();
        assert_eq!(place_top(&ui), PLACE_TOP, "before the pane was measured");
        for (panel, top) in [(700.0, 216.0), (1_252.0, 400.0), (400.0, 116.0)] {
            ui.assistant.panel_h = panel;
            assert_eq!(place_top(&ui), top);
        }
    }
}
