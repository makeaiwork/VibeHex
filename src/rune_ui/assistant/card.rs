//! A setting-change card under an answer: where the setting is, its name,
//! the value now and the one suggested, the model's reason, and the
//! buttons. What it shows comes from `changes::state` each frame, so a
//! value changed by hand meanwhile, a download, or a sign-in shows at
//! once. Only the newest card still waiting gets the Primary button (owner
//! decision 3); a shorter retention's button is Danger, since older records
//! go. Highlights are soft tints and marks, never a coloured side bar.

use rune::{El, Icon, Tx, col, el, icon, row, text};
use rune_kit::{Button, ButtonVariant, RadioItem};
use rune_theme_opencode::icons as theme;
use serde_json::Value;

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint, PushSize};
use super::super::icons;
use super::super::lists::{Tone, tone_tag};
use super::card_diff;
use super::changes::{self as act, At};
use crate::assistant::changes::catalog::{Permission, Shorter};
use crate::assistant::changes::names::{self, Home};
use crate::assistant::changes::parts::PROMPT;
use crate::assistant::changes::state::Shown;
use crate::assistant::changes::{Card, CardKind, Decision, Item, Refused, Why};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// Where the words start: past the 28 pt tile and its gap.
const INDENT: f32 = 38.0;

/// How a card's shell is tinted.
#[derive(Clone, Copy, PartialEq)]
enum Shell {
    /// Waiting on the person.
    Open,
    /// Saved: a soft success tint.
    Done,
    /// Nothing to do: muted.
    Off,
}

/// How a card's button looks.
#[derive(Clone, Copy)]
pub(super) enum Look {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

pub(super) fn push(
    p: Paint,
    key: &str,
    label: &str,
    look: Look,
    press: impl Fn(&mut Ui, &mut Tx) + 'static,
) -> El {
    let variant = match look {
        Look::Primary => ButtonVariant::Contrast,
        Look::Secondary => ButtonVariant::Neutral,
        Look::Ghost => ButtonVariant::Ghost,
        Look::Danger => ButtonVariant::Danger,
    };
    Button::new(label)
        .variant(variant)
        .on_press_tx(press)
        .build_small_push(p.k)
        .key(key)
}

/// The card at `at`; `primary` when its main button is the screen's one.
pub(super) fn view(ui: &Ui, p: Paint, at: At, primary: bool) -> Option<El> {
    let card = act::card(ui, at)?;
    let item = card.items.first()?;
    let shown = act::shown(ui, at)?;
    let main = if primary {
        Look::Primary
    } else {
        Look::Secondary
    };
    let place = names::place(&item.key, &ui.settings);
    let (path, name) = match &place {
        Some(place) => (tr(place.path).to_owned(), tr(&place.name).to_owned()),
        None => (String::new(), item.key.clone()),
    };
    let to = chosen(card).clone();
    let home = place.as_ref().map(|place| place.home);
    let key = format!("card{}-{}", at.turn, at.card);
    let mut body: Vec<El> = Vec::new();
    let (shell, glyph, head) = match &shown {
        Shown::Permission(permission) => {
            let (path, name) = permission_names(*permission);
            let chip = value_chip(p, "now", tr("Not asked"), false, false);
            body.extend(why(
                p,
                tr("Without it only your microphone is recorded. macOS asks once"),
            ));
            body.push(buttons(vec![
                push(p, "ask", tr("Ask macOS"), main, |ui, _| act::ask_macos(ui)),
                push(p, "later", tr("Later"), Look::Secondary, move |ui, _| {
                    act::later(ui, at)
                }),
            ]));
            (Shell::Open, &theme::LOCK, header(p, path, name, Some(chip)))
        }
        Shown::SignIn => {
            let chip = value_chip(p, "now", tr("Signed out"), false, false);
            body.extend(why(
                p,
                tr("You sign in on ChatGPT's page. VibeHex never sees your password"),
            ));
            body.push(buttons(vec![
                push(p, "sign-in", tr("Sign in with ChatGPT"), main, |ui, _| {
                    act::sign_in(ui)
                }),
                push(p, "output", tr("Open Output"), Look::Ghost, |ui, tx| {
                    act::open_output(ui, tx)
                }),
            ]));
            (
                Shell::Open,
                &theme::CODE_LINES,
                header(
                    p,
                    tr("Output › AI cleanup"),
                    tr("Codex sign-in"),
                    Some(chip),
                ),
            )
        }
        _ => {
            let (shell, glyph) = look_of(&shown, card, item, home);
            let struck = matches!(shown, Shown::Declined | Shown::Undone);
            let chips = match (&card.kind, &shown) {
                (CardKind::Choice { .. }, Shown::Proposed { .. }) => None,
                (CardKind::Prompt, _) => {
                    let lines = card_diff::changed(text_of(&item.from), text_of(&to));
                    let label = match lines {
                        1 => tr("1 line changed").to_owned(),
                        lines => tr_fill("{} lines changed", &lines.to_string()),
                    };
                    Some(value_chip(p, "lines", &label, false, false))
                }
                (_, Shown::Blocked { .. }) => Some(value_chip(
                    p,
                    "now",
                    &label(&item.key, &item.from),
                    false,
                    false,
                )),
                _ => Some(change_chips(p, &item.key, &item.from, &to, struck)),
            };
            body = state_body(ui, p, at, card, item, &to, &shown, main);
            let ink = if shell == Shell::Off {
                p.muted()
            } else {
                p.text()
            };
            (shell, glyph, header_in(p, &path, &name, chips, ink))
        }
    };
    let tile = tile(p, glyph, shell);
    let top = row()
        .key("top")
        .gap(10.0)
        .center_y()
        .child(tile)
        .child(head);
    Some(shell_of(p, &key, shell, &name).child(top).children(body))
}

/// The card's tile, shell, and words for every state but a permission and
/// a sign-in, which show what they ask for instead.
#[allow(clippy::too_many_arguments)]
fn state_body(
    ui: &Ui,
    p: Paint,
    at: At,
    card: &Card,
    item: &Item,
    to: &Value,
    shown: &Shown,
    main: Look,
) -> Vec<El> {
    let mut body = Vec::new();
    let key = item.key.clone();
    match shown {
        Shown::Proposed { shorter, also } => {
            if let CardKind::Choice {
                options,
                chosen,
                suggested,
            } = &card.kind
            {
                body.push(radios(p, at, item, options, *chosen, *suggested));
            }
            if card.kind == CardKind::Prompt {
                body.push(card_diff::view(p, text_of(&item.from), text_of(to)));
            }
            body.extend(why(p, &item.why));
            for (index, (key, value)) in also.iter().enumerate() {
                body.push(also_row(ui, p, index, key, value));
            }
            if let Some(shorter) = shorter {
                body.push(warning(p, *shorter));
            }
            let look = if shorter.is_some() {
                Look::Danger
            } else {
                main
            };
            body.push(buttons(vec![
                push(
                    p,
                    "apply",
                    &verb(card, &item.key, to, also.len()),
                    look,
                    move |ui, _| act::apply(ui, at),
                ),
                push(
                    p,
                    "not-now",
                    tr("Not now"),
                    Look::Secondary,
                    move |ui, _| act::decline(ui, at),
                ),
            ]));
        }
        Shown::Applied { undo } => {
            let mut row = vec![
                status(&applied_words(card, &item.key, to), p.success()),
                el().key("space").grow(1.0),
                push(
                    p,
                    "show",
                    show_label(&item.key, ui),
                    Look::Ghost,
                    move |ui, tx| act::show(ui, &key, tx),
                ),
            ];
            if *undo {
                row.push(push(p, "undo", tr("Undo"), Look::Ghost, move |ui, _| {
                    act::undo(ui, at)
                }));
            }
            body.push(buttons(row));
        }
        Shown::Declined | Shown::Undone => {
            let words = if *shown == Shown::Declined {
                tr("Left as it was")
            } else {
                tr("Put back as it was")
            };
            body.push(buttons(vec![
                status(words, p.muted()),
                el().key("space").grow(1.0),
                push(p, "anyway", anyway(card, to), Look::Ghost, move |ui, _| {
                    act::apply(ui, at)
                }),
            ]));
        }
        Shown::Failed(why) => {
            body.push(
                text(tr_fill("Not changed: {}", why))
                    .key("failed")
                    .font(DESCRIPTION)
                    .color(p.danger())
                    .wrap()
                    .pl(INDENT),
            );
            body.push(buttons(vec![push(
                p,
                "retry",
                tr("Try again"),
                Look::Secondary,
                move |ui, _| act::apply(ui, at),
            )]));
        }
        Shown::ChangedSince { now } => {
            let now = label(&item.key, now);
            let words = if matches!(card.decision, Decision::Applied { .. }) {
                tr_fill("Changed since, now {}", &now)
            } else {
                let pane = names::place(&item.key, &ui.settings)
                    .map(|place| place.path.split(" › ").next().unwrap_or(place.path))
                    .unwrap_or_default();
                tr_named(
                    "Now set to {value}, in {pane}. Nothing was changed",
                    &[("value", &now), ("pane", tr(pane))],
                )
            };
            body.extend(why(p, &words));
            body.push(buttons(vec![push(
                p,
                "ask-again",
                tr("Ask again"),
                Look::Secondary,
                move |ui, _| act::ask_again(ui, at),
            )]));
        }
        Shown::Already => {
            body.push(buttons(vec![status(tr("Already set this way"), p.muted())]));
        }
        Shown::Downloading { done, total } => {
            let fraction = if *total > 0 {
                (*done as f32 / *total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            body.push(
                row()
                    .key("progress")
                    .pl(INDENT)
                    .child(rune_kit::Progress::new(Some(fraction)).build(p.k)),
            );
            let megabytes = |bytes: u64| (bytes / 1_000_000).to_string();
            let words = if *total > 0 {
                tr_named(
                    "Downloading, {done} of {total} MB. Switches when done",
                    &[("done", &megabytes(*done)), ("total", &megabytes(*total))],
                )
            } else {
                tr_fill("Downloading, {} MB. Switches when done", &megabytes(*done))
            };
            body.push(buttons(vec![
                status(&words, p.muted()),
                el().key("space").grow(1.0),
                push(p, "cancel", tr("Cancel"), Look::Ghost, |ui, _| {
                    act::cancel_download(ui)
                }),
            ]));
        }
        Shown::Blocked { managed, why } => {
            let words = if *managed {
                tr("Your organization manages this setting").to_owned()
            } else {
                why.clone()
            };
            body.push(buttons(vec![
                icon(&theme::LOCK, 12.0).key("lock").color(p.k.c.icon_muted),
                status(&words, p.muted()),
            ]));
        }
        Shown::Permission(_) | Shown::SignIn => {}
    }
    body
}

/// The tint and glyph a state wears.
fn look_of(shown: &Shown, card: &Card, item: &Item, home: Option<Home>) -> (Shell, &'static Icon) {
    match shown {
        Shown::Applied { .. } => (Shell::Done, &theme::CHECK),
        Shown::ChangedSince { .. } => (Shell::Off, &theme::REFRESH),
        Shown::Blocked { .. } => (Shell::Off, &theme::LOCK),
        Shown::Declined | Shown::Undone | Shown::Already | Shown::Failed(_) => {
            (Shell::Off, glyph(card, item, home))
        }
        _ => (Shell::Open, glyph(card, item, home)),
    }
}

/// What a card is about: a download, the instructions, or its pane's mark.
fn glyph(card: &Card, item: &Item, home: Option<Home>) -> &'static Icon {
    if item.class == crate::assistant::changes::catalog::Class::Download {
        return &theme::DOWNLOAD;
    }
    if card.kind == CardKind::Prompt || item.key == PROMPT {
        return &theme::PENCIL_LINE;
    }
    match home {
        Some(Home::Labs(_)) | None => &theme::FLASK,
        Some(Home::General) => &theme::SLIDERS,
        Some(Home::Output) => &icons::SPARKLES,
        Some(Home::History) => &icons::CLOCK,
        Some(Home::Meetings) => &icons::CALL,
        Some(Home::Annotate) => &icons::PEN,
        Some(Home::Files) => &icons::FOLDER,
    }
}

fn shell_of(p: Paint, key: &str, shell: Shell, name: &str) -> El {
    let c = &p.k.c;
    let (fill, border) = match shell {
        Shell::Open => (c.layer_01, c.border_weak),
        Shell::Done => (p.k.alpha(p.success(), 0.08), p.k.alpha(p.success(), 0.28)),
        Shell::Off => (p.k.alpha(c.layer_01, 0.5), c.border_weak),
    };
    col()
        .key(key.to_owned())
        .role("group")
        .label(name.to_owned())
        .gap(8.0)
        .px(14.0)
        .py(12.0)
        .radius(p.k.radius.md)
        .fill(fill)
        .border(p.hair, border)
}

fn tile(p: Paint, glyph: &'static Icon, shell: Shell) -> El {
    let c = &p.k.c;
    let (fill, ink) = match shell {
        Shell::Open => (c.layer_02, c.icon_base),
        Shell::Done => (p.k.alpha(p.success(), 0.16), p.success()),
        Shell::Off => (c.layer_02, c.icon_muted),
    };
    el().key("tile")
        .size(28.0, 28.0)
        .center()
        .radius(p.k.radius.sm)
        .fill(fill)
        .child(icon(glyph, 14.0).key("glyph").color(ink))
}

/// The path over the name, and the values at the right.
fn header(p: Paint, path: &str, name: &str, chips: Option<El>) -> El {
    header_in(p, path, name, chips, p.text())
}

/// The header with its name in `ink`: muted on a card with nothing to do.
fn header_in(p: Paint, path: &str, name: &str, chips: Option<El>, ink: rune::Color) -> El {
    let mut words = col().key("names").grow(1.0).shrink().min_w(0.0).gap(1.0);
    if !path.is_empty() {
        words = words.child(
            text(path.to_owned())
                .key("path")
                .font(p.k.ty.small)
                .color(p.faint())
                .max_lines(1),
        );
    }
    words = words.child(
        text(name.to_owned())
            .key("name")
            .font(p.k.ty.body_medium)
            .color(ink)
            .wrap(),
    );
    row()
        .key("head")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(10.0)
        .center_y()
        .child(words)
        .children(chips)
}

/// From → to.
pub(super) fn change_chips(p: Paint, key: &str, from: &Value, to: &Value, struck: bool) -> El {
    row()
        .key("values")
        .gap(6.0)
        .center_y()
        .shrink()
        .child(value_chip(p, "from", &label(key, from), false, false))
        .child(text("→").key("arrow").font(p.k.ty.small).color(p.faint()))
        .child(value_chip(p, "to", &label(key, to), !struck, struck))
}

/// A value: the one now outlined, the new one on a soft accent tint, or
/// struck through when it was turned down.
fn value_chip(p: Paint, key: &str, label: &str, new: bool, struck: bool) -> El {
    let c = &p.k.c;
    let (fill, border, ink) = if new {
        (
            p.k.alpha(p.accent(), 0.12),
            p.k.alpha(p.accent(), 0.4),
            p.accent(),
        )
    } else if struck {
        (c.layer_02, c.border_weak, p.faint())
    } else {
        (c.layer_02, c.border_weak, p.muted())
    };
    let mut words = text(label.to_owned())
        .key("label")
        .font(p.k.ty.small)
        .color(ink)
        .max_lines(1)
        .shrink();
    if struck {
        words = words.strike(0, label.len(), None);
    }
    row()
        .key(key.to_owned())
        .h(20.0)
        .px(7.0)
        .center_y()
        .shrink()
        .min_w(0.0)
        .max_w(220.0)
        .radius(p.k.radius.xs)
        .fill(fill)
        .border(p.hair, border)
        .child(words)
}

/// The model's reason, under the names.
fn why(p: Paint, words: &str) -> Option<El> {
    let words = words.trim();
    (!words.is_empty()).then(|| {
        text(words.to_owned())
            .key("why")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap()
            .pl(INDENT)
    })
}

fn status(words: &str, color: rune::Color) -> El {
    text(words.to_owned())
        .key("status")
        .font(DESCRIPTION)
        .color(color)
        .wrap()
        .shrink()
}

fn buttons(children: Vec<El>) -> El {
    row()
        .key("buttons")
        .pl(INDENT)
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .children(children)
}

/// A choice's options: the chosen on a soft accent tint, the suggested
/// one tagged, the current one named.
fn radios(p: Paint, at: At, item: &Item, options: &[Value], chosen: usize, suggested: usize) -> El {
    let c = &p.k.c;
    let mut list = col().key("options").role("radiogroup").pl(INDENT).gap(2.0);
    for (index, option) in options.iter().enumerate() {
        let on = index == chosen;
        let radio = RadioItem::new(label(&item.key, option), on);
        let mut line = row()
            .key(format!("option{index}"))
            .px(8.0)
            .py(6.0)
            .gap(8.0)
            .items_start()
            .radius(p.k.radius.sm)
            .child(
                radio
                    .on_select(move |ui: &mut Ui| act::pick(ui, at, index))
                    .build(p.k),
            )
            .children(
                (index == suggested)
                    .then(|| tone_tag(p, "suggested", tr("Suggested"), Tone::Accent).mt(1.0)),
            )
            .child(el().key("space").grow(1.0))
            .children((*option == item.from).then(|| {
                text(tr("current"))
                    .key("current")
                    .font(p.k.ty.small)
                    .color(p.faint())
                    .mt(2.0)
            }));
        if on {
            line = line
                .fill(p.k.alpha(p.accent(), 0.08))
                .border(p.hair, p.k.alpha(p.accent(), 0.32));
        } else {
            line = line.hover(c.hover);
        }
        list = list.child(line);
    }
    list
}

/// What the save turns on beside: "Also turns on Labs › …".
fn also_row(ui: &Ui, p: Paint, index: usize, key: &str, value: &Value) -> El {
    let name = match names::place(key, &ui.settings) {
        Some(place) if place.path.is_empty() => tr(&place.name).to_owned(),
        Some(place) => format!("{} › {}", tr(place.path), tr(&place.name)),
        None => key.to_owned(),
    };
    let mut line = row()
        .key(format!("also{index}"))
        .pl(INDENT)
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .child(
            icon(&theme::PLUS, 12.0)
                .key("glyph")
                .color(p.k.c.icon_muted),
        );
    let lead = if *value == Value::Bool(true) {
        tr("Also turns on")
    } else {
        tr("Also sets")
    };
    line = line
        .child(text(lead).key("lead").font(DESCRIPTION).color(p.muted()))
        .child(
            text(name)
                .key("name")
                .font(rune::TextStyle {
                    weight: 520.0,
                    ..DESCRIPTION
                })
                .color(p.text()),
        );
    if *value != Value::Bool(true) {
        line = line.child(
            text(tr_fill("to {}", &label(key, value)))
                .key("to")
                .font(DESCRIPTION)
                .color(p.muted()),
        );
    }
    line
}

/// What a shorter retention removes, said plainly (owner decision 4).
fn warning(p: Paint, shorter: Shorter) -> El {
    warned(p, warning_words(shorter)).pl(INDENT)
}

/// A shorter retention's warning: an icon and the words, in the warning
/// tone.
pub(super) fn warned(p: Paint, words: &str) -> El {
    row()
        .key("warning")
        .gap(6.0)
        .items_start()
        .child(
            icon(&theme::WARNING, 12.0)
                .key("glyph")
                .color(p.warning())
                .mt(2.0),
        )
        .child(
            text(words.to_owned())
                .key("words")
                .font(DESCRIPTION)
                .color(p.warning())
                .wrap()
                .shrink(),
        )
}

/// What a shorter retention removes.
pub(super) fn warning_words(shorter: Shorter) -> &'static str {
    match shorter {
        Shorter::Meetings => tr("Meetings older than this will move to the Trash"),
        Shorter::MeetingAudio => {
            tr("Older meetings will lose their audio. Notes and transcripts stay")
        }
        Shorter::History => tr("Older dictations will be deleted from History"),
        Shorter::CaptureMedia => tr("Older captures will lose their video and audio"),
        Shorter::Captures => tr("Older captures will be deleted"),
    }
}

/// A refused line, never an action: muted, with what was wrong.
pub(super) fn refused(ui: &Ui, p: Paint, index: usize, refused: &Refused) -> Option<El> {
    let key = refused.key.as_deref()?;
    let place = names::place(key, &ui.settings);
    let version = if cfg!(test) { "0.0.0" } else { crate::VERSION };
    let (glyph, path, name, words) = match &refused.why {
        Why::Malformed => return None,
        Why::Unknown => (
            &theme::CIRCLE_EXCLAMATION,
            String::new(),
            tr_fill("No such setting in VibeHex {}", version),
            key.to_owned(),
        ),
        Why::Never => {
            let place = place?;
            (
                &theme::LOCK,
                tr(place.path).to_owned(),
                tr(&place.name).to_owned(),
                tr_fill("Changed only by hand, in {}", tr(place.path)),
            )
        }
        Why::Invalid(why) => {
            let (path, name) = place.map_or((String::new(), key.to_owned()), |place| {
                (tr(place.path).to_owned(), tr(&place.name).to_owned())
            });
            (
                &theme::CIRCLE_EXCLAMATION,
                path,
                name,
                tr_fill("Not suggested: {}", why),
            )
        }
    };
    let top = row()
        .key("top")
        .gap(10.0)
        .center_y()
        .child(tile(p, glyph, Shell::Off))
        .child(header_in(p, &path, &name, None, p.muted()));
    Some(
        shell_of(p, &format!("refused{index}"), Shell::Off, &name)
            .child(top)
            .child(
                text(words)
                    .key("why")
                    .font(DESCRIPTION)
                    .color(p.faint())
                    .wrap()
                    .pl(INDENT),
            ),
    )
}

fn permission_names(permission: Permission) -> (&'static str, &'static str) {
    match permission {
        Permission::SystemAudio => (tr("Meetings › Permissions"), tr("System Audio Recording")),
        Permission::ScreenRecording => (tr("Annotate › Permissions"), tr("Screen Recording")),
    }
}

/// The value the card's first item goes to: a choice's chosen option.
fn chosen(card: &Card) -> &Value {
    static NULL: Value = Value::Null;
    let Some(item) = card.items.first() else {
        return &NULL;
    };
    match &card.kind {
        CardKind::Choice {
            options, chosen, ..
        } => options.get(*chosen).unwrap_or(&item.to),
        _ => &item.to,
    }
}

fn text_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// A value as the window names it, in the window's language.
pub(super) fn label(key: &str, value: &Value) -> String {
    tr(&names::label(key, value)).to_owned()
}

/// The main button: what pressing it does.
fn verb(card: &Card, key: &str, to: &Value, also: usize) -> String {
    match &card.kind {
        CardKind::Prompt => return tr("Save instructions").to_owned(),
        CardKind::Choice { .. } => return tr_fill("Use {}", &label(key, to)),
        _ => {}
    }
    match (also, to) {
        (1, _) => tr("Change both").to_owned(),
        (2.., _) => tr("Change all").to_owned(),
        (_, Value::Bool(true)) => tr("Turn on").to_owned(),
        (_, Value::Bool(false)) => tr("Turn off").to_owned(),
        _ => tr("Change").to_owned(),
    }
}

/// The button on a card turned down: the change after all.
fn anyway(card: &Card, to: &Value) -> &'static str {
    match (&card.kind, to) {
        (CardKind::Single, Value::Bool(true)) => tr("Turn on anyway"),
        (CardKind::Single, Value::Bool(false)) => tr("Turn off anyway"),
        _ => tr("Change anyway"),
    }
}

/// What a saved card says.
fn applied_words(card: &Card, key: &str, to: &Value) -> String {
    if card.kind == CardKind::Prompt {
        return tr("Saved").to_owned();
    }
    match (key, to) {
        ("meetings.enabled", Value::Bool(true)) => {
            tr("Turned on, Meetings is in the sidebar").to_owned()
        }
        (_, Value::Bool(true)) => tr("Turned on").to_owned(),
        (_, Value::Bool(false)) => tr("Turned off").to_owned(),
        _ => tr_fill("Set to {}", &label(key, to)),
    }
}

/// Show in Labs, or in the setting's pane.
fn show_label(key: &str, ui: &Ui) -> &'static str {
    match names::place(key, &ui.settings).map(|place| place.home) {
        Some(Home::Labs(_)) | None => tr("Show in Labs"),
        Some(Home::General) => tr("Show in General"),
        Some(Home::Output) => tr("Show in Output"),
        Some(Home::History) => tr("Show in History"),
        Some(Home::Meetings) => tr("Show in Meetings"),
        Some(Home::Annotate) => tr("Show in Annotate"),
        Some(Home::Files) => tr("Show in Files"),
    }
}
