//! The request field, as in Codex and Claude Code: the text takes the full
//! width and grows upward to six lines; a compact row under it holds the
//! model and, for Codex, the reasoning as borderless pills, and a small
//! round Send that turns into Stop while an answer comes. Enter sends,
//! Shift+Enter adds a line.
//!
//! In a session the pills choose the engine for its next question and are
//! kept with the session; elsewhere they choose it for the next new session,
//! starting from Files › Settings.

use std::sync::atomic::AtomicBool;

use rune::{El, TextEdit, col, icon, row, text};
use rune_kit::{Menu as KitMenu, MenuItem, Placement};

use super::super::Ui;
use super::super::controls::Paint;
use super::actions::{menu_open, toggle};
use super::detail::{self, current};
use super::{Tab, settings, submit};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::codex;
use crate::files::rewrite::Engine;
use crate::files::store::{self, Version, VersionKind};
use crate::files::worker::{self, Config, Job};

pub(in super::super) static SEND_ICON: rune::Icon = rune::Icon {
    name: "hex-files-composer-send",
    view: 16.0,
    body: r#"<path d="M8 12.5V3.8M4.2 7.4 8 3.6l3.8 3.8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(in super::super) static STOP_ICON: rune::Icon = rune::Icon {
    name: "hex-files-composer-stop",
    view: 16.0,
    body: r#"<rect x="4.5" y="4.5" width="7" height="7" rx="1.5" fill="currentColor"/>"#,
};

const MODEL_MENU: &str = "files-composer-model";
const EFFORT_MENU: &str = "files-composer-effort";
/// The reasoning levels the pill offers.
pub(in super::super) const EFFORTS: [(&str, &str); 3] =
    [("low", "Low"), ("medium", "Medium"), ("high", "High")];
/// The text's height at most: six lines, then it scrolls.
pub(in super::super) const MAX_LINES: usize = 6;

/// Where the field sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// A follow-up in this request session.
    Session(u32),
    /// A new session on the open version.
    New,
}

/// The engine `target`'s next question runs on, and its effort.
fn chosen(ui: &Ui, target: Target) -> (Option<String>, Option<String>) {
    match target {
        Target::Session(id) => ui
            .files
            .selected()
            .and_then(|record| record.version(id))
            .map(|version| (version.engine.clone(), version.effort.clone()))
            .unwrap_or_default(),
        Target::New => (ui.files.ask_engine.clone(), ui.files.ask_effort.clone()),
    }
}

fn engine(ui: &Ui, target: Target) -> Engine {
    let (engine, effort) = chosen(ui, target);
    Config::from_settings(&ui.settings).request_engine(engine.as_deref(), effort.as_deref())
}

/// The field with its pills and Send, or Stop while `running` answers.
/// `waiting` names another job that holds the field.
pub(super) fn field(
    ui: &Ui,
    p: Paint,
    target: Target,
    placeholder: &str,
    running: Option<u64>,
    waiting: Option<&str>,
    layer: &mut Vec<El>,
) -> El {
    let c = &p.k.c;
    let live = waiting.is_none() && running.is_none();
    let edit = &ui.files.ask;
    let style = p.k.ty.ui;
    let placeholder = match waiting {
        Some(name) => tr_fill("Wait for {}, or stop it", name),
        None => placeholder.to_owned(),
    };
    let mut input = rune::text_input(edit, style)
        .key("input")
        .w_full()
        .color(c.text_base)
        .placeholder(placeholder.clone(), p.placeholder())
        .caret(c.text_base, p.k.selection());
    if waiting.is_none() {
        input = input
            .on_edit(|ui: &mut Ui| &mut ui.files.ask)
            .on_submit(move |ui: &mut Ui, _| {
                if !busy(ui, target) {
                    let question = ui.files.ask.text.clone();
                    send(ui, target, question);
                }
            });
    }
    let text_area = rune::auto_grow(input, 1, MAX_LINES).max_h(120.0);
    let engine = engine(ui, target);
    let mut pills = row()
        .key("pills")
        .gap(2.0)
        .center_y()
        .child(model_pill(ui, p, target, &engine, layer));
    if let Some(effort) = engine.effort() {
        pills = pills.child(effort_pill(ui, p, target, effort, layer));
    }
    let action = match running {
        Some(task) => round_button(p, "stop", tr("Stop"), &STOP_ICON, true, move |_ui| {
            worker::cancel(task);
        }),
        None => {
            let ready = live && !edit.text.trim().is_empty();
            round_button(p, "send", tr("Send"), &SEND_ICON, ready, move |ui| {
                let question = ui.files.ask.text.clone();
                send(ui, target, question);
            })
        }
    };
    let focused = edit.focused && waiting.is_none();
    let shell = shell(
        p,
        "files-composer",
        placeholder,
        text_area,
        pills,
        action,
        (focused, ui.files.text_under),
    );
    if waiting.is_some() {
        shell.dim(0.55)
    } else {
        shell
    }
}

/// The field's frame: the text over the pills and Send, a focus ring while
/// typing, and the floating shadows once text runs under it (`state` is
/// focused, lifted).
pub(in super::super) fn shell(
    p: Paint,
    key: &str,
    label: String,
    text_area: El,
    pills: El,
    action: El,
    state: (bool, bool),
) -> El {
    let c = &p.k.c;
    let (focused, lifted) = state;
    let mut shell = col()
        .key(key.to_owned())
        .role("group")
        .label(label)
        .gap(6.0)
        .pt(10.0)
        .pb(6.0)
        .pl(12.0)
        .pr(6.0)
        .radius(p.k.radius.md + 2.0)
        .fill(c.bg_base)
        .child(text_area)
        .child(
            row()
                .key("controls")
                .center_y()
                .child(pills.grow(1.0).shrink())
                .child(action),
        );
    shell = if focused {
        shell
            .outline(1.0, 0.0, c.border_focus)
            .box_shadow(rune::Shadow {
                spread: 3.0,
                color: c.border_focus.alpha(0.18),
                ..Default::default()
            })
    } else {
        shell.box_shadow(rune::Shadow {
            spread: 1.0,
            color: c.border_base,
            ..Default::default()
        })
    };
    if lifted {
        for shadow in &p.k.elevation.floating {
            shell = shell.box_shadow(*shadow);
        }
    }
    shell
}

/// Whether `target` has an answer coming.
fn busy(ui: &Ui, target: Target) -> bool {
    running(ui, target).is_some()
}

/// The follow-up answering in `target`, if any.
pub(super) fn running(ui: &Ui, target: Target) -> Option<u64> {
    let Target::Session(id) = target else {
        return None;
    };
    let record = ui.files.selected.as_deref()?;
    ui.files
        .tasks_of(record)
        .find(|task| task.session == Some(id))
        .map(|task| task.id)
}

/// A small round Send or Stop: muted until it has something to do.
pub(in super::super) fn round_button(
    p: Paint,
    key: &str,
    label: &str,
    glyph: &'static rune::Icon,
    ready: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    let c = &p.k.c;
    let (fill, ink) = if ready {
        (c.text_base, c.bg_base)
    } else {
        (c.layer_02, c.icon_muted)
    };
    let button = row()
        .key(key)
        .role("button")
        .label(label)
        .size(26.0, 26.0)
        .center()
        .radius(13.0)
        .fill(fill)
        .child(icon(glyph, 14.0).key("glyph").color(ink));
    if !ready {
        return button;
    }
    button
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .hover(c.text_base.alpha(0.85))
        .press(c.text_base.alpha(0.7))
        .on_press(press)
}

/// A borderless pill: plain text with a chevron, a quiet background on hover.
pub(in super::super) fn pill(
    p: Paint,
    key: &str,
    label: &str,
    open: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    let c = &p.k.c;
    let chevron = if open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    row()
        .key(key)
        .role("button")
        .label(label)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .h(24.0)
        .px(6.0)
        .gap(3.0)
        .center_y()
        .radius(6.0)
        .hover(c.hover)
        .press(c.pressed)
        .child(
            text(label.to_owned())
                .key("label")
                .font(rune::TextStyle {
                    size: 12.0,
                    ..p.k.ty.ui
                })
                .color(c.text_weak)
                .max_lines(1),
        )
        .child(icon(chevron, 11.0).key("chevron").color(c.icon_muted))
        .on_press(press)
}

/// The model pill: Codex's models when Codex is set up, then the other
/// engines Files can run on. The choice applies from the next question.
fn model_pill(ui: &Ui, p: Paint, target: Target, engine: &Engine, layer: &mut Vec<El>) -> El {
    let group = match target {
        Target::Session(_) => tr("Model for this session"),
        Target::New => tr("Model for the next session"),
    };
    model_menu(
        ui,
        p,
        MODEL_MENU,
        group,
        engine,
        layer,
        move |ui: &mut Ui, value: String| choose(ui, target, Some(value), None),
    )
}

/// The model pill and its menu, saved under `menu`: Codex's models when
/// Codex is set up, then the other engines requests can run on; `pick`
/// hears the chosen engine in `files.request_engine`'s form.
pub(in super::super) fn model_menu(
    ui: &Ui,
    p: Paint,
    menu: &'static str,
    group: &str,
    engine: &Engine,
    layer: &mut Vec<El>,
    pick: impl Fn(&mut Ui, String) + Clone + 'static,
) -> El {
    let open = menu_open(ui, menu);
    // Codex's own names, once its models are read here.
    let codex = engine.codex_model();
    let listed = codex.and_then(|id| {
        ui.files.codex_models.iter().find(|model| {
            if id.is_empty() {
                model.default
            } else {
                model.id == id
            }
        })
    });
    let label = listed.map_or_else(|| engine.model_name(), |model| model.name.clone());
    let key = menu;
    let mut menu = KitMenu::new(format!("{key}-menu"))
        .placement(Placement::TopStart)
        .gutter(4.0)
        .min_width(260.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(group);
    let server = &ui.settings.dictation_processing.server;
    let codex_set_up = server.runs_on == crate::app_settings::CleanupRunsOn::Codex
        || !server.codex_model.trim().is_empty();
    if codex_set_up {
        for model in &ui.files.codex_models {
            let value = format!("codex:{}", model.id);
            let checked = listed.is_some_and(|listed| listed.id == model.id);
            let pick = pick.clone();
            menu = menu.item(
                MenuItem::new(model.name.clone())
                    .shortcut(model.description.clone())
                    .checked(checked)
                    .on_select(move |ui: &mut Ui| pick(ui, value.clone())),
            );
        }
        if ui.files.codex_models.is_empty() {
            menu = menu.item(MenuItem::new(tr("Loading Codex models…")).disabled(true));
        }
        menu = menu.separator();
    }
    for choice in &ui.files.engines {
        if choice.value == "codex" {
            continue;
        }
        let value = choice.value.clone();
        let pick = pick.clone();
        menu = menu.item(
            MenuItem::new(choice.label.clone())
                .checked(engine.effort().is_none() && choice.label.contains(&label))
                .on_select(move |ui: &mut Ui| pick(ui, value.clone())),
        );
    }
    let trigger = pill(p, "model", &label, open, move |ui: &mut Ui| {
        toggle(ui, key);
        if menu_open(ui, key) {
            ui.files.engines = settings::engine_choices(ui);
            load_codex_models(ui);
        }
    });
    menu.trigger(trigger).build(p.k, layer)
}

fn effort_pill(ui: &Ui, p: Paint, target: Target, effort: &str, layer: &mut Vec<El>) -> El {
    effort_menu(
        ui,
        p,
        EFFORT_MENU,
        effort,
        layer,
        move |ui: &mut Ui, value: String| choose(ui, target, None, Some(value)),
    )
}

/// The reasoning pill and its menu, saved under `menu`.
pub(in super::super) fn effort_menu(
    ui: &Ui,
    p: Paint,
    menu: &'static str,
    effort: &str,
    layer: &mut Vec<El>,
    pick: impl Fn(&mut Ui, String) + Clone + 'static,
) -> El {
    let open = menu_open(ui, menu);
    let shown = EFFORTS
        .iter()
        .find(|(value, _)| *value == effort)
        .map_or("Medium", |(_, name)| *name);
    let key = menu;
    let mut menu = KitMenu::new(format!("{key}-menu"))
        .placement(Placement::TopStart)
        .gutter(4.0)
        .min_width(160.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(tr("Reasoning"));
    for (value, name) in EFFORTS {
        let pick = pick.clone();
        menu = menu.item(
            MenuItem::new(tr(name))
                .checked(value == effort)
                .on_select(move |ui: &mut Ui| pick(ui, value.into())),
        );
    }
    let trigger = pill(p, "effort", tr(shown), open, move |ui: &mut Ui| {
        toggle(ui, key);
    });
    menu.trigger(trigger).build(p.k, layer)
}

/// Saves a pill's choice: on the session, or for the next new one.
fn choose(ui: &mut Ui, target: Target, engine: Option<String>, effort: Option<String>) {
    ui.open = None;
    match target {
        Target::Session(id) => {
            let Some(record) = ui.files.selected.clone() else {
                return;
            };
            detail::change(ui, &record, |record| {
                if let Some(version) = record.version_mut(id) {
                    if let Some(engine) = &engine {
                        version.engine = Some(engine.clone());
                    }
                    if let Some(effort) = &effort {
                        version.effort = Some(effort.clone());
                    }
                }
            });
        }
        Target::New => {
            if engine.is_some() {
                ui.files.ask_engine = engine;
            }
            if effort.is_some() {
                ui.files.ask_effort = effort;
            }
        }
    }
}

/// Reads Codex's models on their own thread when none are known yet.
pub(in super::super) fn load_codex_models(ui: &mut Ui) {
    if !ui.files.codex_models.is_empty() || ui.files.fixture {
        return;
    }
    let known = codex::models();
    if !known.is_empty() {
        ui.files.codex_models = known;
        return;
    }
    let waker = ui.modes.waker.clone();
    let _ = std::thread::Builder::new()
        .name("hex-files-codex-models".into())
        .spawn(move || {
            if let Err(error) = codex::status(&AtomicBool::new(false)) {
                tracing::warn!(%error, "couldn't list Codex's models");
            }
            if let Some(waker) = &waker {
                waker();
            }
        });
}

/// Picks up Codex's models once a status read has listed them.
pub(super) fn poll(ui: &mut Ui) -> bool {
    poll_models(ui, MODEL_MENU)
}

/// Picks up Codex's models while the model menu saved under `menu` is open.
pub(in super::super) fn poll_models(ui: &mut Ui, menu: &'static str) -> bool {
    if ui.files.codex_models.is_empty() && menu_open(ui, menu) {
        let known = codex::models();
        if !known.is_empty() {
            ui.files.codex_models = known;
            return true;
        }
    }
    false
}

/// Sends the field: a follow-up in a session, else a new session on the
/// open version.
fn send(ui: &mut Ui, target: Target, question: String) {
    let question = question.trim().to_string();
    if question.is_empty() {
        return;
    }
    let Some(record) = ui.files.selected().cloned() else {
        return;
    };
    match target {
        Target::Session(session) => {
            submit(ui, &record.id, Job::FollowUp { session, question });
            ui.files.stale = true;
        }
        Target::New => {
            let Some(Tab::Version(from)) = current(ui, &record) else {
                return;
            };
            start(ui, &record, from, question, None);
        }
    }
    ui.files.ask = TextEdit::default();
}

/// Starts a session on version `from` with `prompt`, from the chip named
/// `chip` or typed.
pub(super) fn start(
    ui: &mut Ui,
    record: &store::FileRecord,
    from: u32,
    prompt: String,
    chip: Option<String>,
) {
    let typed = chip.is_none();
    let name = record.unique_name(&chip.unwrap_or_else(|| worker::request_name(&prompt)));
    if typed && !ui.files.fixture {
        if let Err(error) = store::remember(&prompt) {
            tracing::warn!(%error, "couldn't remember the request");
        }
        ui.files.recent = store::recent();
    }
    // A session started from another session reads that session's source.
    let from = match record.version(from) {
        Some(Version {
            kind: VersionKind::Request,
            from: Some(source),
            ..
        }) => *source,
        _ => from,
    };
    let job = Job::Request {
        from,
        prompt,
        name,
        engine: ui.files.ask_engine.clone(),
        effort: ui.files.ask_effort.clone(),
    };
    if let Some(task) = submit(ui, &record.id, job) {
        ui.files.tab = Some(Tab::Task(task));
    }
}
