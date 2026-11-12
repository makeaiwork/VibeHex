//! A vocabulary edit card ("Change 1 entry"): each entry already in
//! Vocabulary that the answer changes, with a checkbox, its term (old →
//! new when renamed), and its spellings: kept ones plain, removed ones
//! struck, added ones on the accent tint. Change saves the checked ones
//! through `hex vocabulary edit`'s own path (`control::edit_vocabulary`),
//! each entry rewritten where it stands; Undo writes each back exactly as
//! it was, unless it changed since.

use rune::{El, Icon, col, el, row, text};
use rune_kit::{Button, ButtonVariant};
use rune_theme_opencode::icons as theme;
use serde_json::{Map, Value, json};

use super::super::Ui;
use super::super::controls::{Paint, PushSize};
use super::super::icons;
use super::card::{self, INDENT, Look, Shell, push};
use super::changes::{self as act, At};
use super::vocab::{mark, show, status};
use crate::assistant::changes::state::{self, Row, Shown};
use crate::assistant::changes::vocab::Term;
use crate::assistant::changes::vocab_edit::{self, entry};
use crate::assistant::changes::{Card, Decision, Item};
use crate::control::{ControlCommand, edit_vocabulary};
use crate::desktop::i18n::{tr, tr_fill, tr_window};

/// The term column's width, as the terms card has it.
const TERM_WIDTH: f32 = 120.0;

/// The card at `at`; `primary` when Change is the screen's one.
pub(super) fn view(ui: &Ui, p: Paint, at: At, primary: bool) -> Option<El> {
    let card = act::card(ui, at)?;
    let shown = act::shown(ui, at)?;
    let live = act::live(ui, at);
    let (shell, glyph): (Shell, &'static Icon) = match shown {
        Shown::Proposed { .. } => (Shell::Open, &icons::BOOK),
        Shown::Applied { .. } => (Shell::Done, &theme::CHECK),
        _ => (Shell::Off, &icons::BOOK),
    };
    let name = entries(card.items.len());
    let ink = if shell == Shell::Off {
        p.muted()
    } else {
        p.text()
    };
    let top = row()
        .key("top")
        .gap(10.0)
        .center_y()
        .child(card::tile(p, glyph, shell))
        .child(card::header_in(
            p,
            &format!("{} › {}", tr_window("General"), tr_window("Vocabulary")),
            &name,
            None,
            ink,
        ));
    let mut list = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak)
        .clip_soft(0.0);
    for (index, item) in card.items.iter().enumerate() {
        if index > 0 {
            list = list.child(
                el().key(format!("rule{index}"))
                    .h(p.hair)
                    .w_full()
                    .fill(p.k.c.border_weak),
            );
        }
        list = list.child(line(p, at, index, item, &state::row(card, index, &live)));
    }
    let key = format!("card{}-{}", at.turn, at.card);
    Some(
        card::shell_of(p, &key, shell, &name)
            .child(top)
            .child(row().key("list").pl(INDENT).child(list))
            .child(card::buttons(foot(p, at, card, &shown, primary))),
    )
}

/// "Change 1 entry", "Change 3 entries".
fn entries(count: usize) -> String {
    match count {
        1 => tr("Change 1 entry").to_owned(),
        count => tr_fill("Change {} entries", &count.to_string()),
    }
}

/// One entry: its checkbox (or how it went), its term, its spellings
/// before → after, and why.
fn line(p: Paint, at: At, index: usize, item: &Item, shown: &Row) -> El {
    let before = entry(&item.from).unwrap_or_default();
    let after = entry(&item.to).unwrap_or_default();
    let lead = match shown {
        Row::Open { checked, .. } => {
            super::tick::tick(p, *checked, &before.term, move |ui: &mut Ui| {
                act::toggle(ui, at, index)
            })
        }
        Row::Applied => mark(&theme::CHECK, p.success()),
        Row::Failed(_) => mark(&theme::CIRCLE_EXCLAMATION, p.danger()),
        Row::Left | Row::Downloading { .. } => mark(&theme::DASH, p.k.c.icon_faint),
    };
    let off = matches!(shown, Row::Left);
    let term = if before.term == after.term {
        let mut words = text(before.term.clone())
            .key("name")
            .font(rune::TextStyle {
                weight: 520.0,
                ..p.k.ty.ui
            })
            .color(if off { p.faint() } else { p.text() })
            .max_lines(1);
        if off {
            words = words.strike(0, before.term.len(), None);
        }
        el().key("term").w(TERM_WIDTH).child(words)
    } else {
        card::change_chips(p, "", &json!(before.term), &json!(after.term), off)
    };
    let mut about = col()
        .key("about")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(4.0)
        .child(spellings(p, &before, &after, off));
    let why = item.why.trim();
    if !why.is_empty() {
        about = about.child(
            text(why.to_owned())
                .key("why")
                .font(p.k.ty.small)
                .color(p.muted())
                .wrap(),
        );
    }
    if let Row::Failed(why) = shown {
        about = about.child(
            text(tr_fill("Not changed: {}", why))
                .key("failed")
                .font(p.k.ty.small)
                .color(p.danger())
                .wrap(),
        );
    }
    row()
        .key(format!("entry{index}"))
        .role("group")
        .label(before.term.clone())
        .px(10.0)
        .py(7.0)
        .gap(10.0)
        .items_start()
        .child(lead)
        .child(term)
        .child(about)
}

/// "sounds like:" and each spelling: kept plain, removed struck, added
/// new; none left reads "no spellings".
fn spellings(p: Paint, before: &Term, after: &Term, off: bool) -> El {
    let same = |one: &str, other: &str| one.to_lowercase() == other.to_lowercase();
    let mut chips = row()
        .key("spellings")
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .child(
            text(tr("sounds like:"))
                .key("label")
                .font(p.k.ty.small)
                .color(p.muted()),
        );
    for (index, spelling) in before.sounds_like.iter().enumerate() {
        let kept = after.sounds_like.iter().any(|now| same(now, spelling));
        chips = chips.child(card::value_chip(
            p,
            &format!("was{index}"),
            spelling,
            false,
            !kept || off,
        ));
    }
    for (index, spelling) in after
        .sounds_like
        .iter()
        .filter(|now| !before.sounds_like.iter().any(|was| same(was, now)))
        .enumerate()
    {
        chips = chips.child(card::value_chip(
            p,
            &format!("new{index}"),
            spelling,
            !off,
            off,
        ));
    }
    if before.sounds_like.is_empty() && after.sounds_like.is_empty() {
        chips = chips.child(
            text(tr("no spellings"))
                .key("none")
                .font(p.k.ty.small)
                .color(p.faint()),
        );
    }
    chips
}

/// Change and Not now; after, how it went, with Undo while it holds.
fn foot(p: Paint, at: At, card: &Card, shown: &Shown, primary: bool) -> Vec<El> {
    let space = || el().key("space").grow(1.0);
    match shown {
        Shown::Proposed { .. } => {
            let selected = card.items.iter().filter(|item| item.included).count();
            let variant = if primary {
                ButtonVariant::Contrast
            } else {
                ButtonVariant::Neutral
            };
            let change = Button::new(entries(selected))
                .variant(variant)
                .disabled(selected == 0)
                .on_press(move |ui: &mut Ui| act::apply(ui, at));
            vec![
                change.build_small_push(p.k).key("apply"),
                push(
                    p,
                    "not-now",
                    tr("Not now"),
                    Look::Secondary,
                    move |ui, _| act::decline(ui, at),
                ),
            ]
        }
        Shown::Applied { undo } => {
            let mut row = vec![
                status(tr("Changed in Vocabulary").to_owned(), p.success()),
                space(),
                push(p, "show", tr("Show in Vocabulary"), Look::Ghost, show),
            ];
            if *undo {
                row.push(push(p, "undo", tr("Undo"), Look::Ghost, move |ui, _| {
                    act::undo(ui, at)
                }));
            }
            row
        }
        Shown::Failed(why) => vec![
            status(tr_fill("Not changed: {}", why), p.danger()),
            space(),
            push(
                p,
                "retry",
                tr("Try again"),
                Look::Secondary,
                move |ui, _| act::apply(ui, at),
            ),
        ],
        _ => {
            let words = if *shown == Shown::Undone {
                tr("Put back as it was")
            } else {
                tr("Left as it was")
            };
            vec![
                status(words.to_owned(), p.muted()),
                space(),
                push(
                    p,
                    "anyway",
                    tr("Change anyway"),
                    Look::Ghost,
                    move |ui, _| act::apply(ui, at),
                ),
            ]
        }
    }
}

/// The command that rewrites `from`'s entry as `to`.
fn command(from: &Term, to: &Term) -> ControlCommand {
    ControlCommand::VocabularyEdit {
        term: from.term.clone(),
        rename: (from.term != to.term).then(|| to.term.clone()),
        sounds_like: Some(to.sounds_like.clone()),
    }
}

/// Change: the checked entries in one save. Each must still be as the
/// card showed it; one changed since is left out with why.
pub(super) fn apply(ui: &mut Ui, at: At, mut card: Card) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let mut list = ui.drafted().vocabulary;
    let mut done: Vec<Value> = Vec::new();
    for item in &mut card.items {
        item.failed = None;
        if !item.included {
            continue;
        }
        let (from, to) = (
            entry(&item.from).unwrap_or_default(),
            entry(&item.to).unwrap_or_default(),
        );
        let now = list
            .iter()
            .find(|entry| entry.term.to_lowercase() == from.term.to_lowercase());
        if now.map(|now| vocab_edit::value(now)) != Some(item.from.clone()) {
            item.failed = Some(tr("Changed since").to_owned());
            continue;
        }
        match edit_vocabulary(&mut list, &command(&from, &to)) {
            Ok(_) => done.push(json!({"from": item.from, "to": item.to})),
            Err(error) => item.failed = Some(error.message),
        }
    }
    let decision = if done.is_empty() {
        let why = card.items.iter().find_map(|item| item.failed.clone());
        Err(why.unwrap_or_else(|| tr("Nothing was selected").to_owned()))
    } else if ui.update(|settings| settings.vocabulary = list) {
        Ok(Decision::Applied {
            before: Map::new(),
            after: Map::from_iter([(vocab_edit::KEY.to_owned(), Value::Array(done))]),
            at_ms: ui.assistant.now_ms(),
        })
    } else {
        Err(ui.error.clone().unwrap_or_default())
    };
    act::decide(ui, at, card, decision);
}

/// Undo: each entry the save changed, written back exactly as it was,
/// where it stands; none is touched when one changed since.
pub(super) fn take_back(ui: &mut Ui, after: &Map<String, Value>) -> Result<(), String> {
    let Some(Value::Array(done)) = after.get(vocab_edit::KEY) else {
        return Ok(());
    };
    let mut list = ui.drafted().vocabulary;
    for change in done {
        let (Some(was), Some(now)) = (entry(&change["from"]), entry(&change["to"])) else {
            continue;
        };
        let held = list
            .iter()
            .find(|entry| entry.term.to_lowercase() == now.term.to_lowercase())
            .map(vocab_edit::value);
        if held.as_ref() != Some(&change["to"]) {
            return Err(tr_fill("“{}” changed since", &now.term));
        }
        edit_vocabulary(&mut list, &command(&now, &was)).map_err(|error| error.message)?;
    }
    if ui.update(|settings| settings.vocabulary = list) {
        Ok(())
    } else {
        Err(ui.error.clone().unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::CardKind;
    use crate::assistant::store::Turn;
    use crate::dictation::vocabulary::VocabularyEntry;

    fn entry_of(term: &str, sounds_like: &[&str]) -> VocabularyEntry {
        VocabularyEntry {
            term: term.into(),
            sounds_like: sounds_like.iter().map(|s| (*s).into()).collect(),
        }
    }

    fn with_edit() -> (Ui, At) {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        ui.settings.vocabulary = vec![
            entry_of("Kubernetes", &["кубер"]),
            entry_of("AI cleanup", &["эй ай клинап", "ай"]),
            entry_of("Grafana", &[]),
        ];
        ui.vocabulary.reload(&ui.settings.vocabulary);
        let card = Card {
            kind: CardKind::VocabularyEdit,
            items: vec![Item {
                key: vocab_edit::KEY.into(),
                from: json!({"term":"AI cleanup","sounds_like":["эй ай клинап","ай"]}),
                to: json!({"term":"AI cleanup","sounds_like":["эй ай клинап"]}),
                included: true,
                ..Item::default()
            }],
            ..Card::default()
        };
        let id = ui.assistant.store.start("Why AI cleanup?", None, None);
        ui.assistant.store.push(
            id,
            Turn {
                question: "Why AI cleanup?".into(),
                cards: vec![card],
                ..Turn::default()
            },
        );
        ui.assistant.open = Some(id);
        act::follow_open(&mut ui);
        (
            ui,
            At {
                conversation: id,
                turn: 0,
                card: 0,
            },
        )
    }

    /// Change rewrites the entry where it stands; Undo writes it back
    /// exactly.
    #[test]
    fn change_edits_the_entry_in_place_and_undo_puts_it_back() {
        let (mut ui, at) = with_edit();
        let before = ui.settings.vocabulary.clone();
        act::apply(&mut ui, at);
        assert_eq!(
            ui.settings.vocabulary[1],
            entry_of("AI cleanup", &["эй ай клинап"])
        );
        assert_eq!(ui.settings.vocabulary.len(), 3, "nothing moved or added");
        assert_eq!(act::shown(&ui, at), Some(Shown::Applied { undo: true }));
        act::undo(&mut ui, at);
        assert_eq!(ui.settings.vocabulary, before);
        assert_eq!(act::shown(&ui, at), Some(Shown::Undone));
    }

    /// An entry changed by hand after the card was written is left alone.
    #[test]
    fn an_entry_changed_since_is_left_alone() {
        let (mut ui, at) = with_edit();
        ui.settings.vocabulary[1] = entry_of("AI cleanup", &["ай"]);
        ui.vocabulary.reload(&ui.settings.vocabulary);
        act::apply(&mut ui, at);
        assert_eq!(ui.settings.vocabulary[1], entry_of("AI cleanup", &["ай"]));
        assert!(matches!(act::shown(&ui, at), Some(Shown::Failed(_))));
    }
}
