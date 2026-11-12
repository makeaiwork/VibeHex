//! Places in the window an answer names, as links that open them.
//!
//! An answer writes a place in bold, as the window names it: a pane
//! (**Output**), a Labs feature (**Labs › Smart spacing**, or its title
//! alone), or a path into a pane (**General › Vocabulary**,
//! **Meetings › Settings › Detected calls**). Names match the window's own
//! words, in its language or in English, ignoring case; a path resolves to
//! its deepest part that is known, and a name that finds nothing, or more
//! than one place, stays bold.
//!
//! Opening one selects its pane (and its Settings view, or its Labs card),
//! lights the titles the path names inside it, section and row, the way a
//! cited passage is lit, and lands the deepest one a third of the way down.
//! A title folded under Advanced opens the fold. The light stays until
//! another pane is chosen, and Back to Assistant leads home.

use std::cell::RefCell;

use rune::{El, TextHighlight, Tx};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::sidebar::Pane;
use super::super::{general, labs};
use crate::assistant::changes::names::{Home, WHERE};
use crate::desktop::i18n::tr;

/// The titles a place link lights in its pane, outermost first, each in
/// the lowercase forms it may show in; and whether it still has to land.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Lit {
    pub(in super::super) titles: Vec<Vec<String>>,
    land: bool,
}

/// A place an answer can name.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Spot {
    to: To,
    /// Titles inside it to light, outermost first.
    lit: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum To {
    /// A pane, on its Settings view when it has one and `true`.
    Pane(Pane, bool),
    /// Labs, on a lab's card by its id.
    Labs(Option<&'static str>),
}

/// Every name the window gives a place, as it shows now.
pub(in super::super) struct Known {
    /// Lowercase names; `None`: more than one place has it.
    names: Vec<(String, Option<Spot>)>,
    /// Labs' titles, lowercase, for `Labs › …`.
    labs: Vec<(String, Spot)>,
}

impl Known {
    pub(in super::super) fn of(ui: &Ui) -> Known {
        let mut known = Known {
            names: Vec::new(),
            labs: Vec::new(),
        };
        for (pane, name) in panes() {
            if pane.shown(ui) {
                known.add(name, spot(To::Pane(pane, false), Vec::new()));
            }
        }
        known.add("Labs", spot(To::Labs(None), Vec::new()));
        let shown = labs::titles(ui);
        // The guide titles each lab as its row does in English.
        let english = crate::assistant::guide::articles()
            .iter()
            .filter(|article| article.is_lab())
            .filter_map(|article| {
                let (id, _) = shown
                    .iter()
                    .find(|(_, title)| *title == tr(&article.title))?;
                Some((*id, article.title.as_str()))
            });
        for (id, title) in shown.iter().copied().chain(english) {
            let lab = spot(To::Labs(Some(id)), vec![title.to_owned()]);
            known.labs.push((lower(title), lab.clone()));
            known.add(title, lab);
        }
        for &(_, path, name, home) in WHERE {
            let Some(pane) = pane_of(home).filter(|pane| pane.shown(ui)) else {
                continue;
            };
            let mut parts = path.split(" › ");
            if parts.next() != english_name(pane) {
                continue;
            }
            let parts: Vec<&str> = parts.collect();
            // A one-word name (Sounds, Reasoning) is too common to find a
            // place alone; written with its path it does.
            if let Some(last) = parts.last().filter(|last| last.contains(' ')) {
                known.add(last, within(pane, &parts));
            }
            if name.contains(' ') && !parts.contains(&name) {
                let mut path = parts.clone();
                path.push(name);
                known.add(name, within(pane, &path));
            }
        }
        known
    }

    /// `name` and its translation, for `spot`; a name a pane or Labs has
    /// keeps it, and one two places share finds neither.
    fn add(&mut self, name: &str, spot: Spot) {
        for name in [lower(name), lower(tr(name))] {
            match self.names.iter_mut().find(|(known, _)| *known == name) {
                None => self.names.push((name, Some(spot.clone()))),
                Some((_, Some(had))) if had.to == spot.to => {}
                Some((_, Some(had))) if had.lit.is_empty() => {}
                Some((_, found)) => *found = None,
            }
        }
    }

    fn find(&self, name: &str) -> Option<&Spot> {
        let name = lower(name);
        self.names
            .iter()
            .find(|(known, _)| *known == name)
            .and_then(|(_, spot)| spot.as_ref())
    }

    /// The place `name` names: its first part found alone, the rest the
    /// titles inside it. `None` when its first part names nothing.
    pub(in super::super) fn resolve(&self, name: &str) -> Option<Spot> {
        let name = name.trim().trim_end_matches([':', '.']);
        let parts: Vec<&str> = name.split('›').map(str::trim).collect();
        if parts.iter().any(|part| part.is_empty()) {
            return None;
        }
        let mut found = self.find(parts[0])?.clone();
        let rest = &parts[1..];
        match found.to {
            To::Labs(None) => {
                let lab = rest.first().map(|title| lower(title));
                if let Some((_, lab)) = self
                    .labs
                    .iter()
                    .find(|(title, _)| Some(title) == lab.as_ref())
                {
                    found = lab.clone();
                }
            }
            To::Labs(Some(_)) => {}
            To::Pane(pane, settings) => {
                let inside = within(pane, rest);
                found.to = To::Pane(pane, settings || inside.to == To::Pane(pane, true));
                found.lit.extend(inside.lit);
            }
        }
        Some(found)
    }
}

fn spot(to: To, lit: Vec<String>) -> Spot {
    Spot { to, lit }
}

/// `parts` inside `pane`: a leading Settings (or the other view's name)
/// picks its view, and the rest are lit.
fn within(pane: Pane, parts: &[&str]) -> Spot {
    let views: &[&str] = match pane {
        Pane::Meetings => &["Recordings", "Settings"],
        Pane::Files => &["Files", "Settings"],
        Pane::Assistant => &["Chat", "Settings"],
        _ => &[],
    };
    let view = parts
        .first()
        .and_then(|first| views.iter().position(|view| is(first, view)));
    let rest = if view.is_some() { &parts[1..] } else { parts };
    spot(
        To::Pane(pane, view == Some(1)),
        rest.iter().map(|part| (*part).to_owned()).collect(),
    )
}

/// The panes a place can be in, by their English names.
fn panes() -> Vec<(Pane, &'static str)> {
    let mut panes = vec![
        (Pane::General, "General"),
        (Pane::Output, "Output"),
        (Pane::History, "History"),
        (Pane::Meetings, "Meetings"),
        (Pane::Files, "Files"),
        (Pane::VoiceFilter, "Voice Filter"),
        (Pane::About, "About"),
        (Pane::Assistant, "Assistant"),
    ];
    #[cfg(hex_capture)]
    panes.extend([(Pane::Annotate, "Annotate"), (Pane::Captures, "Captures")]);
    panes
}

fn english_name(pane: Pane) -> Option<&'static str> {
    panes()
        .into_iter()
        .find(|(known, _)| *known == pane)
        .map(|(_, name)| name)
}

/// The pane a setting's home is, when it is one.
fn pane_of(home: Home) -> Option<Pane> {
    match home {
        Home::General => Some(Pane::General),
        Home::Output => Some(Pane::Output),
        Home::History => Some(Pane::History),
        Home::Meetings => Some(Pane::Meetings),
        Home::Files => Some(Pane::Files),
        #[cfg(hex_capture)]
        Home::Annotate => Some(Pane::Annotate),
        // Labs' own names come from its rows.
        _ => None,
    }
}

fn lower(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `shown` is `english`, in English or as the window translates it.
fn is(shown: &str, english: &str) -> bool {
    let shown = lower(shown);
    shown == lower(english) || shown == lower(tr(english))
}

/// Opens the place `name` names, when it still names one.
pub(super) fn open_named(ui: &mut Ui, name: &str, tx: &mut Tx) {
    if let Some(spot) = Known::of(ui).resolve(name) {
        open(ui, &spot, tx);
    }
}

/// A scene's: `name` opened, landing on its first frame.
pub(super) fn scene(ui: &mut Ui, name: &str) {
    open_named(ui, name, &mut Tx::default());
    if let Some(lit) = ui.place.as_mut() {
        lit.land = true;
    }
}

/// Opens `spot`: its pane, view, or Labs card, its titles lit, and Back to
/// Assistant to the conversation it was named in.
pub(super) fn open(ui: &mut Ui, spot: &Spot, tx: &mut Tx) {
    let from = ui
        .assistant
        .open
        .map(|id| (id, ui.assistant.scroll_y.get()));
    match spot.to {
        To::Labs(lab) => {
            let link = labs::LabsLink {
                lab: lab.map(str::to_owned),
                ..labs::LabsLink::default()
            };
            labs::reveal(ui, &link, tx);
        }
        To::Pane(pane, settings) => {
            ui.select_pane(pane, tx);
            match pane {
                Pane::Meetings => {
                    ui.meetings.view = if settings {
                        super::super::meetings::View::Settings
                    } else {
                        super::super::meetings::View::Recordings
                    };
                }
                Pane::Files if settings => ui.files.view = super::super::files::View::Settings,
                Pane::Assistant if settings => ui.assistant.view = super::View::Settings,
                _ => {}
            }
            if pane != Pane::Assistant
                && let Some((conversation, scroll_y)) = from
            {
                super::jump::came_for_place(ui, conversation, scroll_y);
            }
        }
    }
    ui.place = (!spot.lit.is_empty()).then(|| Lit {
        titles: spot
            .lit
            .iter()
            .map(|title| {
                let mut forms = vec![lower(title), lower(tr(title))];
                forms.dedup();
                forms
            })
            .collect(),
        land: true,
    });
    unfold(ui);
    land(ui, tx);
}

/// Opens the pane's Advanced fold when what the place lights is under it.
fn unfold(ui: &mut Ui) {
    let pane = ui.pane;
    let Some(deepest) = ui.place.as_ref().map(|lit| lit.titles.len() - 1) else {
        return;
    };
    if ui.labs_open
        || !matches!(pane, Pane::General | Pane::Meetings)
        || general::advanced_open(ui, pane)
    {
        return;
    }
    let folded = probe(ui);
    if folded == Some(deepest) {
        return;
    }
    ui.advanced.push(pane);
    if probe(ui) <= folded {
        ui.advanced.retain(|open| *open != pane);
    }
}

/// The deepest title the pane as it is now lights.
fn probe(ui: &Ui) -> Option<usize> {
    let titles = ui.place.as_ref()?.titles.clone();
    let mut layer = Vec::new();
    let p = Paint::new(false, 1.0);
    lighting(&titles, || {
        super::super::sidebar::body(ui, p, true, &mut layer)
    })
    .1
}

/// Scrolls the place's titles in, outermost first, so the deepest one
/// that shows lands.
fn land(ui: &mut Ui, tx: &mut Tx) {
    let top = super::jump::place_top(ui);
    let Some(lit) = ui.place.as_mut() else {
        return;
    };
    lit.land = false;
    for depth in 0..lit.titles.len() {
        tx.scroll_key_to(lit_key(depth), top);
    }
}

/// Lands an opened place once its pane shows (a scene's first frame).
/// True when it scrolled.
pub(super) fn poll(ui: &mut Ui, tx: &mut Tx) -> bool {
    if !ui.place.as_ref().is_some_and(|lit| lit.land) {
        return false;
    }
    land(ui, tx);
    true
}

thread_local! {
    /// While a pane builds: the titles lit, and the deepest one built.
    static LIGHTING: RefCell<(Vec<Vec<String>>, Option<usize>)> = const {
        RefCell::new((Vec::new(), None))
    };
}

/// Builds with `titles` lit, and says the deepest one it lit.
pub(in super::super) fn lighting<R>(
    titles: &[Vec<String>],
    build: impl FnOnce() -> R,
) -> (R, Option<usize>) {
    let before = LIGHTING.with_borrow_mut(|now| std::mem::replace(now, (titles.to_vec(), None)));
    let built = build();
    let (_, found) = LIGHTING.with_borrow_mut(|now| std::mem::replace(now, before));
    (built, found)
}

/// The key a lit title at `depth` lands by.
fn lit_key(depth: usize) -> String {
    format!("place-lit-{depth}")
}

/// `label`, the text of `title`, its words lit when an opened place names
/// it: tinted like a cited line, and what the scroll lands on.
pub(in super::super) fn lit(p: Paint, title: &str, label: El) -> El {
    let depth = LIGHTING.with_borrow_mut(|(titles, found)| {
        if titles.is_empty() {
            return None;
        }
        let title = lower(title);
        let depth = titles.iter().rposition(|forms| forms.contains(&title))?;
        *found = (*found).max(Some(depth));
        Some(depth)
    });
    let Some(depth) = depth else {
        return label;
    };
    let k = p.k;
    label.text_highlights([
        TextHighlight::new(0..title.len(), k.alpha(p.accent(), 0.26))
            .color(k.c.text_base)
            .pad(2.0)
            .key(lit_key(depth)),
    ])
}

#[cfg(test)]
#[path = "places_tests.rs"]
mod tests;
