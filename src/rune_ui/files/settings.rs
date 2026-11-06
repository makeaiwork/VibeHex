//! Files' Settings: the transcription model and vocabulary, how the
//! Original tab opens, cleanup after transcription with its engine and
//! prompt, and the quick requests above the request field.

use rune::{El, TextEdit, col, icon, row, text};
use rune_kit::{
    IconButton, IconButtonVariant, Menu as KitMenu, MenuItem, Placement, TextArea, TextInput,
};

use super::super::controls::{
    ButtonKind, DESCRIPTION, METRICS, Paint, button, icon_row, icon_toggle_row, section,
    section_with, segmented,
};
use super::super::list_detail as ld;
use super::super::{Menu, Ui, icons};
use crate::app_settings::{CleanupRunsOn, DEFAULT_FILES_PROMPT, QuickPrompt};
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
    // Codex, once Output › AI cleanup has been set to it.
    if server.runs_on == CleanupRunsOn::Codex || !server.codex_model.trim().is_empty() {
        let label = match server.codex_model.trim() {
            "" => "Codex".to_owned(),
            model => tr_fill("Codex: {}", model),
        };
        choices.push(EngineChoice {
            value: "codex".into(),
            label,
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

/// A row's borderless choice, as the mockups draw it: the choice, what it
/// resolves to in grey, and a chevron, opening `items` under it.
fn inline_menu(
    ui: &Ui,
    p: Paint,
    key: &'static str,
    value: &str,
    resolved: Option<String>,
    items: Vec<MenuItem>,
    layer: &mut Vec<El>,
) -> El {
    let c = &p.k.c;
    let is_open = open(ui, key);
    let chevron = if is_open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    let trigger = row()
        .key(format!("{key}-control"))
        .role("button")
        .label(value)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .h(24.0)
        .pl(8.0)
        .pr(4.0)
        .mr(-8.0)
        .gap(6.0)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .press(c.pressed)
        .on_press(toggle(key))
        .child(
            text(value.to_string())
                .key("value")
                .font(p.k.ty.ui_medium)
                .color(c.text_base)
                .truncate(),
        )
        .children(resolved.map(|resolved| {
            text(resolved)
                .key("resolved")
                .font(p.k.ty.text_xsmall_medium)
                .color(p.muted())
                .truncate()
        }))
        .child(icon(chevron, 14.0).key("chevron").color(c.icon_muted));
    let mut menu = KitMenu::new(format!("{key}-menu"))
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(220.0)
        .open(is_open)
        .on_close(|ui: &mut Ui| ui.open = None);
    for item in items {
        menu = menu.item(item);
    }
    menu.trigger(trigger).build(p.k, layer)
}

/// A choice in an [`inline_menu`]: checked when chosen, what it resolves
/// to on the right.
fn choice<R>(
    label: &str,
    resolved: Option<&str>,
    chosen: bool,
    pick: impl Fn(&mut Ui) -> R + 'static,
) -> MenuItem {
    let mut item = MenuItem::new(label.to_string()).checked(chosen);
    if let Some(resolved) = resolved {
        item = item.shortcut(resolved.to_string());
    }
    item.on_select(move |ui: &mut Ui| {
        ui.open = None;
        let _ = pick(ui);
    })
}

/// Where cleanup or requests run, named for a person: `Built in, Qwen3 4B`.
fn engine_label(engine: &crate::files::rewrite::Engine) -> String {
    use crate::dictation::cleanup_server::Provider;
    use crate::files::rewrite::Engine;
    let model = engine.model_name();
    match engine {
        Engine::Server(server) if server.provider == Provider::Local => {
            tr_fill("Built in, {}", &model)
        }
        Engine::Server(server) if server.provider == Provider::OpenRouter => {
            tr_fill("OpenRouter: {}", &model)
        }
        Engine::Server(_) => tr_fill("Your server: {}", &model),
        Engine::OpenCode(_) => "OpenCode".into(),
    }
}

pub(super) fn view(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Vec<El> {
    use crate::transcription::models::{definition, language_name};
    let files = &ui.settings.files;
    let dictation = &ui.settings.transcription;
    let same = tr("Same as dictation");

    // Model: dictation's, or a downloaded one.
    let mut model_items = vec![choice(
        same,
        Some(definition(dictation.model).name),
        files.transcription.is_none(),
        |ui: &mut Ui| {
            ui.update(|settings| {
                settings.files.transcription = None;
                settings.files.language = None;
            })
        },
    )];
    if open(ui, "files-model") {
        for (model, name) in super::super::history::installed_models(ui) {
            model_items.push(choice(
                name,
                None,
                files.transcription.as_ref().map(|s| s.model) == Some(model),
                move |ui: &mut Ui| {
                    let selection = ui.settings.transcription_for_model(model);
                    ui.update(|settings| {
                        settings.files.transcription = Some(selection);
                        settings.files.language = None;
                    });
                },
            ));
        }
    }
    let (model_value, model_resolved) = match &files.transcription {
        Some(selection) => (definition(selection.model).name.to_string(), None),
        None => (
            same.to_string(),
            Some(definition(dictation.model).name.to_string()),
        ),
    };
    let mut transcription_rows = vec![icon_row(
        p,
        "files-model",
        &icons::MIC,
        tr("Model"),
        Some(tr(
            "Another model loads beside dictation's and never changes it",
        )),
        inline_menu(
            ui,
            p,
            "files-model",
            &model_value,
            model_resolved,
            model_items,
            layer,
        ),
    )];

    // Language: only for a model that takes a hint.
    let selection = files
        .transcription
        .clone()
        .unwrap_or_else(|| dictation.clone());
    if let Some(languages) = super::super::history::language_choices(selection.model) {
        let follows = selection.language.clone();
        let mut items = vec![choice(
            same,
            Some(language_name(&follows)),
            files.language.is_none(),
            |ui: &mut Ui| ui.update(|settings| settings.files.language = None),
        )];
        for code in languages {
            let chosen = files.language.as_deref() == Some(code);
            items.push(choice(
                language_name(code),
                None,
                chosen,
                move |ui: &mut Ui| {
                    ui.update(|settings| settings.files.language = Some(code.into()))
                },
            ));
        }
        let (value, resolved) = match &files.language {
            Some(code) => (language_name(code).to_string(), None),
            None => (same.to_string(), Some(language_name(&follows).to_string())),
        };
        transcription_rows.push(icon_row(
            p,
            "files-language",
            &icons::GLOBE,
            tr("Language"),
            Some(tr("For models that take a language hint")),
            inline_menu(ui, p, "files-language", &value, resolved, items, layer),
        ));
    }
    transcription_rows.push(icon_toggle_row(
        p,
        "files-vocabulary",
        &icons::BOOK,
        tr("Use vocabulary"),
        tr("Your Vocabulary helps the model hear names and terms, and cleanup spell them"),
        files.use_vocabulary,
        |ui: &mut Ui| {
            let on = !ui.settings.files.use_vocabulary;
            ui.update(|settings| settings.files.use_vocabulary = on);
        },
    ));
    transcription_rows.push(icon_toggle_row(
        p,
        "files-timestamps",
        &icons::CLOCK,
        tr("Show timestamps"),
        tr("How the Original tab opens. Switch to plain text there any time"),
        files.show_timestamps,
        |ui: &mut Ui| {
            let on = !ui.settings.files.show_timestamps;
            ui.files.timestamps = None;
            ui.update(|settings| settings.files.show_timestamps = on);
        },
    ));
    let transcription = section(
        p,
        "files-transcription",
        tr("Transcription"),
        transcription_rows,
    );

    let config = crate::files::worker::Config::from_settings(&ui.settings);
    let mut cleanup_rows = vec![
        icon_toggle_row(
            p,
            "files-cleanup-after",
            &icons::SPARKLES,
            tr("Clean up after transcription"),
            tr(
                "Adds a Cleanup tab as soon as a file is transcribed. Off: a Clean up button instead",
            ),
            files.cleanup_after,
            |ui: &mut Ui| {
                let on = !ui.settings.files.cleanup_after;
                ui.update(|settings| settings.files.cleanup_after = on);
            },
        ),
        icon_row(
            p,
            "files-engine",
            &icons::MONITOR,
            tr("Runs on"),
            Some(tr(
                "Built in picks its window by the text: 8K like dictation when it fits, 32K only for long files",
            )),
            engine_select(
                ui,
                p,
                "files-engine",
                files.cleanup_engine.as_deref(),
                tr("Same as dictation"),
                engine_label(&crate::files::rewrite::Engine::resolve(
                    None,
                    &config.server,
                    config.server_on,
                    &config.mode,
                )),
                |files, value| files.cleanup_engine = value,
                layer,
            ),
        ),
    ];
    let prompt_preview = files.use_default_prompt.then(|| {
        let prompt = ui
            .settings
            .dictation_processing
            .default_mode
            .post_processing
            .prompt
            .trim()
            .replace('\n', " ");
        text(format!("“{prompt}”"))
            .key("prompt-preview")
            .font(rune::TextStyle {
                italic: true,
                ..DESCRIPTION
            })
            .color(p.faint())
            .truncate()
    });
    cleanup_rows.push(
        col()
            .key("files-default-prompt-block")
            .child(icon_toggle_row(
                p,
                "files-default-prompt",
                &icons::MESSAGE,
                tr("Use the Default mode's prompt"),
                tr("The same instructions dictation uses (Output › Default). Off: write one just for files"),
                files.use_default_prompt,
                |ui: &mut Ui| {
                    let on = !ui.settings.files.use_default_prompt;
                    ui.update(|settings| settings.files.use_default_prompt = on);
                },
            ))
            .children(prompt_preview.map(|preview| {
                row()
                    .key("preview-row")
                    .pl(METRICS.row_px + 32.0)
                    .pr(METRICS.row_px)
                    .pb(12.0)
                    .mt(-6.0)
                    .child(preview.shrink())
            })),
    );
    if !files.use_default_prompt {
        cleanup_rows.push(prompt_row(ui, p));
    }
    let cleanup = section(p, "files-cleanup", tr("Cleanup"), cleanup_rows);

    let mut request_rows = vec![icon_row(
        p,
        "files-request-engine",
        &icons::MONITOR,
        tr("Runs on"),
        Some(tr(
            "Summary, Action items and your own requests. A model with a large window reads a long file whole, without 3 steps",
        )),
        engine_select(
            ui,
            p,
            "files-request-engine",
            files.request_engine.as_deref(),
            tr("Same as cleanup"),
            engine_label(&config.engine(None)),
            |files, value| files.request_engine = value,
            layer,
        ),
    )];
    if config.request_engine(None, None).effort().is_some() {
        request_rows.push(effort_row(ui, p));
    }
    let requests = section(p, "files-requests", tr("Requests"), request_rows);
    vec![transcription, cleanup, requests, quick_section(ui, p)]
}

/// Codex's reasoning for requests, while they run on Codex. A session's
/// field can still choose its own.
fn effort_row(ui: &Ui, p: Paint) -> El {
    const EFFORTS: [&str; 3] = ["low", "medium", "high"];
    let saved = match ui.settings.files.request_effort.trim() {
        "" => "medium",
        effort => effort,
    };
    let selected = EFFORTS.iter().position(|e| *e == saved).unwrap_or(1);
    icon_row(
        p,
        "files-request-effort",
        &icons::SPARKLES,
        tr("Reasoning"),
        Some(tr(
            "Low answers soonest. Higher levels think longer before they answer",
        )),
        segmented(
            p,
            "files-request-effort-picker",
            &[tr("Low"), tr("Medium"), tr("High")],
            selected,
            |ui: &mut Ui, index| {
                let effort = EFFORTS[index.min(EFFORTS.len() - 1)];
                if ui.settings.files.request_effort != effort {
                    ui.update(|settings| settings.files.request_effort = effort.into());
                }
            },
        ),
    )
}

/// Where cleanup or requests run: `inherit` (saved as none, resolving to
/// `inherited`), then each engine.
#[allow(clippy::too_many_arguments)]
fn engine_select(
    ui: &Ui,
    p: Paint,
    key: &'static str,
    current: Option<&str>,
    inherit: &str,
    inherited: String,
    set: fn(&mut crate::app_settings::FilesSettings, Option<String>),
    layer: &mut Vec<El>,
) -> El {
    let engines = engine_choices(ui);
    let mut items = vec![choice(
        inherit,
        Some(&inherited),
        current.is_none(),
        move |ui: &mut Ui| ui.update(|settings| set(&mut settings.files, None)),
    )];
    for engine in &engines {
        let value = engine.value.clone();
        let label = if engine.local {
            tr_fill("Built in, {}", &engine.label)
        } else {
            engine.label.clone()
        };
        items.push(choice(
            &label,
            None,
            current == Some(engine.value.as_str()),
            move |ui: &mut Ui| {
                let value = value.clone();
                ui.update(|settings| set(&mut settings.files, Some(value)));
            },
        ));
    }
    let (value, resolved) = match current {
        None => (inherit.to_string(), Some(inherited)),
        Some(value) => (
            engines
                .iter()
                .find(|engine| engine.value == value)
                .map_or_else(
                    || value.to_string(),
                    |engine| {
                        if engine.local {
                            tr_fill("Built in, {}", &engine.label)
                        } else {
                            engine.label.clone()
                        }
                    },
                ),
            None,
        ),
    };
    inline_menu(ui, p, key, &value, resolved, items, layer)
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

/// A quick request being written: a new one, or the one at `index`.
#[derive(Clone, Debug, Default)]
pub(in super::super) struct QuickEdit {
    pub(in super::super) index: Option<usize>,
    pub(in super::super) label: TextEdit,
    pub(in super::super) prompt: TextEdit,
}

fn quick_slot(ui: &mut Ui) -> &mut QuickEdit {
    ui.files.quick.get_or_insert_with(QuickEdit::default)
}

/// The row a quick request is written in, with Save and Cancel.
fn quick_editor(p: Paint, edit: &QuickEdit) -> El {
    let ready = !edit.label.text.trim().is_empty() && !edit.prompt.text.trim().is_empty();
    row()
        .key("quick-editor")
        .px(METRICS.row_px)
        .py(8.0)
        .gap(8.0)
        .center_y()
        .child(
            TextInput::new(&edit.label)
                .placeholder(tr("Name"))
                .width(Some(120.0))
                .on_edit(|ui: &mut Ui| &mut quick_slot(ui).label)
                .build(p.k)
                .key("label"),
        )
        .child(
            TextInput::new(&edit.prompt)
                .placeholder(tr("What it asks for"))
                .width(None)
                .on_edit(|ui: &mut Ui| &mut quick_slot(ui).prompt)
                .on_submit(|ui: &mut Ui, _| save_quick(ui))
                .build(p.k)
                .key("prompt")
                .grow(1.0)
                .shrink(),
        )
        .child(super::actions::quiet_button(
            p,
            "quick-save",
            None,
            tr("Save"),
            None,
            !ready,
            |ui: &mut Ui| save_quick(ui),
        ))
        .child(
            rune_kit::Button::new(tr("Cancel"))
                .variant(rune_kit::ButtonVariant::Ghost)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.files.quick = None)
                .build(p.k)
                .key("quick-cancel"),
        )
}

/// Saves the quick request being written, in its place or at the end.
fn save_quick(ui: &mut Ui) {
    let Some(edit) = ui.files.quick.take() else {
        return;
    };
    let label = edit.label.text.trim().to_string();
    let prompt = edit.prompt.text.trim().to_string();
    if label.is_empty() || prompt.is_empty() {
        ui.files.quick = Some(edit);
        return;
    }
    ui.update(|settings| {
        let chips = &mut settings.files.quick_prompts;
        let chip = QuickPrompt { label, prompt };
        match edit.index.filter(|index| *index < chips.len()) {
            Some(index) => chips[index] = chip,
            None => chips.push(chip),
        }
    });
}

/// The chips above the request field, as a list: each name and what it
/// asks, with edit and delete; Add request writes a new one.
fn quick_section(ui: &Ui, p: Paint) -> El {
    let editing = ui.files.quick.as_ref();
    let mut rows = Vec::new();
    for (index, chip) in ui.settings.files.quick_prompts.iter().enumerate() {
        if let Some(edit) = editing.filter(|edit| edit.index == Some(index)) {
            rows.push(quick_editor(p, edit));
            continue;
        }
        let (label, prompt) = (chip.label.clone(), chip.prompt.clone());
        rows.push(
            row()
                .key(format!("quick{index}"))
                .px(METRICS.row_px)
                .py(8.0)
                .gap(12.0)
                .center_y()
                .child(
                    text(chip.label.clone())
                        .key("label")
                        .font(p.k.ty.ui_medium)
                        .color(p.text())
                        .w(110.0)
                        .truncate(),
                )
                .child(
                    text(chip.prompt.clone())
                        .key("prompt")
                        .font(DESCRIPTION)
                        .color(p.muted())
                        .grow(1.0)
                        .shrink()
                        .truncate(),
                )
                .child(
                    IconButton::new(ld::EDIT_ICON)
                        .variant(IconButtonVariant::Ghost)
                        .control_size(rune_kit::ControlSize::Small)
                        .label(tr("Edit"))
                        .on_press(move |ui: &mut Ui| {
                            ui.files.quick = Some(QuickEdit {
                                index: Some(index),
                                label: TextEdit::new(label.clone()),
                                prompt: TextEdit::new(prompt.clone()),
                            })
                        })
                        .build(p.k)
                        .key("edit"),
                )
                .child(
                    IconButton::new(ld::TRASH_ICON)
                        .variant(IconButtonVariant::Ghost)
                        .control_size(rune_kit::ControlSize::Small)
                        .label(tr("Remove"))
                        .on_press(move |ui: &mut Ui| {
                            ui.files.quick = None;
                            ui.update(|settings| {
                                if index < settings.files.quick_prompts.len() {
                                    settings.files.quick_prompts.remove(index);
                                }
                            });
                        })
                        .build(p.k)
                        .key("remove"),
                ),
        );
    }
    if let Some(edit) = editing.filter(|edit| edit.index.is_none()) {
        rows.push(quick_editor(p, edit));
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
    let section = section_with(
        p,
        "files-quick",
        tr("Quick requests"),
        super::actions::quiet_button(
            p,
            "quick-add",
            None,
            tr("Add request"),
            None,
            false,
            |ui: &mut Ui| ui.files.quick = Some(QuickEdit::default()),
        ),
        rows,
    );
    col().key("files-quick-block").gap(10.0).child(section).child(
        text(tr(
            "The chips above the request field follow this list. Recent requests are kept on their own, last ten.",
        ))
        .key("quick-note")
        .font(DESCRIPTION)
        .color(p.muted())
        .wrap()
        .px(METRICS.row_px),
    )
}
