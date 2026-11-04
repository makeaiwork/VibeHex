//! Labs › Clean up on your own server, in Output: whether every dictation is
//! rewritten, where (OpenCode, OpenRouter, or the person's own
//! OpenAI-compatible server such as LM Studio), with which model, key, and
//! instructions, and a Test that sends one short request. The model field
//! takes any id, and its list offers the models the server reports. Built in
//! (macOS) lists the models Hex downloads instead ([`super::local_cleanup`]).

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use rune::{El, TextEdit, Waker, col, row, text};
use rune_kit::{
    Combobox, ComboboxOption, ComboboxState, IconButton, IconButtonVariant, Placement, TextArea,
    TextInput,
};

use super::super::Ui;
use super::super::controls::{
    ButtonKind, CardRow, DESCRIPTION, METRICS, Paint, button, segmented, setting_row, toggle_row,
};
use crate::app_settings::{CleanupRunsOn, CleanupServerSettings};
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::cleanup_server::models::ServerModel;
use crate::dictation::cleanup_server::{self, LM_STUDIO_URL, OLLAMA_URL, Provider, Server};

/// Fields wide enough for a URL or a model id such as
/// `google/gemini-2.5-flash-lite`.
const FIELD: f32 = 260.0;
/// The model list's button and the gap before it.
const LIST_BUTTON: f32 = 36.0;

/// The section's drafts and the last Test.
#[derive(Default)]
pub(in crate::rune_ui) struct Cleanup {
    url: Option<TextEdit>,
    model: Option<TextEdit>,
    /// A key being typed; never filled from the saved one.
    key: TextEdit,
    prompt: Option<TextEdit>,
    test: Test,
    models: Models,
    /// The model list: open, its search, and its highlight.
    picker: ComboboxState,
    /// Whether the list was open at the last poll: opening it lists the
    /// server's models again, so one added since shows.
    picker_was_open: bool,
    /// The built-in models and downloads.
    pub(super) local: super::local_cleanup::LocalModels,
}

/// Which server a model list came from: a change loads it again.
type Source = (Provider, String);

/// The models the chosen server reports, loaded while Output shows and
/// again each time the list opens.
#[derive(Default)]
enum Models {
    #[default]
    Unloaded,
    /// The list shown meanwhile: the last one from the same server, if any.
    Loading(
        Source,
        Receiver<Result<Vec<ServerModel>, String>>,
        Vec<ServerModel>,
    ),
    Loaded(Source, Vec<ServerModel>),
    Failed(Source, String),
}

impl Models {
    fn source(&self) -> Option<&Source> {
        match self {
            Self::Unloaded => None,
            Self::Loading(source, ..) | Self::Loaded(source, _) | Self::Failed(source, _) => {
                Some(source)
            }
        }
    }

    /// The models to show for `from`: the loaded list, or the previous one
    /// while it loads again.
    fn listed(&self, from: &Source) -> Option<&[ServerModel]> {
        match self {
            Self::Loaded(source, models) if source == from => Some(models),
            Self::Loading(source, _, stale) if source == from && !stale.is_empty() => Some(stale),
            _ => None,
        }
    }
}

fn source(server: &Server) -> Source {
    (server.provider, server.url.trim().to_owned())
}

#[derive(Default)]
enum Test {
    #[default]
    NotTested,
    /// The server and model being tested, so a change discards the answer.
    Testing(Server, Receiver<Result<Duration, String>>),
    Passed(Server, Duration),
    Failed(Server, String),
}

/// The section, while Labs › Clean up on your own server is on.
pub(super) fn section(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Option<El> {
    if !ui.settings.labs.ai_cleanup_server {
        return None;
    }
    let default = &ui
        .settings
        .dictation_processing
        .default_mode
        .post_processing;
    let saved = &ui.settings.dictation_processing.server;
    let mut rows: Vec<CardRow> = vec![
        toggle_row(
            p,
            "ai-cleanup-enabled",
            tr("Clean up with AI"),
            tr("Every dictation is rewritten with your instructions and vocabulary"),
            default.enabled,
            |ui: &mut Ui| {
                ui.update(|settings| {
                    let rewrite = &mut settings.dictation_processing.default_mode.post_processing;
                    rewrite.enabled = !rewrite.enabled;
                });
            },
        )
        .into(),
        runs_on_row(p, saved.runs_on).into(),
    ];
    if let Some(server) = Server::from_settings(saved)
        && server.provider == Provider::Local
    {
        rows.push(status_row(ui, p, &server).into());
        rows.extend(super::local_cleanup::rows(ui, p));
    } else if let Some(server) = Server::from_settings(saved) {
        rows.push(status_row(ui, p, &server).into());
        if server.provider == Provider::Custom {
            rows.push(url_row(ui, p, saved).into());
        }
        rows.push(model_row(ui, p, saved, &server, layer).into());
        rows.push(key_row(ui, p, server.provider).into());
    }
    rows.push(instructions(ui, p).into());
    Some(super::labeled(
        p,
        "ai-cleanup",
        tr("AI cleanup"),
        &cleanup_server::Server::from_settings(saved)
            .map_or_else(|| "OpenCode".into(), |server| server.name()),
        rows,
    ))
}

fn runs_on_row(p: Paint, runs_on: CleanupRunsOn) -> El {
    let selected = match runs_on {
        CleanupRunsOn::Opencode => 0,
        CleanupRunsOn::Openrouter => 1,
        CleanupRunsOn::Custom => 2,
        CleanupRunsOn::Local => 3,
    };
    let description = match runs_on {
        CleanupRunsOn::Opencode => tr("The OpenCode service running on this computer"),
        CleanupRunsOn::Openrouter => tr("Hundreds of models with one API key"),
        CleanupRunsOn::Custom => tr("Ollama, LM Studio, or any OpenAI-compatible URL"),
        CleanupRunsOn::Local => tr("A model Hex downloads and runs itself, offline"),
    };
    // The built-in engine ships in the macOS app only.
    let mut labels = vec!["OpenCode", "OpenRouter", tr("Your own")];
    if cfg!(target_os = "macos") {
        labels.push(tr("Built in"));
    }
    setting_row(
        p,
        "ai-cleanup-runs-on",
        tr("Runs on"),
        Some(description),
        segmented(
            p,
            "ai-cleanup-runs-on-picker",
            &labels,
            selected,
            |ui: &mut Ui, index| {
                let runs_on = match index {
                    1 => CleanupRunsOn::Openrouter,
                    2 => CleanupRunsOn::Custom,
                    3 => CleanupRunsOn::Local,
                    _ => CleanupRunsOn::Opencode,
                };
                if ui.settings.dictation_processing.server.runs_on != runs_on {
                    ui.output.cleanup.url = None;
                    ui.output.cleanup.model = None;
                    ui.output.cleanup.key = TextEdit::default();
                    ui.update(|settings| settings.dictation_processing.server.runs_on = runs_on);
                }
            },
        ),
    )
}

/// The server's name, how the last Test went, and Test.
fn status_row(ui: &Ui, p: Paint, server: &Server) -> El {
    let host = crate::net::host(&server.url);
    let (status, color) = match &ui.output.cleanup.test {
        Test::Testing(tested, _) if tested == server => (tr("Testing…").to_owned(), p.muted()),
        Test::Passed(tested, took) if tested == server => (
            tr_fill("Connected · {} s", &format!("{:.1}", took.as_secs_f32())),
            p.success(),
        ),
        Test::Failed(tested, why) if tested == server => (why.clone(), p.danger()),
        _ if server.provider == Provider::Local => super::local_cleanup::engine_status(ui, p),
        _ => (format!("{} · {host}", tr("Not tested")), p.muted()),
    };
    let testing = matches!(&ui.output.cleanup.test, Test::Testing(tested, _) if tested == server);
    let untestable =
        server.provider == Provider::Local && !super::local_cleanup::chosen_installed(ui);
    let loading = (server.provider == Provider::Local)
        .then(|| super::local_cleanup::loading_bar(ui, p))
        .flatten();
    let control = if let Some(bar) = loading {
        bar
    } else if testing || untestable {
        text("").key("ai-cleanup-test")
    } else {
        button(
            p,
            "ai-cleanup-test",
            tr("Test"),
            ButtonKind::Secondary,
            |ui, _| test(ui),
        )
    };
    let title = if server.provider == Provider::Local {
        tr("Built in").to_owned()
    } else {
        server.name()
    };
    super::super::controls::described_row(
        p,
        "ai-cleanup-status",
        &title,
        Some((status, color)),
        control,
    )
}

fn url_row(ui: &Ui, p: Paint, saved: &CleanupServerSettings) -> El {
    let fill = |key: &str, label: &str, url: &'static str| {
        button(
            p,
            key,
            label,
            ButtonKind::Secondary,
            move |ui: &mut Ui, _| {
                ui.output.cleanup.url = None;
                if ui.settings.dictation_processing.server.url != url {
                    ui.update(|settings| settings.dictation_processing.server.url = url.into());
                }
            },
        )
    };
    let presets = row()
        .key("presets")
        .gap(4.0)
        .center_y()
        .child(
            text(tr("Fill in"))
                .key("label")
                .font(DESCRIPTION)
                .color(p.muted()),
        )
        .child(fill("ai-cleanup-lm-studio", "LM Studio", LM_STUDIO_URL))
        .child(fill("ai-cleanup-ollama", "Ollama", OLLAMA_URL));
    let field = draft_field(
        p,
        "ai-cleanup-url",
        FIELD,
        ui.output.cleanup.url.as_ref(),
        &saved.url,
        LM_STUDIO_URL,
        |ui| &mut ui.output.cleanup.url,
        |ui| ui.settings.dictation_processing.server.url.clone(),
        save_url,
    );
    col()
        .key("ai-cleanup-url-group")
        .child(setting_row(p, "ai-cleanup-url-row", "URL", None, field))
        .child(
            row()
                .key("ai-cleanup-presets")
                .px(METRICS.row_px)
                .pb(10.0)
                .child(presets),
        )
}

fn model_row(
    ui: &Ui,
    p: Paint,
    saved: &CleanupServerSettings,
    server: &Server,
    layer: &mut Vec<El>,
) -> El {
    let (value, placeholder) = match saved.runs_on {
        CleanupRunsOn::Openrouter => (&saved.openrouter_model, "google/gemini-2.5-flash-lite"),
        _ => (&saved.custom_model, "gemma-4-e4b-it-mlx"),
    };
    let field = draft_field(
        p,
        "ai-cleanup-model",
        FIELD - LIST_BUTTON,
        ui.output.cleanup.model.as_ref(),
        value,
        placeholder,
        |ui| &mut ui.output.cleanup.model,
        |ui| saved_model(&ui.settings.dictation_processing.server).to_owned(),
        save_model,
    );
    let description = ui
        .output
        .cleanup
        .models
        .listed(&source(server))
        .filter(|models| !models.is_empty())
        .map(|models| tr_fill("Type one, or choose from {}", &models.len().to_string()));
    let controls = row()
        .key("ai-cleanup-model-controls")
        .gap(8.0)
        .center_y()
        .child(field)
        .child(model_list(ui, p, server, value, layer));
    setting_row(
        p,
        "ai-cleanup-model-row",
        tr("Model"),
        description.as_deref(),
        controls,
    )
}

/// The models the server reports, searchable, beside the field; choosing
/// one saves it. While loading or after a failure the list says so, and a
/// failed list offers Try again.
fn model_list(ui: &Ui, p: Paint, server: &Server, saved: &str, layer: &mut Vec<El>) -> El {
    let current_source = source(server);
    let mut ids = Vec::new();
    let mut options = Vec::new();
    let mut empty = tr("No models match");
    let models = &ui.output.cleanup.models;
    match (models.listed(&current_source), models) {
        (Some(models), _) => {
            for model in models {
                let mut option = ComboboxOption::new(ids.len().to_string(), model.id.clone());
                option.detail = model.name.clone();
                options.push(option);
                ids.push(model.id.clone());
            }
            if models.is_empty() {
                empty = tr("The server lists no models");
            }
        }
        (None, Models::Failed(from, why)) if *from == current_source => {
            let mut option = ComboboxOption::new("retry", tr("Try again"));
            option.detail = why.clone();
            options.push(option);
        }
        _ => {
            let mut option = ComboboxOption::new("loading", tr("Loading models…"));
            option.disabled = true;
            options.push(option);
        }
    }
    let current = ids
        .iter()
        .position(|id| id == saved.trim())
        .map(|index| index.to_string());
    let trigger = IconButton::new(p.k.glyph.chevron_down)
        .variant(IconButtonVariant::Neutral)
        .control_size(rune_kit::ControlSize::Large)
        .label(tr("Choose a model"))
        .expanded(ui.output.cleanup.picker.open)
        .build(p.k)
        .key("ai-cleanup-models");
    Combobox::new(
        "ai-cleanup-model-list",
        &ui.output.cleanup.picker,
        &options,
        |ui: &mut Ui| &mut ui.output.cleanup.picker,
    )
    .trigger(trigger)
    .placement(Placement::BottomEnd)
    // As wide as the field and its button above it: a wider list is narrowed
    // to the window, and its rows (and the check) would overflow it.
    .width(FIELD + 8.0)
    .max_height(360.0)
    .current(current.as_deref())
    .search_placeholder(tr("Search models"))
    .empty_message(empty)
    .on_choose(move |ui: &mut Ui, id: &str, _| {
        if id == "retry" {
            ui.output.cleanup.models = Models::Unloaded;
            return;
        }
        if let Some(model) = id.parse::<usize>().ok().and_then(|index| ids.get(index)) {
            ui.output.cleanup.model = Some(TextEdit::new(model.clone()));
            save_model(ui);
        }
    })
    .build(p.k, layer)
}

/// The key field: what is typed is never shown, and a saved key is never
/// read back into it.
fn key_row(ui: &Ui, p: Paint, provider: Provider) -> El {
    // Read once from the credential store, then from memory.
    let saved = ui.live() && cleanup_server::keys::has(provider);
    let description = match (saved, provider) {
        (true, _) => tr("Saved outside settings.json"),
        (false, Provider::OpenRouter) => tr("Required"),
        (false, Provider::Custom) => tr("Not needed for LM Studio or Ollama"),
        (false, Provider::Local) => "",
    };
    let typed = !ui.output.cleanup.key.text.trim().is_empty();
    let input = TextInput::new(&ui.output.cleanup.key)
        .secure()
        .placeholder(if saved {
            "••••••••"
        } else {
            "sk-…"
        })
        .width(Some(FIELD - if typed || saved { 70.0 } else { 0.0 }))
        .on_edit(|ui: &mut Ui| &mut ui.output.cleanup.key)
        .on_submit(|ui: &mut Ui, _| save_key(ui))
        .build(p.k)
        .key("ai-cleanup-key");
    let action = if typed {
        Some(button(
            p,
            "ai-cleanup-key-save",
            tr("Save"),
            ButtonKind::Secondary,
            |ui, _| save_key(ui),
        ))
    } else if saved {
        Some(button(
            p,
            "ai-cleanup-key-remove",
            tr("Remove"),
            ButtonKind::Secondary,
            |ui, _| {
                ui.output.cleanup.key = TextEdit::default();
                save_key(ui);
            },
        ))
    } else {
        None
    };
    setting_row(
        p,
        "ai-cleanup-key-row",
        tr("API key"),
        Some(description),
        row()
            .key("controls")
            .gap(8.0)
            .center_y()
            .child(input)
            .children(action),
    )
}

/// Global's rewrite instructions, which every server and OpenCode receive
/// along with the vocabulary.
fn instructions(ui: &Ui, p: Paint) -> El {
    let saved = &ui
        .settings
        .dictation_processing
        .default_mode
        .post_processing
        .prompt;
    let fallback;
    let edit = match &ui.output.cleanup.prompt {
        Some(edit) => edit,
        None => {
            fallback = TextEdit::new(saved.clone());
            &fallback
        }
    };
    let edited = edit.text.trim() != saved.trim();
    let area = TextArea::new(edit)
        .placeholder(tr(
            "Your own rules, added to Hex's prompt along with your vocabulary",
        ))
        .rows(4)
        // Long instructions scroll inside the field instead of stretching
        // the pane.
        .max_height(280.0)
        .width(None)
        .on_edit(|ui: &mut Ui| {
            let saved = ui
                .settings
                .dictation_processing
                .default_mode
                .post_processing
                .prompt
                .clone();
            ui.output
                .cleanup
                .prompt
                .get_or_insert_with(|| TextEdit::new(saved))
        })
        .build(p.k)
        .key("ai-cleanup-prompt");
    let header = row()
        .key("header")
        .gap(8.0)
        .center_y()
        .child(
            col()
                .key("copy")
                .grow(1.0)
                .gap(2.0)
                .child(
                    text(tr("Custom instructions"))
                        .key("title")
                        .font(p.k.ty.ui)
                        .color(p.text()),
                )
                .child(
                    text(tr(
                        "Your own rules, added to Hex's prompt along with your vocabulary",
                    ))
                    .key("description")
                    .font(DESCRIPTION)
                    .color(p.muted())
                    .wrap(),
                ),
        )
        .children(edited.then(|| {
            button(
                p,
                "ai-cleanup-prompt-save",
                tr("Save"),
                ButtonKind::Secondary,
                |ui, _| save_prompt(ui),
            )
        }));
    col()
        .key("ai-cleanup-instructions")
        .px(METRICS.row_px)
        .py(12.0)
        .gap(10.0)
        .child(header)
        .child(area)
}

/// A one-line field over a saved text: Save shows while the draft differs.
#[allow(clippy::too_many_arguments)]
fn draft_field(
    p: Paint,
    key: &str,
    width: f32,
    draft: Option<&TextEdit>,
    saved: &str,
    placeholder: &str,
    slot: fn(&mut Ui) -> &mut Option<TextEdit>,
    saved_now: fn(&Ui) -> String,
    save: fn(&mut Ui),
) -> El {
    let fallback;
    let edit = match draft {
        Some(edit) => edit,
        None => {
            fallback = TextEdit::new(saved.to_owned());
            &fallback
        }
    };
    let edited = edit.text.trim() != saved.trim();
    let input = TextInput::new(edit)
        .placeholder(placeholder)
        .width(Some(width - if edited { 70.0 } else { 0.0 }))
        .on_edit(move |ui: &mut Ui| {
            let saved = saved_now(ui);
            slot(ui).get_or_insert_with(|| TextEdit::new(saved))
        })
        .on_submit(move |ui: &mut Ui, _| save(ui))
        .build(p.k)
        .key(key);
    row()
        .key("controls")
        .gap(8.0)
        .center_y()
        .child(input)
        .children(edited.then(|| {
            button(
                p,
                &format!("{key}-save"),
                tr("Save"),
                ButtonKind::Secondary,
                move |ui, _| save(ui),
            )
        }))
}

fn saved_model(server: &CleanupServerSettings) -> &str {
    match server.runs_on {
        CleanupRunsOn::Openrouter => &server.openrouter_model,
        _ => &server.custom_model,
    }
}

fn save_url(ui: &mut Ui) {
    let Some(edit) = ui.output.cleanup.url.take() else {
        return;
    };
    let url = edit.text.trim().to_owned();
    if !url.is_empty()
        && !url::Url::parse(&url).is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"))
    {
        ui.error = Some(tr("The URL must start with http:// or https://").into());
        ui.output.cleanup.url = Some(edit);
        return;
    }
    if url != ui.settings.dictation_processing.server.url {
        ui.update(|settings| settings.dictation_processing.server.url = url);
    }
}

fn save_model(ui: &mut Ui) {
    let Some(edit) = ui.output.cleanup.model.take() else {
        return;
    };
    let model = edit.text.trim().to_owned();
    if model != saved_model(&ui.settings.dictation_processing.server) {
        ui.update(
            |settings| match settings.dictation_processing.server.runs_on {
                CleanupRunsOn::Openrouter => {
                    settings.dictation_processing.server.openrouter_model = model
                }
                _ => settings.dictation_processing.server.custom_model = model,
            },
        );
    }
}

fn save_prompt(ui: &mut Ui) {
    let Some(edit) = ui.output.cleanup.prompt.take() else {
        return;
    };
    let prompt = edit.text.trim().to_owned();
    if prompt
        != ui
            .settings
            .dictation_processing
            .default_mode
            .post_processing
            .prompt
    {
        ui.update(|settings| {
            settings
                .dictation_processing
                .default_mode
                .post_processing
                .prompt = prompt;
        });
    }
}

/// Saves the typed key (an empty field removes the saved one) and clears
/// the field.
fn save_key(ui: &mut Ui) {
    let Some(server) = Server::from_settings(&ui.settings.dictation_processing.server) else {
        return;
    };
    let key = std::mem::take(&mut ui.output.cleanup.key).text.clone();
    if !ui.live() {
        return;
    }
    if let Err(error) = cleanup_server::keys::set(server.provider, &key) {
        ui.error = Some(error);
    }
    ui.output.cleanup.test = Test::NotTested;
    // A server may list more models with the key.
    ui.output.cleanup.models = Models::Unloaded;
}

/// Sends one short request on its own thread; [`poll`] takes the answer.
fn test(ui: &mut Ui) {
    let Some(server) = Server::from_settings(&ui.settings.dictation_processing.server) else {
        return;
    };
    if !ui.live() {
        return;
    }
    let (sender, receiver) = channel();
    let waker: Option<Waker> = ui.modes.waker.clone();
    let tested = server.clone();
    let started = std::thread::Builder::new()
        .name("hex-cleanup-test".into())
        .spawn(move || {
            let key = (tested.provider != Provider::Local)
                .then(|| cleanup_server::keys::get(tested.provider))
                .flatten();
            let _ = sender.send(
                cleanup_server::test(&tested, key.as_deref()).map_err(|error| error.to_string()),
            );
            if let Some(waker) = &waker {
                waker();
            }
        });
    if started.is_ok() {
        ui.output.cleanup.test = Test::Testing(server, receiver);
    }
}

/// Takes a finished Test's answer and model list, and loads the chosen
/// server's list while the window shows; true when it changed what shows.
pub(super) fn poll(ui: &mut Ui) -> bool {
    poll_models(ui) | poll_test(ui) | super::local_cleanup::poll(ui)
}

/// Whether something in the section moves on its own: a download, the
/// model loading, or a Test.
pub(super) fn busy(ui: &Ui) -> bool {
    ui.output.cleanup.local.busy() || matches!(ui.output.cleanup.test, Test::Testing(..))
}

fn poll_models(ui: &mut Ui) -> bool {
    let cleanup = &mut ui.output.cleanup;
    let opened = cleanup.picker.open && !cleanup.picker_was_open;
    cleanup.picker_was_open = cleanup.picker.open;
    if let Models::Loading(from, receiver, _) = &cleanup.models {
        let finished = match receiver.try_recv() {
            Ok(Ok(models)) => Models::Loaded(from.clone(), models),
            Ok(Err(why)) => Models::Failed(from.clone(), why),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Models::Unloaded,
        };
        cleanup.models = finished;
        return true;
    }
    if !ui.settings.labs.ai_cleanup_server
        || !ui.live()
        || !ui.window.as_ref().is_some_and(rune::AppHandle::is_visible)
    {
        return false;
    }
    let Some(server) = Server::from_settings(&ui.settings.dictation_processing.server)
        .filter(|server| server.provider != Provider::Local)
    else {
        return false;
    };
    let wanted = source(&server);
    // Opening the list asks again, so a model added on the server since shows.
    if ui.output.cleanup.models.source() == Some(&wanted) && !opened {
        return false;
    }
    let stale = match std::mem::take(&mut ui.output.cleanup.models) {
        Models::Loaded(from, models) if from == wanted => models,
        _ => Vec::new(),
    };
    let (sender, receiver) = channel();
    let waker: Option<Waker> = ui.modes.waker.clone();
    let started = std::thread::Builder::new()
        .name("hex-cleanup-models".into())
        .spawn(move || {
            let key = cleanup_server::keys::get(server.provider);
            let _ = sender.send(
                cleanup_server::models::list(&server, key.as_deref())
                    .map_err(|error| error.to_string()),
            );
            if let Some(waker) = &waker {
                waker();
            }
        });
    ui.output.cleanup.models = match started {
        Ok(_) => Models::Loading(wanted, receiver, stale),
        Err(error) => Models::Failed(wanted, error.to_string()),
    };
    true
}

fn poll_test(ui: &mut Ui) -> bool {
    let Test::Testing(server, receiver) = &ui.output.cleanup.test else {
        return false;
    };
    let finished = match receiver.try_recv() {
        Ok(Ok(took)) => Test::Passed(server.clone(), took),
        Ok(Err(why)) => Test::Failed(server.clone(), why),
        Err(TryRecvError::Empty) => return false,
        Err(TryRecvError::Disconnected) => Test::NotTested,
    };
    ui.output.cleanup.test = finished;
    true
}

/// The section with LM Studio chosen, its model set, and a Test passed.
pub(super) fn fixture(ui: &mut Ui) {
    ui.settings.labs.ai_cleanup_server = true;
    let server = &mut ui.settings.dictation_processing.server;
    server.runs_on = CleanupRunsOn::Custom;
    server.url = LM_STUDIO_URL.into();
    server.custom_model = "gemma-4-e4b-it-mlx".into();
    let rewrite = &mut ui
        .settings
        .dictation_processing
        .default_mode
        .post_processing;
    rewrite.enabled = true;
    rewrite.prompt = "English words spoken in Russian are written in English: \"клод код\" is \"Claude Code\", \"эм си пи сервер\" is \"MCP server\".".into();
    if let Some(server) = Server::from_settings(&ui.settings.dictation_processing.server) {
        let model = |id: &str| ServerModel {
            id: id.into(),
            name: String::new(),
        };
        ui.output.cleanup.models = Models::Loaded(
            source(&server),
            vec![
                model("gemma-4-e4b-it-mlx"),
                model("gemma-4-e4b-it-mlx@4bit"),
                model("google/gemma-4-e4b"),
            ],
        );
        ui.output.cleanup.test = Test::Passed(server, Duration::from_millis(820));
    }
}
