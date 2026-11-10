//! Assistant › Settings: the model a new conversation asks and how hard it
//! thinks, and whether conversations are kept. Only rows that work now.

use rune::El;

use super::super::Ui;
use super::super::controls::{
    ButtonKind, Paint, button, icon_row, page, section, segmented, setting_row, toggle_row,
};
use super::super::files::settings::{engine_label, engine_select};
use super::super::icons;
use crate::desktop::i18n::{tr, tr_fill};
use crate::files::rewrite::Engine;

const EFFORTS: [&str; 3] = ["low", "medium", "high"];

/// The settings page.
pub(super) fn view(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let settings = &ui.settings;
    let processing = &settings.dictation_processing;
    // What "Same as AI cleanup" resolves to.
    let inherited = Engine::resolve(
        None,
        &processing.server,
        settings.labs.ai_cleanup_server,
        &processing.default_mode,
    );
    let mut model = vec![icon_row(
        p,
        "assistant-engine",
        &icons::MONITOR,
        tr("Runs on"),
        Some(tr(
            "Where a new conversation asks. The field's menu changes it for one conversation; dictation's AI cleanup stays as it is",
        )),
        engine_select(
            ui,
            p,
            "assistant-engine",
            settings.assistant.engine.as_deref(),
            tr("Same as AI cleanup"),
            engine_label(&inherited),
            |settings, value| settings.assistant.engine = value,
            layer,
        ),
    )];
    if super::engine(settings, None, None).effort().is_some() {
        model.push(effort_row(ui, p));
    }
    page("content")
        .pt(8.0)
        .pb(24.0)
        .child(section(p, "assistant-model", tr("Model"), model))
        .child(section(
            p,
            "assistant-conversations",
            tr("Conversations"),
            vec![keep_row(ui, p), clear_row(ui, p)],
        ))
}

/// Codex's reasoning, while Assistant runs on Codex.
fn effort_row(ui: &Ui, p: Paint) -> El {
    let saved = match ui.settings.assistant.effort.trim() {
        "" => "medium",
        effort => effort,
    };
    let selected = EFFORTS.iter().position(|e| *e == saved).unwrap_or(1);
    icon_row(
        p,
        "assistant-effort",
        &icons::SPARKLES,
        tr("Reasoning"),
        Some(tr(
            "Low answers soonest. Higher levels think longer before they answer",
        )),
        segmented(
            p,
            "assistant-effort-picker",
            &[tr("Low"), tr("Medium"), tr("High")],
            selected,
            |ui: &mut Ui, index| {
                let effort = EFFORTS[index.min(EFFORTS.len() - 1)];
                if ui.settings.assistant.effort != effort {
                    ui.update(|settings| settings.assistant.effort = effort.into());
                }
            },
        ),
    )
}

fn keep_row(ui: &Ui, p: Paint) -> El {
    let on = ui.settings.assistant.keep_conversations;
    toggle_row(
        p,
        "assistant-keep",
        tr("Keep on this computer"),
        tr("The 50 most recent. Off, they're gone when VibeHex quits"),
        on,
        move |ui: &mut Ui| {
            ui.update(|settings| settings.assistant.keep_conversations = !on);
        },
    )
}

/// Clear conversations…, then Really clear all? to confirm.
fn clear_row(ui: &Ui, p: Paint) -> El {
    let pane = &ui.assistant;
    let count = pane.store.conversations().len();
    let description = match count {
        0 => tr("None yet").to_owned(),
        1 => tr("1 conversation").to_owned(),
        count => tr_fill("{} conversations", &count.to_string()),
    };
    let control = if pane.clear_armed {
        button(
            p,
            "clear",
            tr("Really clear all?"),
            ButtonKind::Danger,
            |ui, _| clear(ui),
        )
    } else {
        button(
            p,
            "clear",
            tr("Clear conversations…"),
            ButtonKind::Secondary,
            |ui, _| ui.assistant.clear_armed = true,
        )
    };
    setting_row(
        p,
        "assistant-clear",
        tr("Clear conversations"),
        Some(&description),
        if count == 0 { rune::el() } else { control },
    )
}

/// Deletes every conversation, once armed.
fn clear(ui: &mut Ui) {
    let pane = &mut ui.assistant;
    pane.clear_armed = false;
    let ids: Vec<u32> = pane
        .store
        .conversations()
        .iter()
        .map(|conversation| conversation.id)
        .collect();
    for id in ids {
        pane.store.remove(id);
    }
    pane.open = None;
}
