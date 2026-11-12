//! A vocabulary card ("Add 3 terms", the Web board): each term the answer
//! suggests with its "sounds like" spellings, a checkbox, and Edit, which
//! renames the term and changes its spellings in the row itself. Add N
//! terms adds the checked ones through `hex vocabulary add`'s own path
//! (`control::edit_vocabulary`); a term the vocabulary has by then is left
//! out with why, never merged into, so Undo takes out exactly the terms the
//! card added, while it holds (owner decision 2).

use rune::{El, Icon, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, TextInput};
use rune_theme_opencode::icons as theme;
use serde_json::{Map, Value, json};

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint, PushSize};
use super::super::icons;
use super::card::{self, INDENT, Look, Shell, push};
use super::changes::{self as act, At};
use crate::assistant::changes::state::{self, Row, Shown};
use crate::assistant::changes::vocab::{self, Term};
use crate::assistant::changes::{Card, Decision, Item};
use crate::control::{ControlCommand, edit_vocabulary};
use crate::desktop::i18n::{tr, tr_fill, tr_named, tr_window};
use crate::dictation::vocabulary::{self, VocabularyEntry};

/// The term column's width, as the board has it.
const TERM_WIDTH: f32 = 120.0;

/// The card at `at`; `primary` when Add is the screen's one.
pub(super) fn view(ui: &Ui, p: Paint, at: At, primary: bool) -> Option<El> {
    let card = act::card(ui, at)?;
    let shown = act::shown(ui, at)?;
    let live = act::live(ui, at);
    let (shell, glyph): (Shell, &'static Icon) = match shown {
        Shown::Proposed { .. } => (Shell::Open, &icons::BOOK),
        Shown::Applied { .. } => (Shell::Done, &theme::CHECK),
        _ => (Shell::Off, &icons::BOOK),
    };
    let name = match card.items.len() {
        1 => tr("Add 1 term").to_owned(),
        all => tr_fill("Add {} terms", &all.to_string()),
    };
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
    let mut words = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak)
        .clip_soft(0.0);
    for (index, item) in card.items.iter().enumerate() {
        if index > 0 {
            words = words.child(
                el().key(format!("rule{index}"))
                    .h(p.hair)
                    .w_full()
                    .fill(p.k.c.border_weak),
            );
        }
        words = words.child(line(
            ui,
            p,
            at,
            index,
            item,
            &state::row(card, index, &live),
        ));
    }
    let key = format!("card{}-{}", at.turn, at.card);
    Some(
        card::shell_of(p, &key, shell, &name)
            .child(top)
            .child(row().key("list").pl(INDENT).child(words))
            .child(card::buttons(foot(p, at, card, &shown, primary))),
    )
}

/// One term: its checkbox (or how it went), the term, what it sounds like,
/// and Edit; while edited, the two fields and Done.
fn line(ui: &Ui, p: Paint, at: At, index: usize, item: &Item, shown: &Row) -> El {
    let term = vocab::terms(&item.to)
        .into_iter()
        .next()
        .unwrap_or_default();
    let lead = match shown {
        Row::Open { checked, .. } => {
            super::tick::tick(p, *checked, &term.term, move |ui: &mut Ui| {
                act::toggle(ui, at, index)
            })
        }
        Row::Applied => mark(&theme::CHECK, p.success()),
        Row::Failed(_) => mark(&theme::CIRCLE_EXCLAMATION, p.danger()),
        Row::Left | Row::Downloading { .. } => mark(&theme::DASH, p.k.c.icon_faint),
    };
    let base = row()
        .key(format!("term{index}"))
        .role("group")
        .label(term.term.clone())
        .px(10.0)
        .py(7.0)
        .gap(10.0)
        .center_y()
        .child(lead);
    if ui.assistant.vocab_edit == Some((at, index)) {
        return base
            .child(
                TextInput::new(&ui.assistant.vocab_term)
                    .placeholder(tr("Term"))
                    .label(tr("Term"))
                    .width(Some(TERM_WIDTH + 20.0))
                    .on_edit(|ui: &mut Ui| &mut ui.assistant.vocab_term)
                    .build(p.k)
                    .key("term"),
            )
            .child(
                TextInput::new(&ui.assistant.vocab_sounds)
                    .placeholder(tr("Sounds like, comma-separated"))
                    .label(tr("Sounds like, comma-separated"))
                    .width(None)
                    .on_edit(|ui: &mut Ui| &mut ui.assistant.vocab_sounds)
                    .build(p.k)
                    .key("sounds-like")
                    .grow(1.0)
                    .shrink(),
            )
            .child(push(p, "done", tr("Done"), Look::Ghost, |ui, _| done(ui)));
    }
    let off = matches!(shown, Row::Left);
    let sounds = if term.sounds_like.is_empty() {
        String::new()
    } else {
        tr_fill("sounds like: {}", &term.sounds_like.join(", "))
    };
    let mut about = col()
        .key("about")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(1.0)
        .child(
            text(sounds)
                .key("sounds-like")
                .font(p.k.ty.small)
                .color(p.muted())
                .wrap(),
        );
    if let Row::Failed(why) = shown {
        about = about.child(
            text(tr_fill("Not added: {}", why))
                .key("failed")
                .font(p.k.ty.small)
                .color(p.danger())
                .wrap(),
        );
    }
    let mut words = text(term.term.clone())
        .key("name")
        .font(rune::TextStyle {
            weight: 520.0,
            ..p.k.ty.ui
        })
        .color(if off { p.faint() } else { p.text() })
        .max_lines(1);
    if off {
        words = words.strike(0, term.term.len(), None);
    }
    let mut line = base
        .child(el().key("term").w(TERM_WIDTH).child(words))
        .child(about);
    if matches!(shown, Row::Open { .. }) {
        line = line.child(push(p, "edit", tr("Edit"), Look::Ghost, move |ui, _| {
            edit(ui, at, index)
        }));
    }
    line
}

fn mark(glyph: &'static Icon, color: rune::Color) -> El {
    el().key("mark")
        .size(16.0, 16.0)
        .center()
        .child(icon(glyph, 13.0).key("glyph").color(color))
}

/// Add N terms and Not now; after, how it went, with Undo while it holds.
fn foot(p: Paint, at: At, card: &Card, shown: &Shown, primary: bool) -> Vec<El> {
    let space = || el().key("space").grow(1.0);
    match shown {
        Shown::Proposed { .. } => {
            let selected = card.items.iter().filter(|item| item.included).count();
            let label = match selected {
                1 => tr("Add 1 term").to_owned(),
                count => tr_fill("Add {} terms", &count.to_string()),
            };
            let variant = if primary {
                ButtonVariant::Contrast
            } else {
                ButtonVariant::Neutral
            };
            let add = Button::new(label)
                .variant(variant)
                .disabled(selected == 0)
                .on_press(move |ui: &mut Ui| act::apply(ui, at));
            vec![
                add.build_small_push(p.k).key("apply"),
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
            let (done, total) = state::applied_count(card);
            let words = if done == total {
                tr("Added to Vocabulary").to_owned()
            } else {
                tr_named(
                    "{done} of {total} terms added",
                    &[("done", &done.to_string()), ("total", &total.to_string())],
                )
            };
            let mut row = vec![
                status(words, p.success()),
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
            status(tr_fill("Not added: {}", why), p.danger()),
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
                tr("Taken out again")
            } else {
                tr("Left as it was")
            };
            vec![
                status(words.to_owned(), p.muted()),
                space(),
                push(p, "anyway", tr("Add anyway"), Look::Ghost, move |ui, _| {
                    act::apply(ui, at)
                }),
            ]
        }
    }
}

fn status(words: String, color: rune::Color) -> El {
    text(words)
        .key("status")
        .font(DESCRIPTION)
        .color(color)
        .wrap()
        .shrink()
}

/// Edit: the row's term and spellings in two fields; another row's edit
/// is kept first.
pub(super) fn edit(ui: &mut Ui, at: At, index: usize) {
    done(ui);
    let Some(item) = act::card(ui, at).and_then(|card| card.items.get(index)) else {
        return;
    };
    let term = vocab::terms(&item.to)
        .into_iter()
        .next()
        .unwrap_or_default();
    ui.assistant.vocab_term = rune::TextEdit::new(term.term);
    ui.assistant.vocab_sounds = rune::TextEdit::new(term.sounds_like.join(", "));
    ui.assistant.vocab_edit = Some((at, index));
}

/// Done: the edited term back into its row. An emptied term keeps the
/// one before.
pub(super) fn done(ui: &mut Ui) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let Some((at, index)) = ui.assistant.vocab_edit.take() else {
        return;
    };
    let name = ui.assistant.vocab_term.text.trim().to_owned();
    let sounds_like = vocabulary::parse_sounds_like(&ui.assistant.vocab_sounds.text);
    act::set(ui, at, |card| {
        let Some(item) = card.items.get_mut(index) else {
            return;
        };
        let before = vocab::terms(&item.to)
            .into_iter()
            .next()
            .unwrap_or_default();
        let term = Term {
            term: if name.is_empty() { before.term } else { name },
            sounds_like,
        };
        item.to = term.value();
    });
}

/// Adds `item`'s terms to `list` as `hex vocabulary add` would, leaving
/// out a term the list has; the terms it added.
pub(super) fn add(list: &mut Vec<VocabularyEntry>, item: &mut Item) -> Vec<String> {
    let mut added = Vec::new();
    let mut why = None;
    for term in vocab::terms(&item.to) {
        if vocabulary::contains(list, &term.term) {
            why = Some(tr("Already in Vocabulary").to_owned());
            continue;
        }
        let command = ControlCommand::VocabularyAdd {
            term: term.term.clone(),
            sounds_like: term.sounds_like.clone(),
        };
        match edit_vocabulary(list, &command) {
            Ok(_) => added.push(term.term),
            Err(error) => why = Some(error.message),
        }
    }
    item.failed = if added.is_empty() { why } else { None };
    added
}

/// Add N terms: the checked ones in one save.
pub(super) fn apply(ui: &mut Ui, at: At, mut card: Card) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let mut list = ui.drafted().vocabulary;
    let mut added: Vec<String> = Vec::new();
    for item in &mut card.items {
        item.failed = None;
        if item.included {
            added.extend(add(&mut list, item));
        }
    }
    let decision = if added.is_empty() {
        let why = card.items.iter().find_map(|item| item.failed.clone());
        Err(why.unwrap_or_else(|| tr("Nothing was selected").to_owned()))
    } else if ui.update(|settings| settings.vocabulary = list) {
        Ok(Decision::Applied {
            before: Map::new(),
            after: added_map(added),
            at_ms: ui.assistant.now_ms(),
        })
    } else {
        Err(ui.error.clone().unwrap_or_default())
    };
    act::decide(ui, at, card, decision);
}

/// What a save records of the terms it added, for Undo.
pub(super) fn added_map(added: Vec<String>) -> Map<String, Value> {
    Map::from_iter([(vocab::KEY.to_owned(), json!(added))])
}

/// Undo: takes out exactly the terms the save added, wherever they are
/// in the list by now.
pub(super) fn take_back(ui: &mut Ui, after: &Map<String, Value>) -> Result<(), String> {
    let Some(Value::Array(terms)) = after.get(vocab::KEY) else {
        return Ok(());
    };
    let mut list = ui.drafted().vocabulary;
    for term in terms.iter().filter_map(Value::as_str) {
        let command = ControlCommand::VocabularyRemove {
            term: term.to_owned(),
        };
        edit_vocabulary(&mut list, &command).map_err(|error| error.message)?;
    }
    if ui.update(|settings| settings.vocabulary = list) {
        Ok(())
    } else {
        Err(ui.error.clone().unwrap_or_default())
    }
}

/// Show in Vocabulary: General, with the Vocabulary sheet open.
fn show(ui: &mut Ui, tx: &mut rune::Tx) {
    ui.select_pane(super::super::sidebar::Pane::General, tx);
    super::super::vocabulary::open(ui);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::CardKind;
    use crate::assistant::store::Turn;

    fn term(term: &str, sounds_like: &[&str]) -> Item {
        Item {
            key: vocab::KEY.into(),
            from: Value::Null,
            to: json!({"term": term, "sounds_like": sounds_like}),
            included: true,
            ..Item::default()
        }
    }

    fn with_terms(items: Vec<Item>) -> (Ui, At) {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        ui.settings.vocabulary = vec![VocabularyEntry {
            term: "Docker".into(),
            sounds_like: vec!["докер".into()],
        }];
        ui.vocabulary.reload(&ui.settings.vocabulary);
        let id = ui.assistant.store.start("q", None, None);
        ui.assistant.store.push(
            id,
            Turn {
                question: "q".into(),
                answer: "a".into(),
                cards: vec![Card {
                    kind: CardKind::Vocabulary,
                    items,
                    ..Card::default()
                }],
                ..Turn::default()
            },
        );
        ui.assistant.open = Some(id);
        act::follow_open(&mut ui);
        let at = At {
            conversation: id,
            turn: 0,
            card: 0,
        };
        (ui, at)
    }

    fn terms(ui: &Ui) -> Vec<String> {
        let list = &ui.settings.vocabulary;
        list.iter().map(|entry| entry.term.clone()).collect()
    }

    /// Checked terms go in through the CLI's path; one the list has by now
    /// is left out with why; Undo takes out exactly the added ones.
    #[test]
    fn add_n_terms_adds_the_checked_ones_and_undo_takes_them_out() {
        let (mut ui, at) = with_terms(vec![
            term("Kubernetes", &["кубернетис", "кубер"]),
            term("docker", &["доккер"]),
            term("Grafana", &["графана"]),
        ]);
        act::toggle(&mut ui, at, 2);
        act::apply(&mut ui, at);
        assert_eq!(terms(&ui), ["Docker", "Kubernetes"]);
        assert_eq!(
            ui.settings.vocabulary[0].sounds_like,
            ["докер"],
            "a term there is never merged into"
        );
        let card = act::card(&ui, at).unwrap().clone();
        assert_eq!(state::applied_count(&card), (1, 2));
        assert!(card.items[1].failed.is_some());
        assert_eq!(act::shown(&ui, at), Some(Shown::Applied { undo: true }));
        // A term added by hand since stays.
        ui.update(|settings| {
            settings.vocabulary.push(VocabularyEntry {
                term: "Prometheus".into(),
                sounds_like: Vec::new(),
            })
        });
        act::undo(&mut ui, at);
        assert_eq!(terms(&ui), ["Docker", "Prometheus"]);
        assert_eq!(act::shown(&ui, at), Some(Shown::Undone));
    }

    /// Edit renames the term and changes its spellings in the row; Add
    /// saves what the row says.
    #[test]
    fn edit_renames_a_term_in_its_row() {
        let (mut ui, at) = with_terms(vec![term("Kubernetis", &["кубер"])]);
        edit(&mut ui, at, 0);
        assert_eq!(ui.assistant.vocab_term.text, "Kubernetis");
        assert_eq!(ui.assistant.vocab_sounds.text, "кубер");
        ui.assistant.vocab_term = rune::TextEdit::new("Kubernetes".to_owned());
        ui.assistant.vocab_sounds = rune::TextEdit::new("кубер, кубернетис".to_owned());
        // Add keeps the edit still open first.
        act::apply(&mut ui, at);
        assert!(ui.assistant.vocab_edit.is_none());
        let added = ui.settings.vocabulary.last().unwrap().clone();
        assert_eq!(added.term, "Kubernetes");
        assert_eq!(added.sounds_like, ["кубер", "кубернетис"]);
        // An emptied term keeps the one before.
        let (mut ui, at) = with_terms(vec![term("Grafana", &[])]);
        edit(&mut ui, at, 0);
        ui.assistant.vocab_term = rune::TextEdit::new("  ".to_owned());
        done(&mut ui);
        let item = &act::card(&ui, at).unwrap().items[0];
        assert_eq!(vocab::terms(&item.to)[0].term, "Grafana");
    }

    /// Nothing checked, or every term there already: nothing is saved.
    #[test]
    fn nothing_to_add_saves_nothing() {
        let (mut ui, at) = with_terms(vec![term("DOCKER", &[])]);
        let saved = ui.saved;
        act::apply(&mut ui, at);
        assert_eq!(ui.saved, saved);
        assert!(matches!(
            act::card(&ui, at).unwrap().decision,
            Decision::Failed { .. }
        ));
    }
}
