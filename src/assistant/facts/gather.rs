//! Reads what the facts need off this install: the speech models on disk,
//! the keychain (whether a key is there, never the key), the Applications
//! folder, the screens and pointing devices, and what earlier setups
//! answered and declined. Called when a question is sent or setup moves
//! on, never per frame, and on the UI thread: `NSScreen` answers only
//! there (`desktop::hardware`), so off it the screens stay unknown.
//!
//! The window passes only what lives in its own state ([`Seen`]): the
//! settings with what is being typed, the setups its store keeps, what
//! Output and the pane last heard of Codex and OpenCode (neither is asked
//! here: Codex is never started to look), and a scene's stand-ins.

use super::{Fact, State};
use crate::app_settings::AppSettings;
use crate::assistant::setup::{engines, past, since};
use crate::desktop::hardware::{self, Hardware};
use crate::dictation::cleanup_server::{Provider, keys};
use crate::transcription::models as transcription_models;

/// What only the window knows.
pub struct Seen<'a> {
    /// The settings, with every text field's typing.
    pub settings: &'a AppSettings,
    /// The setups kept with the conversations.
    pub setups: &'a [past::Setup],
    /// Whether Codex is signed in, as Output › AI cleanup last read it.
    pub codex_signed_in: Option<bool>,
    /// Whether OpenCode runs, as the pane last asked.
    pub opencode_running: Option<bool>,
    /// A scene: nothing of this computer is read but, with no list of its
    /// own, the models on disk.
    pub scene: Option<Scene>,
}

/// A scene's stand-ins for this computer: it plays a Mac.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    /// The speech models its picker shows as downloaded, if it says.
    pub installed: Option<Vec<String>>,
    /// Its screens and pointing devices; unknown by default.
    pub hardware: Hardware,
}

/// The ids of the `models` this computer can run that `has` finds
/// downloaded.
pub fn installed(
    models: &[&transcription_models::ModelDefinition],
    has: impl Fn(&transcription_models::ModelDefinition) -> bool,
) -> Vec<String> {
    models
        .iter()
        .filter(|model| model.available() && has(model))
        .map(|model| model.id.as_str().to_owned())
        .collect()
}

/// The speech models on disk, each asked for in the language it is saved
/// with, as the picker asks.
pub fn on_disk(settings: &AppSettings) -> Vec<String> {
    installed(&transcription_models::all_models(), |model| {
        let language = settings.transcription_for_model(model.id).language;
        transcription_models::is_installed(model, &language)
    })
}

/// This install's state now.
pub fn gather(seen: Seen) -> State {
    let settings = seen.settings;
    let answered = past::answered_before(seen.setups, settings, since::changed_since);
    let declined = past::declined(seen.setups);
    let codex_signed_in = seen.codex_signed_in.into();
    let opencode_running = seen.opencode_running.into();
    match seen.scene {
        Some(scene) => State {
            os: "macos",
            // In the picker's order, whatever order the scene names them in.
            installed: Fact::Known(match &scene.installed {
                Some(ids) => installed(&transcription_models::all_models(), |model| {
                    ids.iter().any(|id| id == model.id.as_str())
                }),
                None => on_disk(settings),
            }),
            codex_signed_in,
            opencode_running,
            // The keychain and the Applications folder: never in a scene.
            openrouter_key: Fact::Unknown,
            servers: Fact::Unknown,
            declined,
            answered,
            hardware: scene.hardware,
        },
        None => State {
            os: std::env::consts::OS,
            installed: Fact::Known(on_disk(settings)),
            codex_signed_in,
            opencode_running,
            openrouter_key: Fact::Known(keys::has(Provider::OpenRouter)),
            servers: Fact::Known(engines::installed_servers()),
            declined,
            answered,
            hardware: hardware::detect(),
        },
    }
}
