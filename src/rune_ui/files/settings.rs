//! Files' Settings: the transcription model and vocabulary, how the
//! Original tab opens, cleanup after transcription with its engine and
//! prompt, and the quick requests above the request field.

use rune::{El, TextEdit, col, row, text};
use rune_kit::{IconButton, IconButtonVariant, TextArea, TextInput};

use super::super::controls::{
    ButtonKind, Choice, DESCRIPTION, METRICS, Paint, button, section, section_with, select,
    setting_row, toggle_row,
};
use super::super::{Menu, Ui};
use crate::app_settings::{DEFAULT_FILES_PROMPT, QuickPrompt};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::local_models;

/// One place cleanup can run, as Settings and Clean up again offer it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct EngineChoice {
    /// As `files.cleanup_engine` saves it.
    pub(in super::super) value: String,
    pub(in super::super) label: String,
    /// A built-in model rather than a service set up in Output.
    pub(in super::super) local: bool,
}

/// The downloaded built-in models, then the services Output › AI cleanup
/// set up, then OpenCode.
pub(in super::super) fn engine_choices(ui: &Ui) -> Vec<EngineChoice> {
    let server = &ui.settings.dictation_processing.server;
    let mut choices: Vec<EngineChoice> = local_models::MODELS
        .iter()
        .filter(|model| model.installed())
        .map(|model| EngineChoice {
            value: format!("local:{}", model.id),
            label: model.name.into(),
            local: true,
        })
        .collect();
    let model = |model: &str| model.rsplit('/').next().unwrap_or(model).to_string();
    if !server.openrouter_model.trim().is_empty() {
        choices.push(EngineChoice {
            value: "openrouter".into(),
            label: tr_fill("OpenRouter: {}", &model(&server.openrouter_model)),
            local: false,
        });
    }
    if !server.url.trim().is_empty() {
        let name = if server.custom_model.trim().is_empty() {
            server.url.clone()
        } else {
            model(&server.custom_model)
        };
        choices.push(EngineChoice {
            value: "custom".into(),
            label: tr_fill("Your server: {}", &name),
            local: false,
        });
    }
    choices.push(EngineChoice {
        value: "opencode".into(),
        label: "OpenCode".into(),
        local: false,
    });
    choices
}

fn open(ui: &Ui, key: &'static str) -> bool {
    ui.open == Some(Menu::Pick(key))
}

fn toggle(key: &'static str) -> impl Fn(&mut Ui) + 'static {
    move |ui: &mut Ui| ui.open = (ui.open != Some(Menu::Pick(key))).then_some(Menu::Pick(key))
}

pub(super) fn view(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Vec<El> {
    let files = &ui.settings.files;
    let dictation = crate::transcription::models::definition(ui.settings.transcription.model).name;
    let same = tr("Same as dictation");
    let model_label = match &files.transcription {
        Some(selection) => crate::transcription::models::definition(selection.model)
            .name
            .to_string(),
        None => same.into(),
    };
    let mut models = Vec::new();
    if open(ui, "files-model") {
        models.push(Choice {
            label: same.into(),
            detail: Some(dictation.into()),
            selected: files.transcription.is_none(),
            choose: Box::new(|ui: &mut Ui| {
                ui.open = None;
                ui.update(|settings| settings.files.transcription = None);
            }),
        });
        for (model, name) in super::super::history::installed_models(ui) {
            models.push(Choice {
                label: name.into(),
                detail: None,
                selected: files.transcription.as_ref().map(|s| s.model) == Some(model),
                choose: Box::new(move |ui: &mut Ui| {
                    ui.open = None;
                    let selection = ui.settings.transcription_for_model(model);
                    ui.update(|settings| settings.files.transcription = Some(selection));
                }),
            });
        }
    }
    let transcription = section(
        p,
        "files-transcription",
        tr("Transcription"),
        vec![
            setting_row(
                p,
                "files-model",
                tr("Model"),
                Some(tr(
                    "Downloaded models only. Its language is the one dictation uses with it.",
                )),
                select(
                    p,
                    "files-model",
                    &model_label,
                    open(ui, "files-model"),
                    models,
                    toggle("files-model"),
                    layer,
                ),
            ),
            toggle_row(
                p,
                "files-vocabulary",
                tr("Use vocabulary"),
                tr("Your vocabulary helps the model hear names and terms, and cleanup write them."),
                files.use_vocabulary,
                |ui: &mut Ui| {
                    let on = !ui.settings.files.use_vocabulary;
                    ui.update(|settings| settings.files.use_vocabulary = on);
                },
            ),
            toggle_row(
                p,
                "files-timestamps",
                tr("Show timestamps"),
                tr("How the Original tab opens. Switch it there in one click."),
                files.show_timestamps,
                |ui: &mut Ui| {
                    let on = !ui.settings.files.show_timestamps;
                    ui.files.timestamps = None;
                    ui.update(|settings| settings.files.show_timestamps = on);
                },
            ),
        ],
    );

    let mut cleanup_rows = vec![
        toggle_row(
            p,
            "files-cleanup-after",
            tr("Clean up after transcription"),
            tr("Adds a Cleanup tab as soon as the text is ready."),
            files.cleanup_after,
            |ui: &mut Ui| {
                let on = !ui.settings.files.cleanup_after;
                ui.update(|settings| settings.files.cleanup_after = on);
            },
        ),
        setting_row(
            p,
            "files-engine",
            tr("Runs on"),
            Some(tr("More models are in Output › AI cleanup.")),
            engine_select(
                ui,
                p,
                "files-engine",
                files.cleanup_engine.as_deref(),
                tr("Same as dictation"),
                |files, value| files.cleanup_engine = value,
                layer,
            ),
        ),
        toggle_row(
            p,
            "files-default-prompt",
            tr("Use the Default mode's prompt"),
            tr("Off, cleanup follows the prompt below."),
            files.use_default_prompt,
            |ui: &mut Ui| {
                let on = !ui.settings.files.use_default_prompt;
                ui.update(|settings| settings.files.use_default_prompt = on);
            },
        ),
    ];
    if !files.use_default_prompt {
        cleanup_rows.push(prompt_row(ui, p));
    }
    let cleanup = section(p, "files-cleanup", tr("AI cleanup"), cleanup_rows);
    let requests = section(
        p,
        "files-requests",
        tr("Requests"),
        vec![setting_row(
            p,
            "files-request-engine",
            tr("Runs on"),
            Some(tr(
                "Summary, Action items, and your own requests. A model with a large window reads a long file whole.",
            )),
            engine_select(
                ui,
                p,
                "files-request-engine",
                files.request_engine.as_deref(),
                tr("Same as cleanup"),
                |files, value| files.request_engine = value,
                layer,
            ),
        )],
    );
    vec![transcription, cleanup, requests, quick_section(ui, p)]
}

/// Where cleanup or requests run: `inherit` (saved as none), then each
/// engine.
fn engine_select(
    ui: &Ui,
    p: Paint,
    key: &'static str,
    current: Option<&str>,
    inherit: &str,
    set: fn(&mut crate::app_settings::FilesSettings, Option<String>),
    layer: &mut Vec<El>,
) -> El {
    let engines = engine_choices(ui);
    let label = match current {
        None => inherit.to_string(),
        Some(value) => engines
            .iter()
            .find(|engine| engine.value == value)
            .map_or_else(|| value.to_string(), |engine| engine.label.clone()),
    };
    let mut choices = vec![Choice {
        label: inherit.into(),
        detail: None,
        selected: current.is_none(),
        choose: Box::new(move |ui: &mut Ui| {
            ui.open = None;
            ui.update(|settings| set(&mut settings.files, None));
        }),
    }];
    for engine in engines {
        let value = engine.value.clone();
        choices.push(Choice {
            label: engine.label,
            detail: engine.local.then(|| tr("built in").to_string()),
            selected: current == Some(engine.value.as_str()),
            choose: Box::new(move |ui: &mut Ui| {
                ui.open = None;
                let value = value.clone();
                ui.update(|settings| set(&mut settings.files, Some(value)));
            }),
        });
    }
    select(p, key, &label, open(ui, key), choices, toggle(key), layer)
}

/// Files' own cleanup prompt, with Save while it differs.
fn prompt_row(ui: &Ui, p: Paint) -> El {
    let saved = &ui.settings.files.prompt;
    let fallback;
    let edit = match &ui.files.prompt {
        Some(edit) => edit,
        None => {
            fallback = TextEdit::new(saved.clone());
            &fallback
        }
    };
    let edited = edit.text.trim() != saved.trim();
    let area = TextArea::new(edit)
        .rows(5)
        .max_height(280.0)
        .width(None)
        .on_edit(|ui: &mut Ui| {
            let saved = ui.settings.files.prompt.clone();
            ui.files.prompt.get_or_insert_with(|| TextEdit::new(saved))
        })
        .build(p.k)
        .key("files-prompt");
    let mut buttons = row().key("buttons").gap(8.0);
    if edited {
        buttons = buttons.child(button(
            p,
            "files-prompt-save",
            tr("Save"),
            ButtonKind::Secondary,
            |ui: &mut Ui, _| {
                if let Some(edit) = ui.files.prompt.take() {
                    let prompt = edit.text.trim().to_owned();
                    ui.update(|settings| settings.files.prompt = prompt);
                }
            },
        ));
    }
    if saved.trim() != DEFAULT_FILES_PROMPT.trim() || edited {
        buttons = buttons.child(button(
            p,
            "files-prompt-reset",
            tr("Use Hex's prompt"),
            ButtonKind::Secondary,
            |ui: &mut Ui, _| {
                ui.files.prompt = None;
                ui.update(|settings| settings.files.prompt = DEFAULT_FILES_PROMPT.into());
            },
        ));
    }
    col()
        .key("files-prompt-row")
        .px(METRICS.row_px)
        .py(12.0)
        .gap(10.0)
        .child(
            text(tr("Cleanup prompt"))
                .key("title")
                .font(p.k.ty.ui)
                .color(p.text()),
        )
        .child(area)
        .child(buttons)
}

/// The quick requests being edited, or the saved ones.
fn quick_edits(ui: &Ui) -> Vec<(TextEdit, TextEdit)> {
    ui.files.quick.clone().unwrap_or_else(|| {
        ui.settings
            .files
            .quick_prompts
            .iter()
            .map(|chip| {
                (
                    TextEdit::new(chip.label.clone()),
                    TextEdit::new(chip.prompt.clone()),
                )
            })
            .collect()
    })
}

fn quick_slot(ui: &mut Ui) -> &mut Vec<(TextEdit, TextEdit)> {
    if ui.files.quick.is_none() {
        ui.files.quick = Some(quick_edits(ui));
    }
    ui.files.quick.get_or_insert_with(Vec::new)
}

fn projected(edits: &[(TextEdit, TextEdit)]) -> Vec<QuickPrompt> {
    edits
        .iter()
        .filter(|(label, prompt)| !label.text.trim().is_empty() && !prompt.text.trim().is_empty())
        .map(|(label, prompt)| QuickPrompt {
            label: label.text.trim().into(),
            prompt: prompt.text.trim().into(),
        })
        .collect()
}

/// The chips above the request field: a name and what it asks.
fn quick_section(ui: &Ui, p: Paint) -> El {
    let edits = quick_edits(ui);
    let edited = ui
        .files
        .quick
        .as_ref()
        .is_some_and(|edits| projected(edits) != ui.settings.files.quick_prompts);
    let mut rows = Vec::new();
    for (index, (label, prompt)) in edits.iter().enumerate() {
        rows.push(
            row()
                .key(format!("quick{index}"))
                .px(METRICS.row_px)
                .py(8.0)
                .gap(8.0)
                .center_y()
                .child(
                    TextInput::new(label)
                        .placeholder(tr("Name"))
                        .width(Some(130.0))
                        .on_edit(move |ui: &mut Ui| &mut quick_slot(ui)[index].0)
                        .build(p.k)
                        .key("label"),
                )
                .child(
                    TextInput::new(prompt)
                        .placeholder(tr("What it asks for"))
                        .width(None)
                        .on_edit(move |ui: &mut Ui| &mut quick_slot(ui)[index].1)
                        .build(p.k)
                        .key("prompt")
                        .grow(1.0)
                        .shrink(),
                )
                .child(
                    IconButton::new(super::super::list_detail::TRASH_ICON)
                        .variant(IconButtonVariant::Ghost)
                        .control_size(rune_kit::ControlSize::Small)
                        .label(tr("Remove"))
                        .on_press(move |ui: &mut Ui| {
                            let edits = quick_slot(ui);
                            if index < edits.len() {
                                edits.remove(index);
                            }
                        })
                        .build(p.k)
                        .key("remove"),
                ),
        );
    }
    if rows.is_empty() {
        rows.push(
            text(tr(
                "No quick requests. Add one to see it above the request field.",
            ))
            .key("quick-empty")
            .font(DESCRIPTION)
            .color(p.faint())
            .px(METRICS.row_px)
            .py(12.0),
        );
    }
    if edited {
        rows.push(
            row()
                .key("quick-actions")
                .px(METRICS.row_px)
                .py(10.0)
                .gap(8.0)
                .child(button(
                    p,
                    "quick-save",
                    tr("Save"),
                    ButtonKind::Secondary,
                    |ui: &mut Ui, _| {
                        if let Some(edits) = ui.files.quick.take() {
                            let quick = projected(&edits);
                            ui.update(|settings| settings.files.quick_prompts = quick);
                        }
                    },
                ))
                .child(button(
                    p,
                    "quick-cancel",
                    tr("Cancel"),
                    ButtonKind::Secondary,
                    |ui: &mut Ui, _| ui.files.quick = None,
                )),
        );
    }
    section_with(
        p,
        "files-quick",
        tr("Quick requests"),
        button(
            p,
            "quick-add",
            tr("Add"),
            ButtonKind::Secondary,
            |ui: &mut Ui, _| quick_slot(ui).push((TextEdit::default(), TextEdit::default())),
        ),
        rows,
    )
}
