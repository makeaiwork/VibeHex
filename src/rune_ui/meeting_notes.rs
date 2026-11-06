//! Meetings › Settings › Notes: who writes titles, notes, and live
//! chapters, how hard Codex thinks, and the summary prompt, whose page
//! opens from Edit…: the prompt for the speech model's language, Reset to
//! default, and Save.

use rune::{El, TextEdit, col, row, text};
use rune_kit::{
    Button, ButtonVariant, Menu as KitMenu, MenuItem, Placement, Select, SelectAppearance, TextArea,
};

use super::controls::{DESCRIPTION, Paint, PushSize, icon_row, section, segmented};
use super::icons;
use super::lists::{Tone, tone_tag};
use super::settings as s;
use super::{Menu, Ui};
use crate::app_settings::notes_settings::default_prompt;
use crate::app_settings::prompt_defaults::{self, Basis};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

const EFFORTS: [&str; 3] = ["low", "medium", "high"];
/// Written by's menu, wide enough for a built-in model's whole name.
const MENU_WIDTH: f32 = 260.0;

/// One choice in Written by: its saved value (`None` for Apple Intelligence
/// or OpenCode as before), its group, and its label.
struct Writer {
    value: Option<String>,
    group: String,
    label: String,
}

/// What Written by offers, in groups, so a model's name shows whole in the
/// narrow menu: Default, Codex, Built in, then Other.
fn writers(ui: &Ui) -> Vec<Writer> {
    let writer = |value: Option<String>, group: &str, label: String| Writer {
        value,
        group: group.to_owned(),
        label,
    };
    let before = if cfg!(target_os = "macos") {
        tr("Apple Intelligence")
    } else {
        tr("OpenCode, when turned on")
    };
    let mut writers = vec![writer(None, tr("Default"), before.to_owned())];
    let mut others = Vec::new();
    if ui.settings.labs.ai_cleanup_server {
        for model in &ui.files.codex_models {
            let value = Some(format!("codex:{}", model.id));
            writers.push(writer(value, "Codex", model.name.clone()));
        }
        for engine in super::files::settings::engine_choices(ui) {
            if engine.value == "opencode" {
                continue;
            }
            if engine.local {
                writers.push(writer(Some(engine.value), tr("Built in"), engine.label));
            } else {
                others.push(writer(Some(engine.value), tr("Other"), engine.label));
            }
        }
    }
    writers.extend(others);
    writers.push(writer(
        Some("opencode".into()),
        tr("Other"),
        "OpenCode".into(),
    ));
    writers
}

/// The Notes section: Written by, Reasoning on Codex, and the prompt.
pub(super) fn notes_section(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let notes = &ui.settings.meetings.notes;
    let writers = writers(ui);
    let current = writers
        .iter()
        .position(|writer| writer.value == notes.writer)
        .unwrap_or_default();
    let menu = Menu::Pick("meeting-notes-writer");
    let open = ui.open == Some(menu);
    // The select's own field, over a menu wide enough for a model's name.
    let trigger = Select::new("meeting-notes-writer", &[] as &[&str])
        .appearance(SelectAppearance::Inline)
        .current(writers.get(current).map(|writer| writer.label.as_str()))
        .open(open)
        .on_toggle(move |ui: &mut Ui| s::toggle_menu(ui, menu))
        .trigger(p.k);
    let mut list = KitMenu::new("meeting-notes-writer-menu")
        .placement(Placement::BottomEnd)
        .gutter(2.0)
        .min_width(MENU_WIDTH)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None);
    let mut group = None;
    for (index, writer) in writers.into_iter().enumerate() {
        if group.as_ref() != Some(&writer.group) {
            list = list.group_label(writer.group.clone());
            group = Some(writer.group);
        }
        let value = writer.value;
        list = list.item(
            MenuItem::new(writer.label)
                .checked(index == current)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    let writer = value.clone();
                    ui.update(|settings| settings.meetings.notes.writer = writer);
                }),
        );
    }
    let select = list.trigger(trigger).build(p.k, layer).mr(-8.0);
    // Left to an Apple Intelligence this Mac can't run: say why, and who
    // writes the notes instead.
    let unavailable =
        cfg!(target_os = "macos") && notes.writer.is_none() && !ui.platform.notes.is_available();
    let description = if unavailable {
        let why = super::platform_settings::notes_description(ui.platform.notes);
        if ui.settings.labs.ai_cleanup_server {
            format!(
                "{why}. {}",
                tr("Your AI cleanup model writes the notes instead")
            )
        } else {
            format!("{why}. {}", tr("Pick another writer to get notes"))
        }
    } else {
        tr("Title, notes, and live chapters. Apple Intelligence keeps the transcript on this Mac; any other writer receives it").to_owned()
    };
    let mut rows = vec![icon_row(
        p,
        "meeting-notes-writer",
        &icons::SPARKLES,
        tr("Written by"),
        Some(&description),
        select,
    )];
    let codex = notes
        .writer
        .as_deref()
        .is_some_and(|writer| writer.starts_with("codex"));
    if codex {
        let selected = EFFORTS
            .iter()
            .position(|effort| *effort == notes.effort())
            .unwrap_or_default();
        rows.push(icon_row(
            p,
            "meeting-notes-effort",
            &icons::CLOCK,
            tr("Reasoning"),
            Some(tr(
                "Low answers soonest. Higher levels think longer before they answer",
            )),
            segmented(
                p,
                "meeting-notes-effort-picker",
                &[tr("Low"), tr("Medium"), tr("High")],
                selected,
                |ui: &mut Ui, index| {
                    let effort = EFFORTS[index.min(EFFORTS.len() - 1)];
                    if ui.settings.meetings.notes.effort() != effort {
                        ui.update(|settings| settings.meetings.notes.effort = effort.into());
                    }
                },
            ),
        ));
    }
    if notes.writer.is_some() || !cfg!(target_os = "macos") {
        rows.push(prompt_row(ui, p));
    }
    section(p, "meeting-notes", tr("Notes"), rows)
}

/// The prompts' language and why, and the speech model's name.
fn language(ui: &Ui) -> (&'static str, Basis, &'static str) {
    let selection = &ui.settings.transcription;
    let (language, basis) = prompt_defaults::prompt_language(selection);
    let model = crate::transcription::models::definition(selection.model).name;
    (language, basis, model)
}

fn prompt_row(ui: &Ui, p: Paint) -> El {
    let (language, basis, model) = language(ui);
    let name = prompt_defaults::language_name(language);
    let description = match basis {
        Basis::SpeechModel => tr_named(
            "What the notes cover and in what order. Default for {language}, the language of your speech model {model}",
            &[("language", name), ("model", model)],
        ),
        Basis::Interface => tr_named(
            "What the notes cover and in what order. Default for {language}, the app's language",
            &[("language", name)],
        ),
    };
    let status = if ui.settings.meetings.notes.edited(language) {
        tr("Edited")
    } else {
        tr("Default")
    };
    let control = row()
        .key("prompt-control")
        .gap(10.0)
        .center_y()
        .child(
            text(status)
                .key("status")
                .font(DESCRIPTION)
                .color(p.faint()),
        )
        .child(
            Button::new(tr("Edit…"))
                .variant(ButtonVariant::Neutral)
                .on_press(open_prompt)
                .build_push(p.k)
                .key("meeting-prompt-edit"),
        );
    icon_row(
        p,
        "meeting-notes-prompt",
        &icons::MESSAGE,
        tr("Summary prompt"),
        Some(&description),
        control,
    )
}

/// Opens the prompt page on the prompt for the speech model's language.
pub(super) fn open_prompt(ui: &mut Ui) {
    let (language, _, _) = language(ui);
    let prompt = ui.settings.meetings.notes.prompt(language);
    ui.meetings.prompt = Some(TextEdit::new(prompt));
}

/// The summary prompt's page.
pub(super) fn prompt_page(ui: &Ui, p: Paint, edit: &TextEdit) -> El {
    let (language, basis, model) = language(ui);
    let saved = ui.settings.meetings.notes.prompt(language);
    let default = default_prompt(language);
    let typed = edit.text.trim();
    let note = match basis {
        Basis::SpeechModel => tr_fill(
            "The language of your speech model, {}. Pick another model or language and the prompts switch to that language's defaults",
            model,
        ),
        Basis::Interface => tr_fill(
            "Your speech model, {}, isn't set to one language Hex writes prompts in, so they follow the app's language",
            model,
        ),
    };
    let back = Button::new(format!("‹ {}", tr("Settings")))
        .variant(ButtonVariant::Ghost)
        .on_press(|ui: &mut Ui| ui.meetings.prompt = None)
        .build_small_push(p.k)
        .key("prompt-back");
    let reset = Button::new(tr("Reset to default"))
        .variant(ButtonVariant::Neutral)
        .disabled(typed == default.trim())
        .on_press(move |ui: &mut Ui| ui.meetings.prompt = Some(TextEdit::new(default.to_owned())))
        .build_push(p.k)
        .key("prompt-reset");
    let save = Button::new(tr("Save"))
        .variant(ButtonVariant::Contrast)
        .disabled(typed.is_empty() || typed == saved.trim())
        .on_press(move |ui: &mut Ui| save_prompt(ui, language))
        .build_push(p.k)
        .key("prompt-save");
    col()
        .key("meeting-prompt-page")
        .gap(12.0)
        .child(row().key("back-row").child(back))
        .child(
            text(tr("Summary prompt"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(
            row()
                .key("language")
                .gap(10.0)
                .child(tone_tag(
                    p,
                    "language-tag",
                    prompt_defaults::language_name(language),
                    Tone::Neutral,
                ))
                .child(
                    text(note)
                        .key("note")
                        .font(DESCRIPTION)
                        .color(p.muted())
                        .wrap()
                        .grow(1.0)
                        .shrink(),
                ),
        )
        .child(
            TextArea::new(edit)
                .rows(18)
                .max_height(560.0)
                .width(None)
                .on_edit(|ui: &mut Ui| ui.meetings.prompt.get_or_insert_with(TextEdit::default))
                .build(p.k)
                .key("prompt"),
        )
        .child(
            row()
                .key("footer")
                .gap(8.0)
                .center_y()
                .child(
                    text(if typed == default.trim() {
                        tr("Default prompt")
                    } else {
                        tr("Edited")
                    })
                    .key("status")
                    .font(DESCRIPTION)
                    .color(p.faint())
                    .grow(1.0)
                    .shrink(),
                )
                .child(reset)
                .child(save),
        )
}

/// Saves the typed prompt for `language`; Hex's own is saved as none.
fn save_prompt(ui: &mut Ui, language: &'static str) {
    let Some(edit) = ui.meetings.prompt.as_ref() else {
        return;
    };
    let typed = edit.text.trim().to_owned();
    if typed.is_empty() {
        return;
    }
    ui.update(|settings| {
        let prompts = &mut settings.meetings.notes.prompts;
        if typed == default_prompt(language).trim() {
            prompts.remove(language);
        } else {
            prompts.insert(language.to_owned(), typed);
        }
    });
}
