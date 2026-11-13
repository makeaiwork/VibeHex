//! What Assistant knows about this install: one typed value, [`Facts`],
//! made in one place from the settings and what was read off this computer
//! ([`State`], by [`gather`]). Every fact is an effective value, as the
//! window and the runtime resolve it, never a raw key: who writes meeting
//! notes is `meeting::generator`'s decision, a model is downloaded when
//! `transcription::models::is_installed` says so, a shortcut reads as its
//! keycaps do. What couldn't be told is [`Fact::Unknown`], never an empty
//! list or a "no".
//!
//! [`text`] is the one formatter that turns the facts into what a model
//! reads (`<state>` and the lines beside it), and into the record a turn
//! keeps of what it was told ([`text::knew`]).
//!
//! Privacy: the facts hold no secret, address, path, microphone, calendar,
//! or app list. Whether an OpenRouter key is saved and which local servers
//! are installed are for the app's own rules (setup) and are never written
//! for a model. Everything a model reads of them goes only while Assistant
//! › Settings › Your current settings is on, except what this computer
//! can't use, which names no setting of the person's.

mod gather;
pub mod text;
mod vocabulary;

#[cfg(test)]
mod tests;

use serde_json::Value;

use crate::app_settings::{AppSettings, MeetingSummaries};
use crate::assistant::changes::{catalog, names, parts};
use crate::assistant::guide;
use crate::assistant::setup::{Ask, past};
use crate::control::settings;
use crate::desktop::hardware::{Hardware, Need};
use crate::dictation::cleanup_server::{self, Provider};
use crate::files::rewrite::Engine;
use crate::transcription::models;

#[cfg(test)]
pub use gather::on_disk;
pub use gather::{Scene, Seen, gather};
pub use vocabulary::{Reads, Vocabulary};

/// One thing read off this install: its value, or that it couldn't be told.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Fact<T> {
    Known(T),
    #[default]
    Unknown,
}

impl<T> Fact<T> {
    /// The value, when it is known.
    pub fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Unknown => None,
        }
    }

    /// Whether it is known to be `value`.
    pub fn is(&self, value: &T) -> bool
    where
        T: PartialEq,
    {
        self.known() == Some(value)
    }
}

impl<T> From<Option<T>> for Fact<T> {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Unknown, Self::Known)
    }
}

/// What was read off this install besides its settings, when a question is
/// sent or setup moves on ([`gather`]); a scene's or a test's own.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub os: &'static str,
    /// The speech model ids downloaded.
    pub installed: Fact<Vec<String>>,
    pub codex_signed_in: Fact<bool>,
    pub opencode_running: Fact<bool>,
    /// Whether an OpenRouter key is saved. Only the app reads it: no model
    /// is told.
    pub openrouter_key: Fact<bool>,
    /// The local servers installed (LM Studio, Ollama), for setup to name.
    /// Only the app reads it.
    pub servers: Fact<Vec<&'static str>>,
    /// What earlier setups offered and the person declined: not offered
    /// again while the setting is as it was (`setup::past`).
    pub declined: Vec<past::Line>,
    /// The follow-ups answered in an earlier setup and not asked again
    /// (`setup::past::answered_before`), as answered then.
    pub answered: Vec<Ask>,
    /// The screens and pointing devices, for Labs features that need them;
    /// each part `None` while unknown.
    pub hardware: Hardware,
    /// The system's first preferred language (`es`, `pt-BR`), for setup's
    /// language question when recognition is on Auto; empty in a scene,
    /// which then follows the window.
    pub system_language: String,
}

impl State {
    /// The local servers known to be installed; none while unknown.
    pub fn servers(&self) -> &[&'static str] {
        self.servers.known().map_or(&[], Vec::as_slice)
    }
}

/// What kind of engine runs something.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Codex,
    /// A model on this computer, on the bundled engine.
    BuiltIn,
    OpenRouter,
    /// The person's own server.
    Server,
    OpenCode,
}

impl Kind {
    /// The name the window gives it, in English.
    pub fn name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::BuiltIn => "Built in",
            Self::OpenRouter => "OpenRouter",
            Self::Server => "Your server",
            Self::OpenCode => "OpenCode",
        }
    }
}

/// The engine something runs on, resolved as the runtime resolves it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Runs {
    /// As a settings key stores it: `codex:gpt-6.1-terra`, `local:<id>`,
    /// `openrouter`, `custom`, `opencode`.
    pub id: String,
    pub kind: Kind,
    /// Its model as the window names it: a built-in or Codex model's name;
    /// for OpenRouter and the person's server, what they typed (shown to
    /// them, never written for a model); empty when it has none.
    pub model: String,
}

impl Runs {
    /// The engine `engine` is.
    pub fn of(engine: &Engine) -> Self {
        match engine {
            Engine::OpenCode(_) => Self {
                id: "opencode".into(),
                kind: Kind::OpenCode,
                model: String::new(),
            },
            Engine::Server(server) => match server.provider {
                Provider::Local => Self {
                    id: format!("local:{}", server.model),
                    kind: Kind::BuiltIn,
                    model: cleanup_server::local_models::chosen(&server.model)
                        .name
                        .into(),
                },
                Provider::Codex => Self {
                    id: match server.model.trim() {
                        "" => "codex".into(),
                        model => format!("codex:{model}"),
                    },
                    kind: Kind::Codex,
                    model: cleanup_server::codex::model_name(&server.model),
                },
                Provider::OpenRouter => Self {
                    id: "openrouter".into(),
                    kind: Kind::OpenRouter,
                    model: server.model.clone(),
                },
                Provider::Custom => Self {
                    id: "custom".into(),
                    kind: Kind::Server,
                    model: server.model.clone(),
                },
            },
        }
    }

    /// The name a model reads: `Codex (GPT-6.1 Terra)`, `Built in (Qwen3
    /// 4B)`, `OpenRouter`. Never what the person typed as a model or an
    /// address.
    pub fn name(&self) -> String {
        match self.kind {
            Kind::Codex | Kind::BuiltIn => names::engine_with(self.kind.name(), &self.model),
            _ => self.kind.name().to_owned(),
        }
    }
}

/// A speech model: its id and the name the picker shows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Model {
    pub id: String,
    pub name: String,
}

impl Model {
    fn of(id: &str) -> Self {
        let name = models::all_models()
            .iter()
            .find(|model| model.id.as_str() == id)
            .map_or(id, |model| model.name);
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
        }
    }
}

/// Speech recognition: the model in use, its language, and what is on disk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Speech {
    pub model: Model,
    /// The recognition language as saved: `ru`, `en`, `auto`.
    pub language: String,
    /// The models downloaded, as the picker lists them.
    pub downloaded: Fact<Vec<Model>>,
}

/// Dictation's AI cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cleanup {
    /// Clean up with AI is off in the Global mode.
    Off,
    On(Runs),
}

/// What Summaries falls back to while no notes writer is chosen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Fallback {
    AppleIntelligence,
    OpenCode,
    Off,
}

impl Fallback {
    /// As the notes line names it for a model.
    pub fn name(self) -> &'static str {
        match self {
            Self::AppleIntelligence => "On this Mac (Apple Intelligence)",
            Self::OpenCode => "OpenCode",
            Self::Off => "Off",
        }
    }
}

/// Who writes meeting notes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notes {
    /// Meetings is on.
    pub meetings: bool,
    /// The engine Meetings › Settings › Notes › Written by chose, when it
    /// is in use (`meeting::generator`): any but OpenCode needs Labs › AI
    /// cleanup server on.
    pub writer: Option<Runs>,
    /// `meetings.summaries`: decides only while no writer is in use.
    pub fallback: Fallback,
}

/// Where Files cleans a text up.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilesCleanup {
    /// Labs › Files is off.
    Off,
    /// Files has no engine of its own: dictation's.
    AsDictation(Runs),
    Own(Runs),
}

/// A shortcut that is set, as the window's keycaps show it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shortcut {
    pub key: &'static str,
    /// What it is called in the window: `Dictation shortcut`.
    pub name: String,
    /// `⌥⇧A` on macOS, `Ctrl+Alt+Space` elsewhere.
    pub keys: String,
}

/// A Labs feature this computer can't use, and what it needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unfit {
    pub title: String,
    pub needs: Vec<Need>,
}

/// What the person lets Assistant read.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Consent {
    /// `assistant.send_settings`: a model may read the settings' values.
    pub send_settings: bool,
    /// `assistant.read_recordings`: `None` until first asked.
    pub recordings: Option<bool>,
}

/// Every fact about this install, resolved. See the module's words.
#[derive(Clone, Debug)]
pub struct Facts {
    /// What was read off this install: its OS, the models on disk, Codex's
    /// sign-in, whether OpenCode runs, the hardware, what earlier setups
    /// answered and declined; and, for the app's rules only and never
    /// written for a model, whether an OpenRouter key is saved and the
    /// local servers installed.
    pub state: State,
    pub speech: Speech,
    pub cleanup: Cleanup,
    pub notes: Notes,
    pub files: FilesCleanup,
    /// Where Assistant answers when a conversation picks nothing else.
    pub assistant: Runs,
    /// The shortcuts set, of features that are on.
    pub shortcuts: Vec<Shortcut>,
    /// The Labs features on, by title.
    pub labs_on: Vec<String>,
    /// How many terms Vocabulary holds, and who reads how many of them.
    pub vocabulary: Vocabulary,
    /// The Labs features this computer is known not to fit.
    pub unfit: Vec<Unfit>,
    pub consent: Consent,
}

/// The engine Assistant asks: `engine` and `effort` (a conversation's own
/// choice), else Assistant's settings, else what dictation's AI cleanup
/// uses.
pub fn assistant_engine(
    settings: &AppSettings,
    engine: Option<&str>,
    effort: Option<&str>,
) -> Engine {
    let effort = effort.filter(|effort| !effort.trim().is_empty()).unwrap_or(
        match settings.assistant.effort.trim() {
            "" => "medium",
            effort => effort,
        },
    );
    let processing = &settings.dictation_processing;
    Engine::resolve(
        engine.or(settings.assistant.engine.as_deref()),
        &processing.server,
        settings.labs.ai_cleanup_server,
        &processing.default_mode,
    )
    .with_effort(effort)
}

/// The engine dictation's AI cleanup runs on, as `dictation::cleanup_server`
/// and the OpenCode path decide it.
fn dictation_engine(settings: &AppSettings) -> Engine {
    let processing = &settings.dictation_processing;
    Engine::resolve(
        None,
        &processing.server,
        settings.labs.ai_cleanup_server,
        &processing.default_mode,
    )
}

/// Dictation's AI cleanup for `settings`: off, or the engine it runs on.
pub fn cleanup(settings: &AppSettings) -> Cleanup {
    match parts::Part::Cleanup.value(settings) {
        Value::Bool(true) => Cleanup::On(Runs::of(&dictation_engine(settings))),
        _ => Cleanup::Off,
    }
}

/// Who writes meeting notes for `settings`, as `meeting::generator`
/// decides it.
pub fn notes(settings: &AppSettings) -> Notes {
    Notes {
        meetings: settings.meetings.enabled,
        writer: crate::meeting::generator::NotesConfig::from_settings(settings)
            .chosen()
            .map(|engine| Runs::of(&engine)),
        fallback: match settings.meetings.summaries {
            MeetingSummaries::Mac => Fallback::AppleIntelligence,
            MeetingSummaries::OpenCode => Fallback::OpenCode,
            MeetingSummaries::Off => Fallback::Off,
        },
    }
}

/// Where Files cleans a text up for `settings`, as `files::worker` decides
/// it.
pub fn files(settings: &AppSettings) -> FilesCleanup {
    if !settings.labs.files {
        return FilesCleanup::Off;
    }
    let own = settings
        .files
        .cleanup_engine
        .as_deref()
        .is_some_and(|engine| !engine.trim().is_empty());
    match own {
        true => FilesCleanup::Own(Runs::of(
            &crate::files::worker::Config::from_settings(settings).cleanup_engine(None),
        )),
        false => FilesCleanup::AsDictation(Runs::of(&dictation_engine(settings))),
    }
}

impl Facts {
    /// The facts for `settings` on the install `state` describes.
    pub fn of(settings: &AppSettings, state: &State) -> Self {
        Self {
            state: state.clone(),
            speech: Speech {
                model: Model::of(settings.transcription.model.as_str()),
                language: settings.transcription.language.clone(),
                downloaded: state
                    .installed
                    .known()
                    .map(|ids| ids.iter().map(|id| Model::of(id)).collect())
                    .into(),
            },
            cleanup: cleanup(settings),
            notes: notes(settings),
            files: files(settings),
            assistant: Runs::of(&assistant_engine(settings, None, None)),
            shortcuts: shortcuts(settings),
            labs_on: labs_on(settings),
            vocabulary: Vocabulary::of(settings),
            unfit: unfit(&state.hardware),
            consent: Consent {
                send_settings: settings.assistant.send_settings,
                recordings: settings.assistant.read_recordings,
            },
        }
    }
}

/// A shortcut as the window's keycaps show it: `⌥⇧A` on macOS,
/// `Ctrl+Alt+Space` elsewhere; `None` when it is off.
pub fn keycaps(stored: &Value, key: &str) -> Option<String> {
    let binding = stored
        .pointer(&format!("/{}", key.replace('.', "/")))
        .cloned()
        .and_then(|raw| {
            serde_json::from_value::<Option<crate::app_settings::HotkeyBinding>>(raw).ok()
        })
        .flatten()?;
    let caps = binding.keycaps();
    let glyphs = caps.iter().all(|cap| cap.chars().count() == 1);
    Some(caps.join(if glyphs { "" } else { "+" }))
}

/// Every shortcut that is set, on this system, whose feature is on.
fn shortcuts(settings: &AppSettings) -> Vec<Shortcut> {
    let Ok(stored) = serde_json::to_value(settings) else {
        return Vec::new();
    };
    settings::kinds()
        .filter(|(key, kind, ..)| {
            *kind == "shortcut" && !catalog::never(key) && settings::unavailable(key).is_none()
        })
        .filter(|(key, ..)| {
            settings::requires(key, &Value::Bool(true))
                .iter()
                .all(|(needs, value)| {
                    crate::assistant::changes::current(settings, needs).as_ref() == Some(value)
                })
        })
        .filter_map(|(key, ..)| {
            Some(Shortcut {
                key,
                name: shortcut_name(key, settings)?,
                keys: keycaps(&stored, key)?,
            })
        })
        .collect()
}

/// What a shortcut is for, from where it lives in the window: `Paste last
/// dictation`, `Dictation` for General › Shortcut, `Screenshots: Record
/// shortcut` for a Labs feature's.
fn shortcut_name(key: &str, settings: &AppSettings) -> Option<String> {
    let place = names::place(key, settings)?;
    if key == "dictation_hotkey" {
        return Some("Dictation".into());
    }
    let last = place.path.rsplit(" › ").next().unwrap_or(place.path);
    Some(if last == "Shortcuts" || last == place.name {
        place.name
    } else {
        format!("{last}: {}", place.name)
    })
}

/// The Labs features whose switch is on, by the title their guide article
/// and their row give them.
fn labs_on(settings: &AppSettings) -> Vec<String> {
    crate::labs::Feature::ALL
        .into_iter()
        .filter(|feature| feature.on(settings))
        .map(|feature| {
            names::place(feature.settings_key(), settings)
                .map_or_else(|| feature.name().to_owned(), |place| place.name)
        })
        .collect()
}

/// The Labs features `hardware` is known not to fit.
fn unfit(hardware: &Hardware) -> Vec<Unfit> {
    guide::articles()
        .iter()
        .filter(|article| article.is_lab() && !article.fits(hardware))
        .map(|article| Unfit {
            title: article.title.clone(),
            needs: article
                .needs
                .iter()
                .copied()
                .filter(|need| hardware.has(*need) == Some(false))
                .collect(),
        })
        .collect()
}
