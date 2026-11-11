//! Narrow addresses inside two JSON keys a card may change: one app's
//! choice in `meetings.providers` (`meetings.providers.<app>.policy`), and
//! the Global mode's instructions (`dictation_processing.default_mode.prompt`)
//! and its Clean up with AI switch (`dictation_processing.default_mode.cleanup`).
//! A part is read from and written back into its whole key, which
//! `control::settings::check` then validates as it would any other value.

use serde_json::Value;

use crate::app_settings::{AppSettings, MeetingPolicy, MeetingProviderChoice};

/// The Global mode's AI cleanup instructions.
pub const PROMPT: &str = "dictation_processing.default_mode.prompt";
/// The Global mode's Clean up with AI switch.
pub const CLEANUP: &str = "dictation_processing.default_mode.cleanup";
const PROVIDERS: &str = "meetings.providers";
const DEFAULT_MODE: &str = "dictation_processing.default_mode";

/// A part of a JSON key.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Prompt,
    Cleanup,
    /// One call service's Ask / Always record / Never offer.
    Provider {
        key: String,
        label: String,
    },
}

/// The part `key` names, if it names one: an app by its provider key or
/// its name (`zoom`, `us.zoom.xos`, `Google Meet`).
pub fn part(key: &str, settings: &AppSettings) -> Option<Part> {
    if key == PROMPT {
        return Some(Part::Prompt);
    }
    if key == CLEANUP {
        return Some(Part::Cleanup);
    }
    let app = key
        .strip_prefix("meetings.providers.")?
        .strip_suffix(".policy")?;
    provider(app, settings).map(|(key, label)| Part::Provider { key, label })
}

/// Every call service Detected calls lists, then the ones chosen for.
fn providers(settings: &AppSettings) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> =
        crate::app_settings::known_meeting_providers(settings.labs.more_call_apps)
            .map(|(key, label)| (key.to_owned(), label.to_owned()))
            .collect();
    for choice in &settings.meetings.providers {
        if !all.iter().any(|(key, _)| *key == choice.key) {
            all.push((choice.key.clone(), choice.label.clone()));
        }
    }
    all
}

fn provider(app: &str, settings: &AppSettings) -> Option<(String, String)> {
    let squash = |text: &str| -> String {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let wanted = squash(app);
    if wanted.is_empty() {
        return None;
    }
    let all = providers(settings);
    all.iter()
        .find(|(key, label)| key.eq_ignore_ascii_case(app) || squash(label) == wanted)
        .or_else(|| {
            all.iter().find(|(key, _)| {
                key.split('.')
                    .any(|piece| piece.eq_ignore_ascii_case(app.trim()))
            })
        })
        .cloned()
}

impl Part {
    /// The whole key the part lives in.
    pub fn parent(&self) -> &'static str {
        match self {
            Self::Prompt | Self::Cleanup => DEFAULT_MODE,
            Self::Provider { .. } => PROVIDERS,
        }
    }

    /// Its value in `settings`: the instructions, or the app's policy as
    /// `ask`, `always_record`, or `never_offer`.
    pub fn value(&self, settings: &AppSettings) -> Value {
        match self {
            Self::Prompt => Value::String(
                settings
                    .dictation_processing
                    .default_mode
                    .post_processing
                    .prompt
                    .clone(),
            ),
            Self::Cleanup => Value::Bool(
                settings
                    .dictation_processing
                    .default_mode
                    .post_processing
                    .enabled,
            ),
            Self::Provider { key, .. } => {
                let policy = settings
                    .meetings
                    .providers
                    .iter()
                    .find(|choice| choice.key == *key)
                    .map_or(MeetingPolicy::Ask, |choice| choice.policy);
                serde_json::to_value(policy).unwrap_or(Value::Null)
            }
        }
    }

    /// The whole key with the part set to `value`: `(key, its JSON)`.
    pub fn with(
        &self,
        settings: &AppSettings,
        value: &Value,
    ) -> Result<(&'static str, Value), String> {
        let json = match self {
            Self::Prompt => {
                let Value::String(prompt) = value else {
                    return Err("instructions are text".into());
                };
                let mut mode = settings.dictation_processing.default_mode.clone();
                mode.post_processing.prompt = prompt.clone();
                serde_json::to_value(mode)
            }
            Self::Cleanup => {
                let Value::Bool(on) = value else {
                    return Err("expected on or off".into());
                };
                let mut mode = settings.dictation_processing.default_mode.clone();
                mode.post_processing.enabled = *on;
                serde_json::to_value(mode)
            }
            Self::Provider { key, label } => {
                let policy: MeetingPolicy = serde_json::from_value(value.clone())
                    .map_err(|_| "expected ask, always_record, or never_offer".to_owned())?;
                let mut meetings = settings.meetings.clone();
                crate::app_settings::set_meeting_provider(
                    &mut meetings,
                    MeetingProviderChoice {
                        key: key.clone(),
                        label: label.clone(),
                        policy,
                    },
                );
                serde_json::to_value(meetings.providers)
            }
        };
        json.map(|json| (self.parent(), json))
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_app_is_found_by_its_key_or_its_name() {
        let settings = AppSettings::default();
        for app in ["zoom", "Zoom", "us.zoom.xos"] {
            let part = part(&format!("meetings.providers.{app}.policy"), &settings);
            assert_eq!(
                part,
                Some(Part::Provider {
                    key: "us.zoom.xos".into(),
                    label: "Zoom".into()
                }),
                "{app}"
            );
        }
        assert!(part("meetings.providers.googlemeet.policy", &settings).is_some());
        assert!(part("meetings.providers.nothing.policy", &settings).is_none());
        assert!(part("meetings.providers", &settings).is_none());
    }

    #[test]
    fn a_part_writes_back_into_its_whole_key() {
        let settings = AppSettings::default();
        let zoom = part("meetings.providers.zoom.policy", &settings).unwrap();
        assert_eq!(zoom.value(&settings), json!("ask"));
        let (parent, json) = zoom.with(&settings, &json!("always_record")).unwrap();
        assert_eq!(parent, "meetings.providers");
        assert_eq!(json[0]["key"], "us.zoom.xos");
        assert_eq!(json[0]["policy"], "always_record");
        assert!(zoom.with(&settings, &json!("sometimes")).is_err());
        let (parent, mode) = Part::Prompt
            .with(&settings, &json!("Fix punctuation."))
            .unwrap();
        assert_eq!(parent, "dictation_processing.default_mode");
        assert_eq!(mode["post_processing"]["prompt"], "Fix punctuation.");
        let cleanup = part(CLEANUP, &settings).unwrap();
        assert_eq!(cleanup.value(&settings), json!(false));
        let (parent, mode) = cleanup.with(&settings, &json!(true)).unwrap();
        assert_eq!(parent, "dictation_processing.default_mode");
        assert_eq!(mode["post_processing"]["enabled"], true);
        assert!(cleanup.with(&settings, &json!("on")).is_err());
    }
}
