//! Setup's one list of changes (the Setup board): "Suggested changes,
//! 6 changes, 5 selected", a row per setting with its checkbox, where it
//! lives, why, and its value now and after, then Not now and the screen's
//! Primary, "Apply 5 changes". Vocabulary's terms are a row each, as the
//! Vocabulary card has them (checkbox, term, how it sounds, Edit), under
//! why they're offered; each counts as a change.
//!
//! Before anything is applied the list says what else it takes: a speech
//! model's download and its size, the OS permission a row needs, a ChatGPT
//! sign-in (that row starts unchecked, with Sign in beside it), and what a
//! shorter retention removes (never checked to begin with, owner decision
//! 4). After the save each row shows how it went, a failed one why, the
//! model its download, and the footer "4 of 5 changes applied" with Undo
//! while it holds (owner decision 2).

use rune::{El, Icon, col, el, icon, row, text};
use rune_kit::Progress;
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint};
use super::card::{self, Look, push};
use super::changes::{self as act, At};
use crate::assistant::changes::catalog::{self, Class, Permission};
use crate::assistant::changes::names;
use crate::assistant::changes::state::{self, Row, Shown};
use crate::assistant::changes::{Card, CardKind, Item, vocab};
use crate::assistant::setup;
use crate::desktop::i18n::{tr, tr_fill, tr_named, tr_window};
use crate::transcription::models;

/// Whether the open conversation ends in a plan still to decide: the
/// field asks about it.
pub(super) fn waiting(ui: &Ui) -> bool {
    let Some(conversation) = ui.assistant.conversation() else {
        return false;
    };
    let Some(index) = conversation.turns.len().checked_sub(1) else {
        return false;
    };
    conversation.turns[index]
        .cards
        .iter()
        .enumerate()
        .any(|(number, card)| {
            card.kind == CardKind::Plan
                && act::shown(
                    ui,
                    At {
                        conversation: conversation.id,
                        turn: index,
                        card: number,
                    },
                )
                .is_some_and(|shown| matches!(shown, Shown::Proposed { .. }))
        })
}

/// The plan at `at`; `primary` when its Apply is the screen's one.
pub(super) fn view(ui: &Ui, p: Paint, at: At, primary: bool) -> Option<El> {
    let card = act::card(ui, at)?;
    let shown = act::shown(ui, at)?;
    let live = act::live(ui, at);
    let c = &p.k.c;
    let selected = card.items.iter().filter(|item| item.included).count();
    let count = match card.items.len() {
        1 => tr_fill("1 change, {} selected", &selected.to_string()),
        all => tr_named(
            "{all} changes, {selected} selected",
            &[
                ("all", &all.to_string()),
                ("selected", &selected.to_string()),
            ],
        ),
    };
    let head = row()
        .key("head")
        .px(14.0)
        .py(11.0)
        .gap(8.0)
        .center_y()
        .child(
            text(tr("Suggested changes"))
                .key("title")
                .font(rune::TextStyle {
                    weight: 600.0,
                    ..p.k.ty.body_medium
                })
                .color(p.text()),
        )
        .child(text(count).key("count").font(p.k.ty.small).color(p.faint()));
    let mut list = col()
        .key(format!("plan{}-{}", at.turn, at.card))
        .role("group")
        .label(tr("Suggested changes"))
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .border(p.hair, c.border_base)
        .clip_soft(0.0)
        .child(head);
    let included: Vec<&str> = card
        .items
        .iter()
        .filter(|item| item.included)
        .map(|item| item.key.as_str())
        .collect();
    let mut area = None;
    let mut why: Option<&str> = None;
    for (index, item) in card.items.iter().enumerate() {
        let shown = state::row(card, index, &live);
        let here = setup::plan::area(&item.key);
        list = list.child(rule(p, &format!("rule{index}")));
        if area != Some(here) {
            area = Some(here);
            list = list.child(heading(p, here));
        }
        if item.key != vocab::KEY {
            list = list.child(line(ui, p, at, index, item, &shown, &included));
            continue;
        }
        // A term as the Vocabulary card has it, under why it's offered.
        let reason = item.why.trim();
        if why != Some(reason) && !reason.is_empty() {
            list = list.child(caption(p, index, reason));
        }
        why = Some(reason);
        list = list.child(
            row()
                .key(format!("row{index}"))
                .px(4.0)
                .child(super::vocab::line(ui, p, at, index, item, &shown).grow(1.0)),
        );
    }
    if !card.kept.is_empty() {
        list = list
            .child(rule(p, "rule-kept"))
            .child(kept(ui, p, at, card));
    }
    if card.items.is_empty() {
        return Some(list);
    }
    Some(
        list.child(rule(p, "rule-foot"))
            .child(footer(ui, p, at, card, &shown, primary)),
    )
}

/// An area's heading over its rows: Dictation, Code, Vocabulary,
/// Meetings, Files, Models.
fn heading(p: Paint, area: setup::plan::Area) -> El {
    row()
        .key(format!("area-{}", area.name().to_lowercase()))
        .px(14.0)
        .pt(9.0)
        .child(
            text(tr(area.name()))
                .key("heading")
                .font(rune::TextStyle {
                    weight: 600.0,
                    ..p.k.ty.small
                })
                .color(p.faint()),
        )
}

/// Why the terms under it are offered.
fn caption(p: Paint, index: usize, why: &str) -> El {
    row().key(format!("caption{index}")).px(14.0).pt(7.0).child(
        text(why.to_owned())
            .key("why")
            .font(p.k.ty.small)
            .color(p.muted())
            .wrap(),
    )
}

/// "Already set: 3 things ›", and what they are once unfolded.
fn kept(ui: &Ui, p: Paint, at: At, card: &Card) -> El {
    let open = ui.assistant.kept_open.contains(&at);
    let count = card.kept.len();
    let words = match count {
        1 => tr("Already set: 1 thing").to_owned(),
        count => tr_fill("Already set: {} things", &count.to_string()),
    };
    let c = &p.k.c;
    let toggle = row()
        .key("kept-toggle")
        .role("button")
        .label(words.clone())
        .checked(open)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .gap(6.0)
        .px(6.0)
        .h(24.0)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .child(text(words).key("words").font(p.k.ty.small).color(p.muted()))
        .child(
            icon(
                if open {
                    &theme::CHEVRON_DOWN
                } else {
                    &theme::CHEVRON_RIGHT
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_faint),
        )
        .on_press(move |ui: &mut Ui| {
            let kept = &mut ui.assistant.kept_open;
            match kept.iter().position(|known| *known == at) {
                Some(index) => {
                    kept.remove(index);
                }
                None => kept.push(at),
            }
        });
    let mut fold = col()
        .key("kept")
        .px(8.0)
        .py(6.0)
        .gap(2.0)
        .child(row().key("head").child(toggle));
    if open {
        for (index, item) in card.kept.iter().enumerate() {
            let (name, path) = match names::place(&item.key, &ui.settings) {
                Some(place) => (
                    tr_window(&place.name).to_owned(),
                    tr_window(place.path).to_owned(),
                ),
                None => (item.key.clone(), String::new()),
            };
            fold = fold.child(
                row()
                    .key(format!("kept{index}"))
                    .px(6.0)
                    .py(3.0)
                    .gap(8.0)
                    .center_y()
                    .child(icon(&theme::CHECK, 12.0).key("glyph").color(p.success()))
                    .child(
                        row()
                            .key("names")
                            .gap(8.0)
                            .grow(1.0)
                            .shrink()
                            .min_w(0.0)
                            .flex_wrap()
                            .child(text(name).key("name").font(p.k.ty.small).color(p.text()))
                            .child(text(path).key("path").font(p.k.ty.small).color(p.faint())),
                    )
                    .child(card::value_chip(
                        p,
                        "now",
                        &card::label(&item.key, &item.from),
                        false,
                        false,
                    )),
            );
        }
    }
    fold
}

/// A hairline between rows.
fn rule(p: Paint, key: &str) -> El {
    el().key(key.to_owned())
        .h(p.hair)
        .w_full()
        .fill(p.k.c.border_weak)
}

/// One row: its checkbox (or how it went), where and what it is, why, what
/// else it takes, and from → to.
fn line(
    ui: &Ui,
    p: Paint,
    at: At,
    index: usize,
    item: &Item,
    shown: &Row,
    included: &[&str],
) -> El {
    let place = names::place(&item.key, &ui.settings);
    let (path, name) = match &place {
        Some(place) => (
            tr_window(place.path).to_owned(),
            tr_window(&place.name).to_owned(),
        ),
        None => (String::new(), item.key.clone()),
    };
    let lead = match shown {
        Row::Open { checked, .. } => super::tick::tick(p, *checked, &name, move |ui: &mut Ui| {
            act::toggle(ui, at, index)
        }),
        Row::Applied => mark(&theme::CHECK, p.success()),
        Row::Failed(_) => mark(&theme::CIRCLE_EXCLAMATION, p.danger()),
        Row::Left => mark(&theme::DASH, p.k.c.icon_faint),
        Row::Downloading { .. } => mark(&theme::DOWNLOAD, p.accent()),
    };
    let off = matches!(shown, Row::Left);
    let mut words = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(
            row()
                .key("names")
                .gap(8.0)
                .center_y()
                .flex_wrap()
                .child(
                    text(name.clone())
                        .key("name")
                        .font(rune::TextStyle {
                            weight: 520.0,
                            ..p.k.ty.ui
                        })
                        .color(if off { p.muted() } else { p.text() }),
                )
                .children(
                    (!path.is_empty())
                        .then(|| text(path).key("path").font(p.k.ty.small).color(p.faint())),
                ),
        );
    if !item.why.trim().is_empty() {
        words = words.child(
            text(item.why.trim().to_owned())
                .key("why")
                .font(DESCRIPTION)
                .color(p.muted())
                .wrap(),
        );
    }
    words = words.children(notes(p, item, shown));
    words = words.children(takes(ui, p, item, shown, included));
    row()
        .key(format!("row{index}"))
        .role("group")
        .label(name)
        .px(14.0)
        .py(9.0)
        .gap(10.0)
        .items_start()
        .child(row().key("lead").pt(2.0).child(lead))
        .child(words)
        .child(
            row()
                .key("change")
                .pt(1.0)
                .child(card::change_chips(p, &item.key, &item.from, &item.to, off)),
        )
}

/// `words` in Assistant's language with the places it names in the
/// window's words, which link to them: "Output › AI cleanup".
fn window_places(words: &str) -> String {
    const PLACES: [[&str; 2]; 1] = [["Output", "AI cleanup"]];
    PLACES.iter().fold(words.to_owned(), |words, [pane, row]| {
        let spoken = format!("{} › {}", tr(pane), tr(row));
        let shown = format!("{} › {}", tr_window(pane), tr_window(row));
        words.replace(&spoken, &shown)
    })
}

/// A setup row's notes while it is open: "You chose" its value now, what
/// to finish before it can run, and what the Labs feature takes after the
/// switch.
fn notes(p: Paint, item: &Item, shown: &Row) -> Vec<El> {
    if !matches!(shown, Row::Open { .. }) {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let line = |key: &str, words: String, color: rune::Color| {
        text(words)
            .key(key.to_owned())
            .font(DESCRIPTION)
            .color(color)
            .wrap()
    };
    if item.yours {
        lines.push(line(
            "yours",
            tr_fill("You chose {}", &card::label(&item.key, &item.from)),
            p.muted(),
        ));
    }
    if !item.note.is_empty() {
        lines.push(line("note", window_places(tr(&item.note)), p.warning()));
    }
    let then = crate::assistant::guide::articles()
        .iter()
        .find(|article| article.is_lab() && article.keys.first() == Some(&item.key))
        .map(|article| article.then.as_str())
        .filter(|then| !then.is_empty());
    if let Some(then) = then {
        lines.push(line("then", tr_fill("Then: {}", tr(then)), p.faint()));
    }
    lines
}

/// A row's mark once the plan is decided.
fn mark(glyph: &'static Icon, color: rune::Color) -> El {
    el().key("mark")
        .size(16.0, 16.0)
        .center()
        .child(icon(glyph, 13.0).key("glyph").color(color))
}

/// What else a row takes or how it went: a download and its size, a
/// sign-in, what a shorter retention removes, what it turns on beside, a
/// failure, a download's progress.
fn takes(ui: &Ui, p: Paint, item: &Item, shown: &Row, included: &[&str]) -> Vec<El> {
    let mut lines = Vec::new();
    match shown {
        Row::Open {
            sign_in, shorter, ..
        } => {
            if let Some(size) = download_size(item) {
                lines.push(
                    row()
                        .key("download")
                        .gap(6.0)
                        .center_y()
                        .child(
                            text(tr("Needs a download"))
                                .key("words")
                                .font(DESCRIPTION)
                                .color(p.muted()),
                        )
                        .child(text(size).key("size").font(DESCRIPTION).color(p.warning())),
                );
            }
            if *sign_in {
                lines.push(
                    row()
                        .key("sign-in")
                        .gap(8.0)
                        .center_y()
                        .mt(2.0)
                        .child(
                            text(tr("Needs a ChatGPT sign-in"))
                                .key("words")
                                .font(DESCRIPTION)
                                .color(p.muted()),
                        )
                        .child(push(
                            p,
                            "sign-in",
                            tr("Sign in with ChatGPT"),
                            Look::Secondary,
                            |ui, _| act::sign_in(ui),
                        )),
                );
            }
            if let Some(shorter) = shorter {
                lines.push(card::warned(p, card::warning_words(*shorter)));
            }
            for (key, _) in item
                .requires
                .iter()
                .filter(|(key, value)| {
                    !included.contains(&key.as_str())
                        && crate::assistant::changes::current(&ui.settings, key).as_ref()
                            != Some(value)
                })
                .take(1)
            {
                let name = match names::place(key, &ui.settings) {
                    Some(place) => {
                        format!("{} › {}", tr_window(place.path), tr_window(&place.name))
                    }
                    None => key.clone(),
                };
                lines.push(
                    text(format!("{} {name}", tr("Also turns on")))
                        .key("also")
                        .font(DESCRIPTION)
                        .color(p.faint())
                        .wrap(),
                );
            }
        }
        Row::Failed(why) => lines.push(
            text(tr_fill("Not changed: {}", why))
                .key("failed")
                .font(DESCRIPTION)
                .color(p.danger())
                .wrap(),
        ),
        Row::Downloading { done, total } => {
            let fraction = if *total > 0 {
                (*done as f32 / *total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let megabytes = |bytes: u64| (bytes / 1_000_000).to_string();
            let words = if *total > 0 {
                tr_named(
                    "Downloading, {done} of {total} MB. Switches when done",
                    &[("done", &megabytes(*done)), ("total", &megabytes(*total))],
                )
            } else {
                tr_fill("Downloading, {} MB. Switches when done", &megabytes(*done))
            };
            lines.push(
                row()
                    .key("progress")
                    .pt(4.0)
                    .child(Progress::new(Some(fraction)).build(p.k)),
            );
            lines.push(
                text(words)
                    .key("downloading")
                    .font(DESCRIPTION)
                    .color(p.muted())
                    .wrap(),
            );
        }
        Row::Applied | Row::Left => {}
    }
    lines
}

/// A speech model's download, as the picker says it: `274 MB`.
fn download_size(item: &Item) -> Option<String> {
    if item.class != Class::Download || item.key != "transcription.model" {
        return None;
    }
    let id: models::TranscriptionModelId = serde_json::from_value(item.to.clone()).ok()?;
    let bytes = models::definition(id).download_bytes()?;
    Some(format!("{} MB", bytes / 1_000_000))
}

/// The OS permissions the checked rows need, named before Apply.
fn permissions(ui: &Ui, card: &Card, system_audio: bool) -> Option<&'static str> {
    let mac = super::ask::os(ui) == "macos";
    card.items
        .iter()
        .filter(|item| item.included)
        .find_map(|item| match catalog::permission(&item.key, &item.to)? {
            Permission::SystemAudio if !system_audio => Some(tr(
                "macOS will ask for System Audio Recording at the first call",
            )),
            Permission::ScreenRecording if mac => {
                Some(tr("macOS will ask for Screen Recording the first time"))
            }
            _ => None,
        })
}

/// Not now and Apply, or how the plan went and Undo.
fn footer(ui: &Ui, p: Paint, at: At, card: &Card, shown: &Shown, primary: bool) -> El {
    let system_audio = act::live(ui, at).system_audio;
    let mut foot = row()
        .key("foot")
        .px(14.0)
        .py(10.0)
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .fill(p.k.alpha(p.k.c.layer_02, 0.5));
    let status = |words: String, color: rune::Color| {
        text(words)
            .key("status")
            .font(p.k.ty.small)
            .color(color)
            .wrap()
            .shrink()
    };
    let space = || el().key("space").grow(1.0);
    match shown {
        Shown::Proposed { .. } => {
            let selected = card.items.iter().filter(|item| item.included).count();
            let label = match selected {
                1 => tr("Apply 1 change").to_owned(),
                count => tr_fill("Apply {} changes", &count.to_string()),
            };
            let main = if primary {
                rune_kit::ButtonVariant::Contrast
            } else {
                rune_kit::ButtonVariant::Neutral
            };
            let apply = rune_kit::Button::new(label)
                .variant(main)
                .disabled(selected == 0)
                .on_press(move |ui: &mut Ui| act::apply(ui, at));
            foot = foot
                .children(
                    permissions(ui, card, system_audio)
                        .map(|words| status(words.to_owned(), p.faint())),
                )
                .child(space())
                .child(push(
                    p,
                    "not-now",
                    tr("Not now"),
                    Look::Secondary,
                    move |ui, _| act::decline(ui, at),
                ))
                .child(super::super::controls::PushSize::build_small_push(apply, p.k).key("apply"));
        }
        Shown::Applied { .. } | Shown::ChangedSince { .. } => {
            let (done, total) = state::applied_count(card);
            let words = tr_named(
                "{done} of {total} changes applied",
                &[("done", &done.to_string()), ("total", &total.to_string())],
            );
            let color = if done == total {
                p.success()
            } else {
                p.muted()
            };
            foot = foot.child(status(words, color)).child(space());
            if *shown == (Shown::Applied { undo: true }) {
                foot = foot.child(push(p, "undo", tr("Undo"), Look::Ghost, move |ui, _| {
                    act::undo(ui, at)
                }));
            }
        }
        Shown::Failed(why) => {
            foot = foot
                .child(status(tr_fill("Not changed: {}", why), p.danger()))
                .child(space())
                .child(push(
                    p,
                    "retry",
                    tr("Try again"),
                    Look::Secondary,
                    move |ui, _| act::apply(ui, at),
                ));
        }
        _ => {
            let words = if *shown == Shown::Undone {
                tr("Put back as it was")
            } else {
                tr("Left as it was")
            };
            foot = foot
                .child(status(words.to_owned(), p.muted()))
                .child(space())
                .child(push(
                    p,
                    "anyway",
                    tr("Change anyway"),
                    Look::Ghost,
                    move |ui, _| act::apply(ui, at),
                ));
        }
    }
    foot
}
