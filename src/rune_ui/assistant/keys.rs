//! The shortcuts card in the pane (`assistant::changes::shortcuts`): a
//! row per shortcut, named as General names it, with General's recorder.
//! Pressed, a row records the way General does (`settings::begin_capture`:
//! the host pauses VibeHex's global shortcuts until it ends, so the
//! dictation key records rather than dictates), with General's rules and
//! "Already in use" counting the card's other rows as they would be saved.
//! Its rows are places in the window, named in the window's words; the
//! rest speaks Assistant's language, as a plan does.
//! Esc ends it. What a row records stays in the card: nothing is saved
//! until its button, one all-or-none save with Undo.
//!
//! The preset's second step shows Denis's bindings over the person's, from
//! → to where they differ, a row already the same as "Already set", and
//! "Keep mine" and "Use these"; an answer about one's shortcuts shows them
//! as they are, a row recorded anew from → to, and "Save".

use rune::{El, Icon, col, el, icon, row, text};
use rune_theme_opencode::icons as theme;
use serde_json::Value;

use super::super::controls::{DESCRIPTION, Paint, PushSize};
use super::super::general::{keys_tray, quiet_select, recorder};
use super::super::icons::*;
use super::super::settings::{self as s, Capture, Face};
use super::super::shortcut_mode::Trigger;
use super::super::{Menu, ShortcutTarget, Ui};
use super::card::{Look, push};
use super::changes::{self as act, At};
use super::value::{self, Slot};
use crate::app_settings::{HotkeyBinding, HotkeyModifiers};
use crate::assistant::changes::names;
use crate::assistant::changes::shortcuts::{self, MODE};
use crate::assistant::changes::state::Shown;
use crate::assistant::changes::{Card, Item};
use crate::assistant::setup::flow::SHORTCUTS;
use crate::control::settings;
use crate::desktop::i18n::{tr, tr_fill, tr_window};

/// A shortcut's row as General has it: what records it, its title, what
/// it does, and its glyph. `None` for a key this OS has no recorder for.
fn place(
    key: &str,
) -> Option<(
    ShortcutTarget,
    &'static str,
    Option<&'static str>,
    &'static Icon,
)> {
    let dictates = || Some(tr_window("Dictates like the shortcut above"));
    Some(match key {
        "dictation_hotkey" => (
            ShortcutTarget::Dictation,
            tr_window("Shortcut"),
            None,
            &KEYBOARD,
        ),
        "cancel_hotkey" => (
            ShortcutTarget::Cancel,
            tr_window("Cancel"),
            Some(tr_window("Discards the recording")),
            &CIRCLE_X,
        ),
        #[cfg(target_os = "macos")]
        "second_dictation_hotkey" => (
            ShortcutTarget::SecondDictation,
            tr_window("Second dictation shortcut"),
            dictates(),
            &KEYBOARD,
        ),
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        "terminal_paste_hotkey" => (
            ShortcutTarget::TerminalPaste,
            tr_window("Terminal paste shortcut"),
            dictates(),
            &KEYBOARD,
        ),
        #[cfg(target_os = "macos")]
        "stop_hotkey" => (
            ShortcutTarget::Stop,
            tr_window("Stop shortcut"),
            Some(tr_window("Finishes the recording and pastes it")),
            &KEYBOARD,
        ),
        #[cfg(hex_capture)]
        "capture.hotkey" => (
            ShortcutTarget::Capture,
            tr_window("Annotate"),
            None,
            &super::super::sidebar::SCRAWL_ICON,
        ),
        #[cfg(all(hex_capture, target_os = "macos"))]
        "capture.replay_hotkey" => (
            ShortcutTarget::CaptureReplay,
            tr_window("Replay last 15 s"),
            None,
            &super::super::sidebar::SCRAWL_ICON,
        ),
        "paste_last_hotkey" => (
            ShortcutTarget::PasteLast,
            tr_window("Paste last dictation"),
            None,
            &PASTE,
        ),
        #[cfg(target_os = "macos")]
        "vocabulary_add_hotkey" => (
            ShortcutTarget::VocabularyAdd,
            tr_window("Add selection to vocabulary"),
            None,
            &BOOK,
        ),
        "raw_dictation_hotkey" => (
            ShortcutTarget::RawDictation,
            tr_window("Dictate without processing"),
            Some(tr_window("Pastes your words as VibeHex heard them")),
            &MIC,
        ),
        #[cfg(target_os = "linux")]
        "typed_text_hotkey" => (
            ShortcutTarget::TypedText,
            tr_window("Transform typed text"),
            Some(tr_window("Replaces what you typed where it is")),
            &KEYBOARD,
        ),
        #[cfg(target_os = "macos")]
        "screenshots.hotkey" => (
            ShortcutTarget::Screenshot,
            tr_window("Screenshots"),
            None,
            &CAMERA,
        ),
        #[cfg(target_os = "macos")]
        "screenshots.record_hotkey" => (
            ShortcutTarget::ScreenshotRecord,
            tr_window("Record screen"),
            None,
            &CAMERA,
        ),
        _ => {
            let _ = dictates;
            return None;
        }
    })
}

/// Row `key`'s value as `hex settings` shows it, as a binding: the one
/// saved when it is the value now (Esc, which `hex settings` can't name),
/// else as read back, a key alone (Return) too.
fn binding(ui: &Ui, (key, target): (&str, ShortcutTarget), value: &Value) -> Option<HotkeyBinding> {
    if crate::assistant::changes::current(&ui.settings, key).as_ref() == Some(value) {
        return s::binding(&ui.settings, target).cloned();
    }
    let rules = super::super::shortcut_rules();
    let text = value.as_str()?;
    settings::parse_hotkey(text, rules.key_named)
        .or_else(|why| {
            let key = (rules.key_named)(text).map_err(|_| why)?;
            Ok::<_, String>(HotkeyBinding {
                modifiers: HotkeyModifiers::default(),
                key: Some(key),
            })
        })
        .ok()
}

/// A binding as one line of keycaps: `Right ⌥`, `⌥⇧V`; "Not set".
fn caps(binding: Option<&HotkeyBinding>) -> String {
    binding.map_or_else(
        || tr("Not set").to_owned(),
        // ⌥⇧V on a Mac; Ctrl+Alt+V where caps are words.
        |binding| {
            s::shortcut_caps(binding, false).join(match cfg!(target_os = "macos") {
                true => "",
                false => "+",
            })
        },
    )
}

/// Whether the card at `at` is the preset's second step.
fn preset(ui: &Ui, at: At) -> bool {
    ui.assistant
        .store
        .get(at.conversation)
        .and_then(|open| open.turns.get(at.turn))
        .is_some_and(|turn| turn.preset == SHORTCUTS)
}

/// The settings as the card's save would make them, without row `skip`.
fn after(ui: &Ui, card: &Card, skip: Option<usize>) -> crate::app_settings::AppSettings {
    shortcuts::after(card, &ui.drafted(), skip)
}

/// The card at `at`; `primary` when its main button is the screen's one.
pub(super) fn view(ui: &Ui, p: Paint, at: At, primary: bool, layer: &mut Vec<El>) -> Option<El> {
    let card = act::card(ui, at)?;
    let shown = act::shown(ui, at)?;
    let open = matches!(shown, Shown::Proposed { .. });
    let preset = preset(ui, at);
    let c = &p.k.c;
    let head = row()
        .key("head")
        .px(14.0)
        .py(11.0)
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .child(
            text(tr("Shortcuts"))
                .key("title")
                .font(rune::TextStyle {
                    weight: 600.0,
                    ..p.k.ty.body_medium
                })
                .color(p.text()),
        )
        .children(open.then(|| {
            text(tr("Click a field and press the keys"))
                .key("hint")
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap()
                .shrink()
        }));
    let mut list = col()
        .key(format!("shortcuts{}-{}", at.turn, at.card))
        .role("group")
        .label(tr("Shortcuts"))
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .border(p.hair, c.border_base)
        .clip_soft(0.0)
        .child(head);
    let mut mode = false;
    for (index, item) in card.items.iter().enumerate() {
        let line = if MODE.contains(&item.key.as_str()) {
            if std::mem::replace(&mut mode, true) {
                continue;
            }
            mode_line(ui, p, at, card, index, (open, preset), layer)
        } else {
            match key_line(ui, p, at, (index, item), (open, preset, &shown)) {
                Some(line) => line,
                None => continue,
            }
        };
        list = list.child(rule(p, &format!("rule{index}"))).child(line);
    }
    Some(
        list.child(rule(p, "rule-foot"))
            .child(footer(p, at, card, &shown, (primary, preset))),
    )
}

/// A hairline between rows.
fn rule(p: Paint, key: &str) -> El {
    el().key(key.to_owned())
        .h(p.hair)
        .w_full()
        .fill(p.k.c.border_weak)
}

/// A row: its glyph, its name and what it does, a note, and its value;
/// the name sits level with the middle of a one-line value.
fn line(p: Paint, key: String, glyph: &'static Icon, words: El, value: El) -> El {
    row()
        .key(key)
        .px(14.0)
        .py(9.0)
        .gap(10.0)
        .items_start()
        .child(
            row()
                .key("lead")
                .pt(9.0)
                .child(icon(glyph, 14.0).key("glyph").color(p.k.c.icon_faint)),
        )
        .child(words.pt(5.0))
        .child(value)
}

/// A row's words: its name, General's line under it, and a note.
fn words(p: Paint, name: &str, purpose: Option<&str>, notes: Vec<El>) -> El {
    col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(
            text(name.to_owned())
                .key("name")
                .font(rune::TextStyle {
                    weight: 520.0,
                    ..p.k.ty.ui
                })
                .color(p.text())
                .wrap(),
        )
        .children(purpose.map(|purpose| {
            text(purpose.to_owned())
                .key("purpose")
                .font(DESCRIPTION)
                .color(p.muted())
                .wrap()
        }))
        .children(notes)
}

/// A row the preset already matches: its soft mark and "Already set".
fn already(p: Paint) -> El {
    row()
        .key("already")
        .gap(4.0)
        .center_y()
        .child(icon(&theme::CHECK, 12.0).key("mark").color(p.success()))
        .child(
            text(tr("Already set"))
                .key("words")
                .font(DESCRIPTION)
                .color(p.muted()),
        )
}

/// What a row turns on beside it, before it is saved.
fn also(ui: &Ui, p: Paint, item: &Item) -> Option<El> {
    let (key, _) = item.requires.iter().find(|(key, value)| {
        crate::assistant::changes::current(&ui.settings, key).as_ref() != Some(value)
    })?;
    let name = names::place(key, &ui.settings).map_or_else(
        || key.clone(),
        |place| format!("{} › {}", tr_window(place.path), tr_window(&place.name)),
    );
    Some(
        text(format!("{} {name}", tr("Also turns on")))
            .key("also")
            .font(DESCRIPTION)
            .color(p.faint())
            .wrap_anywhere(),
    )
}

/// The value before a change, and the arrow to what comes after it.
fn was(p: Paint, label: &str) -> [El; 2] {
    [
        value::short(p, "from", label, false, false),
        text("→").key("arrow").font(p.k.ty.small).color(p.faint()),
    ]
}

/// A shortcut's row: from → its recorder while it changes, the recorder
/// alone while it doesn't.
fn key_line(
    ui: &Ui,
    p: Paint,
    at: At,
    (index, item): (usize, &Item),
    (open, preset, shown): (bool, bool, &Shown),
) -> Option<El> {
    let (target, name, purpose, glyph) = place(&item.key)?;
    let kept = matches!(shown, Shown::Declined | Shown::Undone);
    let mut notes = Vec::new();
    if preset && !item.included && !kept {
        notes.push(already(p));
    }
    if open && item.included {
        notes.extend(also(ui, p, item));
    }
    let shows = if kept { &item.from } else { &item.to };
    let keys = binding(ui, (&item.key, target), shows);
    let control = if open {
        let listening = ui.assistant.recording == Some((at, index))
            && matches!(ui.capture, Capture::Listening { target: active, .. } if active == target);
        let face = match listening {
            true => s::face(ui, target),
            false => Face::Shortcut { saved: false },
        };
        recorder(
            p,
            "shortcut-keys",
            (face, listening),
            keys.as_ref(),
            tr("Not set"),
            move |ui: &mut Ui| record(ui, at, index, target),
        )
    } else {
        keys_tray(p, keys.as_ref(), tr("Not set"))
    };
    let value = row()
        .key("value")
        .gap(6.0)
        .center_y()
        .children(match item.included && !kept {
            true => was(
                p,
                &caps(binding(ui, (&item.key, target), &item.from).as_ref()),
            )
            .to_vec(),
            false => Vec::new(),
        })
        .child(control);
    let words = words(p, name, purpose, notes);
    Some(line(p, format!("row{index}"), glyph, words, value).label(name))
}

/// The Shortcut mode's one row in place of its three settings, with the
/// choice General's modal makes, as a select.
fn mode_line(
    ui: &Ui,
    p: Paint,
    at: At,
    card: &Card,
    index: usize,
    (open, preset): (bool, bool),
    layer: &mut Vec<El>,
) -> El {
    let rows: Vec<&Item> = card
        .items
        .iter()
        .filter(|item| MODE.contains(&item.key.as_str()))
        .collect();
    let changed = rows.iter().any(|item| item.included);
    let now = Trigger::of(&ui.drafted());
    let then = after(ui, card, None);
    let chosen = Trigger::of(&then);
    let name = tr_window("Shortcut mode");
    let mut notes = Vec::new();
    if preset && !changed && open {
        notes.push(already(p));
    }
    let control = if open {
        let choices: Vec<Trigger> = Trigger::ALL
            .into_iter()
            .filter(|trigger| trigger.unavailable(&then).is_none())
            .collect();
        let labels = choices.iter().map(|trigger| trigger.name()).collect();
        let selected = choices
            .iter()
            .position(|trigger| *trigger == chosen)
            .unwrap_or(0);
        quiet_select(
            ui,
            p,
            Menu::Companion(at.value(index, Slot::To)),
            "shortcut-mode".to_owned(),
            name,
            labels,
            selected,
            move |ui: &mut Ui, picked| {
                if let Some(trigger) = choices.get(picked) {
                    pick(ui, at, *trigger);
                }
            },
            layer,
        )
    } else {
        value::short(p, "to", chosen.name(), false, false)
    };
    let value = row()
        .key("value")
        .gap(6.0)
        .center_y()
        .children(match changed {
            true => was(p, now.name()).to_vec(),
            false => Vec::new(),
        })
        .child(control);
    let words = words(p, name, None, notes);
    line(p, format!("row{index}"), &HAND, words, value).label(name)
}

/// The buttons while it waits, then how the save went and Undo.
fn footer(p: Paint, at: At, card: &Card, shown: &Shown, (primary, preset): (bool, bool)) -> El {
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
            .wrap_anywhere()
            .shrink()
    };
    let space = || el().key("space").grow(1.0);
    match shown {
        Shown::Proposed { .. } => {
            let variant = match primary {
                true => rune_kit::ButtonVariant::Contrast,
                false => rune_kit::ButtonVariant::Neutral,
            };
            let label = match preset {
                true => tr("Use these"),
                false => tr("Save"),
            };
            let changed = card.items.iter().any(|item| item.included);
            let save = rune_kit::Button::new(label)
                .variant(variant)
                .disabled(!changed)
                .on_press(move |ui: &mut Ui| save(ui, at));
            foot = foot.child(space());
            if preset {
                foot = foot.child(push(
                    p,
                    "keep",
                    tr("Keep mine"),
                    Look::Secondary,
                    move |ui, _| keep(ui, at),
                ));
            }
            foot = foot.child(save.build_small_push(p.k).key("save"));
        }
        Shown::Applied { .. } | Shown::ChangedSince { .. } => {
            foot = foot
                .child(status(tr("Saved").to_owned(), p.success()))
                .child(space());
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
                    move |ui, _| save(ui, at),
                ));
        }
        _ => {
            let words = match shown {
                Shown::Undone => tr("Put back as it was"),
                _ => tr("Your shortcuts stay as they are."),
            };
            foot = foot
                .child(status(words.to_owned(), p.muted()))
                .child(space())
                .child(push(
                    p,
                    "anyway",
                    tr("Change anyway"),
                    Look::Ghost,
                    move |ui, _| save(ui, at),
                ));
        }
    }
    foot
}

/// Row `index` records: the way General records (the host pauses the
/// global shortcuts until it ends); pressed again, it stops.
fn record(ui: &mut Ui, at: At, index: usize, target: ShortcutTarget) {
    let again = ui.assistant.recording == Some((at, index));
    s::begin_capture(ui, target);
    if again {
        return s::cancel_capture(ui);
    }
    if matches!(ui.capture, Capture::Listening { .. }) {
        ui.assistant.recording = Some((at, index));
    }
}

/// A chord recorded for the row that records: into the card, unsaved,
/// once it passes General's rules against the card's other rows as they
/// would be saved; else the recorder says why and keeps listening.
pub(in super::super) fn recorded(
    ui: &mut Ui,
    target: ShortcutTarget,
    binding: Result<HotkeyBinding, &'static str>,
) {
    let Some((at, index)) = ui.assistant.recording else {
        return;
    };
    let Some(card) = act::card(ui, at).cloned() else {
        return s::cancel_capture(ui);
    };
    let others = after(ui, &card, Some(index));
    let item = card.items.get(index).cloned().unwrap_or_default();
    let saves = |binding: HotkeyBinding| {
        s::validate(&others, target, &binding)?;
        // What General saves at once the card saves through `check`,
        // which takes no lone Return or Esc: refused here, not at Save.
        let to = Value::String(settings::format_hotkey(&binding));
        let mut values: serde_json::Map<String, Value> = item.requires.iter().cloned().collect();
        values.insert(item.key.clone(), to.clone());
        match settings::check(&others, &values, &super::super::shortcut_rules()) {
            Ok(_) => Ok(to),
            Err(_) if binding.modifiers.is_empty() => Err(tr("Add a modifier")),
            Err(_) => Err(tr("Unsupported key")),
        }
    };
    // As `hex settings` reads it back, so it compares with the saved ones.
    let rules = super::super::shortcut_rules();
    let binding = binding.map(|binding| {
        settings::parse_hotkey(&settings::format_hotkey(&binding), rules.key_named)
            .unwrap_or(binding)
    });
    match binding.and_then(saves) {
        Ok(to) => {
            s::cancel_capture(ui);
            act::set(ui, at, |card| shortcuts::set(card, index, to));
        }
        Err(message) => {
            ui.capture = Capture::Listening {
                target,
                message: Some(message),
                held: HotkeyModifiers::default(),
            };
        }
    }
}

/// A Shortcut mode picked: its three settings follow it.
fn pick(ui: &mut Ui, at: At, trigger: Trigger) {
    let Some(card) = act::card(ui, at).cloned() else {
        return;
    };
    let mut then = after(ui, &card, None);
    trigger.apply(&mut then);
    act::set(ui, at, |card| {
        for index in 0..card.items.len() {
            let key = card.items[index].key.clone();
            if let Some(to) = MODE
                .contains(&key.as_str())
                .then(|| crate::assistant::changes::current(&then, &key))
                .flatten()
            {
                shortcuts::set(card, index, to);
            }
        }
    });
}

/// Use these, Save: what the rows hold, in one save.
fn save(ui: &mut Ui, at: At) {
    if ui.assistant.recording.is_some_and(|(card, _)| card == at) {
        s::cancel_capture(ui);
    }
    act::apply(ui, at);
}

/// Keep mine: nothing changes.
fn keep(ui: &mut Ui, at: At) {
    if ui.assistant.recording.is_some_and(|(card, _)| card == at) {
        s::cancel_capture(ui);
    }
    act::decline(ui, at);
}

/// Scenes: the preset's step with its dictation row recording.
pub(in super::super) fn preset_recording(ui: &mut Ui) {
    super::setup_fixture::setup_preset_shortcuts(ui);
    // The first row with a recorder: the dictation shortcut where Denis's
    // is taken, else the next.
    let first = shortcuts_at(ui).and_then(|at| {
        let card = act::card(ui, at)?;
        card.items
            .iter()
            .enumerate()
            .find_map(|(index, item)| Some((at, index, place(&item.key)?.0)))
    });
    if let Some((at, index, target)) = first {
        ui.assistant.recording = Some((at, index));
        ui.capture = Capture::Listening {
            target,
            message: None,
            held: HotkeyModifiers::default(),
        };
    }
}

/// The preset's step with a key recorded in place of Denis's.
pub(in super::super) fn preset_recorded(ui: &mut Ui) {
    super::setup_fixture::setup_preset_shortcuts(ui);
    if let Some(at) = shortcuts_at(ui) {
        recorded_in(ui, at, "paste_last_hotkey", "control+shift+v");
    }
}

/// An answer to "Help me set up my shortcuts": the ones in use as they
/// are, one recorded anew, Save waiting.
pub(in super::super) fn answer(ui: &mut Ui) {
    super::fixture::open(ui);
    let question = "Help me set up my shortcuts";
    let mut turn = super::fixture::turn(
        question,
        "Here are the shortcuts you use. Click one and press the keys you want.",
        &["topics/dictation-shortcuts"],
        1,
    );
    turn.cards = shortcuts::card(&ui.settings, &Default::default(), &Default::default(), &[])
        .into_iter()
        .collect();
    let store = &mut ui.assistant.store;
    let id = store.start(question, None, None);
    store.push(id, turn);
    ui.assistant.open = Some(id);
    let at = At {
        conversation: id,
        turn: 0,
        card: 0,
    };
    recorded_in(ui, at, "paste_last_hotkey", "control+shift+v");
}

/// The open conversation's shortcuts card.
fn shortcuts_at(ui: &Ui) -> Option<At> {
    let open = ui.assistant.conversation()?;
    open.turns.iter().enumerate().find_map(|(turn, shown)| {
        let card = shown
            .cards
            .iter()
            .position(|card| card.kind == crate::assistant::changes::CardKind::Shortcuts)?;
        Some(At {
            conversation: open.id,
            turn,
            card,
        })
    })
}

/// Row `key` of the card at `at` recorded as `to`.
fn recorded_in(ui: &mut Ui, at: At, key: &str, to: &str) {
    act::set(ui, at, |card| {
        if let Some(index) = card.items.iter().position(|item| item.key == key) {
            shortcuts::set(card, index, Value::String(to.into()));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::Decision;

    fn fresh() -> Ui {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.settings = crate::app_settings::AppSettings::default();
        ui
    }

    /// An answer's card on a fresh install, open.
    fn card(ui: &mut Ui) -> At {
        answer(ui);
        ui.settings = crate::app_settings::AppSettings::default();
        let at = shortcuts_at(ui).unwrap();
        let fresh = shortcuts::card(&ui.settings, &Default::default(), &Default::default(), &[]);
        act::set(ui, at, |card| *card = fresh.unwrap());
        at
    }

    /// What the window asked the host since last asked.
    fn sent(ui: &mut Ui) -> Vec<super::super::super::Request> {
        match &mut ui.service {
            super::super::super::Service::Fixture(sent) => std::mem::take(sent),
            _ => Vec::new(),
        }
    }

    fn row(ui: &Ui, at: At, key: &str) -> usize {
        act::card(ui, at)
            .unwrap()
            .items
            .iter()
            .position(|item| item.key == key)
            .unwrap()
    }

    /// A binding as this OS's recorder makes it: `control+v`, `return`.
    fn chord(text: &str) -> HotkeyBinding {
        let rules = super::super::super::shortcut_rules();
        settings::parse_hotkey(text, rules.key_named).unwrap_or_else(|_| HotkeyBinding {
            modifiers: HotkeyModifiers::default(),
            key: Some((rules.key_named)(text).unwrap()),
        })
    }

    /// Recording pauses the global shortcuts as General's does: the host
    /// is asked to record (`CaptureShortcutFor`), and asked to stop once
    /// the key is in the card or Esc ends it.
    #[test]
    fn recording_asks_the_host_to_pause_the_shortcuts_until_it_ends() {
        let mut ui = fresh();
        let at = card(&mut ui);
        let paste = row(&ui, at, "paste_last_hotkey");
        sent(&mut ui);
        record(&mut ui, at, paste, ShortcutTarget::PasteLast);
        assert_eq!(ui.assistant.recording, Some((at, paste)));
        assert!(matches!(ui.capture, Capture::Listening { .. }));
        use super::super::super::Request;
        assert!(sent(&mut ui).iter().any(|request| matches!(
            request,
            Request::CaptureShortcutFor(ShortcutTarget::PasteLast)
        )));
        // Esc: nothing recorded, the shortcuts back.
        s::cancel_capture(&mut ui);
        assert_eq!(ui.assistant.recording, None);
        assert!(
            sent(&mut ui)
                .iter()
                .any(|request| matches!(request, Request::CancelShortcut))
        );
        assert!(
            act::card(&ui, at)
                .unwrap()
                .items
                .iter()
                .all(|item| !item.included)
        );
    }

    /// A recorded key goes into its row, unsaved; Save saves only the
    /// rows changed, with Undo.
    #[test]
    fn save_changes_only_the_rows_recorded() {
        let mut ui = fresh();
        let at = card(&mut ui);
        let before = ui.settings.clone();
        let paste = row(&ui, at, "paste_last_hotkey");
        record(&mut ui, at, paste, ShortcutTarget::PasteLast);
        recorded(
            &mut ui,
            ShortcutTarget::PasteLast,
            Ok(chord("control+shift+v")),
        );
        assert_eq!(ui.assistant.recording, None);
        assert!(matches!(ui.capture, Capture::Idle));
        assert_eq!(ui.settings, before, "nothing saved before Save");
        save(&mut ui, at);
        let card = act::card(&ui, at).unwrap();
        let Decision::Applied { after, .. } = &card.decision else {
            panic!("{:?}", card.decision);
        };
        assert_eq!(after.keys().collect::<Vec<_>>(), ["paste_last_hotkey"]);
        assert_eq!(
            crate::assistant::changes::current(&ui.settings, "paste_last_hotkey"),
            Some(Value::String("control+shift+v".into()))
        );
        assert_eq!(ui.settings.dictation_hotkey, before.dictation_hotkey);
        act::undo(&mut ui, at);
        assert_eq!(ui.settings.paste_last_hotkey, before.paste_last_hotkey);
    }

    /// A key another shortcut has is refused with General's words, the
    /// other rows counted as they would be saved.
    #[test]
    fn a_key_in_use_is_refused_against_the_rows_and_the_settings() {
        let mut ui = fresh();
        let at = card(&mut ui);
        // The dictation shortcut as it is now, one with a key: the paste
        // shortcut takes no modifier alone.
        let dictation = chord("control+option+space");
        ui.settings.dictation_hotkey = dictation.clone();
        let paste = row(&ui, at, "paste_last_hotkey");
        record(&mut ui, at, paste, ShortcutTarget::PasteLast);
        recorded(&mut ui, ShortcutTarget::PasteLast, Ok(dictation));
        assert!(matches!(
            ui.capture,
            Capture::Listening { message: Some(message), .. } if message == tr("Already in use")
        ));
        s::cancel_capture(&mut ui);
        // Another row's new key: the dictation row takes ⌃⌥⇧D, then the
        // paste row can't.
        let dictation_row = row(&ui, at, "dictation_hotkey");
        record(&mut ui, at, dictation_row, ShortcutTarget::Dictation);
        let taken = chord("control+option+shift+d");
        recorded(&mut ui, ShortcutTarget::Dictation, Ok(taken.clone()));
        assert!(act::card(&ui, at).unwrap().items[dictation_row].included);
        record(&mut ui, at, paste, ShortcutTarget::PasteLast);
        recorded(&mut ui, ShortcutTarget::PasteLast, Ok(taken));
        assert!(matches!(
            ui.capture,
            Capture::Listening { message: Some(message), .. } if message == tr("Already in use")
        ));
        assert!(!act::card(&ui, at).unwrap().items[paste].included);
    }

    /// A key `hex settings` can't write, Return alone, is refused while
    /// the row records rather than failing the save.
    #[test]
    fn a_key_the_save_cannot_take_is_refused_as_it_is_recorded() {
        let mut ui = fresh();
        let at = card(&mut ui);
        let paste = row(&ui, at, "paste_last_hotkey");
        record(&mut ui, at, paste, ShortcutTarget::PasteLast);
        recorded(&mut ui, ShortcutTarget::PasteLast, Ok(chord("return")));
        assert!(matches!(
            ui.capture,
            Capture::Listening { message: Some(message), .. } if message == tr("Add a modifier")
        ));
        assert!(!act::card(&ui, at).unwrap().items[paste].included);
    }

    /// Keep mine changes nothing.
    #[test]
    fn keep_mine_changes_nothing() {
        let mut ui = fresh();
        ui.settings.transcription.language = "ru".into();
        super::super::preset::begin(&mut ui, String::new());
        let plan = At {
            conversation: ui.assistant.open.unwrap(),
            turn: 0,
            card: 0,
        };
        act::decline(&mut ui, plan);
        let at = shortcuts_at(&ui).unwrap();
        assert!(preset(&ui, at));
        let before = ui.settings.clone();
        keep(&mut ui, at);
        assert_eq!(ui.settings, before);
        assert_eq!(act::card(&ui, at).unwrap().decision, Decision::Declined);
    }

    /// The rows are named as General names them, in the window's
    /// language.
    #[test]
    fn the_rows_are_named_in_the_window_s_language() {
        for (language, dictation, paste) in [
            ("en", "Shortcut", "Paste last dictation"),
            ("ru", "Сочетание клавиш", "Вставить последнюю диктовку"),
        ] {
            crate::desktop::i18n::apply(Some(language));
            assert_eq!(place("dictation_hotkey").unwrap().1, dictation);
            assert_eq!(place("paste_last_hotkey").unwrap().1, paste);
        }
        crate::desktop::i18n::apply(Some("en"));
    }
}
