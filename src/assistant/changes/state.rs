//! What a card shows now, derived each frame from what was decided and
//! what is true now: the settings (changed by hand since?), the model
//! being downloaded, the Codex sign-in, the OS permissions, and whether
//! Undo still holds this session. Nothing here is stored.

use serde_json::Value;

use super::catalog::{self, Class, Permission, Shorter};
use super::{Card, CardKind, Decision, current};
use crate::app_settings::AppSettings;
use crate::control::settings;

/// What is true now, as far as a card cares.
pub struct Live<'a> {
    pub settings: &'a AppSettings,
    /// The speech model the service is preparing: its id, bytes come, and
    /// bytes in all (0 while unknown).
    pub preparing: Option<(String, u64, u64)>,
    /// Whether Codex is signed in; `None` until something asked it.
    pub codex_signed_in: Option<bool>,
    /// Screen & System Audio Recording is allowed (true off macOS).
    pub system_audio: bool,
    /// Undo is still offered for this card: this session, its
    /// conversation open since it was applied.
    pub undoable: bool,
}

/// What a card shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Shown {
    /// Its buttons: the change, and Not now. `shorter` names what a
    /// shorter retention removes; `also` what it turns on beside.
    Proposed {
        shorter: Option<Shorter>,
        also: Vec<(String, Value)>,
    },
    /// Saved. Undo while `undo`.
    Applied {
        undo: bool,
    },
    Declined,
    Undone,
    Failed(String),
    /// The value moved since: by hand, or another card.
    ChangedSince {
        now: Value,
    },
    /// Already set as suggested.
    Already,
    /// A speech model downloading; it switches when done.
    Downloading {
        done: u64,
        total: u64,
    },
    /// Saved; an OS permission is still to grant.
    Permission(Permission),
    /// Runs on Codex, which isn't signed in.
    SignIn,
    /// Managed by an organization, or not on this OS.
    Blocked {
        managed: bool,
        why: String,
    },
}

/// What `card` shows with `live`.
pub fn shown(card: &Card, live: &Live) -> Shown {
    let Some(item) = card.items.first() else {
        return Shown::Declined;
    };
    let to = match &card.kind {
        CardKind::Choice {
            options, chosen, ..
        } => options.get(*chosen).unwrap_or(&item.to),
        _ => &item.to,
    };
    let now = current(live.settings, &item.key).unwrap_or(Value::Null);
    match &card.decision {
        Decision::Declined => Shown::Declined,
        Decision::Undone => Shown::Undone,
        Decision::Failed { why } => Shown::Failed(why.clone()),
        Decision::Applied { after, .. } => {
            if let Some(shown) = downloading(item.class, &item.key, to, live) {
                return shown;
            }
            let moved = after
                .iter()
                .find(|(key, value)| current(live.settings, key).is_some_and(|now| now != **value));
            if moved.is_some() {
                return Shown::ChangedSince { now };
            }
            if catalog::permission(&item.key, to) == Some(Permission::SystemAudio)
                && !live.system_audio
                && !card.later
            {
                return Shown::Permission(Permission::SystemAudio);
            }
            Shown::Applied {
                undo: live.undoable,
            }
        }
        Decision::Proposed => {
            if let Some(blocked) = blocked(card, live.settings) {
                return blocked;
            }
            if card.kind == CardKind::Plan {
                return Shown::Proposed {
                    shorter: None,
                    also: Vec::new(),
                };
            }
            if let Some(shown) = downloading(item.class, &item.key, to, live) {
                return shown;
            }
            // A choice shows its options whatever the value is now.
            let choice = matches!(card.kind, CardKind::Choice { .. });
            if !choice && now == *to {
                return Shown::Already;
            }
            if now != item.from {
                return Shown::ChangedSince { now };
            }
            if catalog::needs_codex(&item.key, to) && live.codex_signed_in == Some(false) {
                return Shown::SignIn;
            }
            let also = item
                .requires
                .iter()
                .filter(|(key, value)| current(live.settings, key).as_ref() != Some(value))
                .cloned()
                .collect();
            Shown::Proposed {
                shorter: catalog::shorter(&item.key, &item.from, to),
                also,
            }
        }
    }
}

fn downloading(class: Class, key: &str, to: &Value, live: &Live) -> Option<Shown> {
    let (model, done, total) = live.preparing.as_ref()?;
    let wanted = match key {
        "transcription.model" => to.as_str()?,
        "transcription.language" => live.settings.transcription.model.as_str(),
        _ => return None,
    };
    (class == Class::Download && model == wanted).then_some(Shown::Downloading {
        done: *done,
        total: *total,
    })
}

/// Why `card` can't be applied here, if it can't.
fn blocked(card: &Card, settings: &AppSettings) -> Option<Shown> {
    card.items
        .iter()
        .flat_map(|item| {
            let key = match super::parts::part(&item.key, settings) {
                Some(part) => part.parent().to_owned(),
                None => item.key.clone(),
            };
            std::iter::once(key).chain(item.requires.iter().map(|(key, _)| key.clone()))
        })
        .find_map(|key| settings::unavailable(&key))
        .map(|error| Shown::Blocked {
            managed: error.code == "managed",
            why: error.message,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::{Item, applied};
    use serde_json::json;

    fn card(key: &str, from: Value, to: Value) -> Card {
        Card {
            items: vec![Item {
                key: key.into(),
                from,
                to,
                included: true,
                ..Item::default()
            }],
            ..Card::default()
        }
    }

    fn live(settings: &AppSettings) -> Live<'_> {
        Live {
            settings,
            preparing: None,
            codex_signed_in: None,
            system_audio: true,
            undoable: true,
        }
    }

    #[test]
    fn a_card_follows_its_decision_and_the_settings() {
        let mut settings = AppSettings::default();
        settings.labs.smart_spacing = false;
        let mut smart = card("labs.smart_spacing", json!(false), json!(true));
        assert_eq!(
            shown(&smart, &live(&settings)),
            Shown::Proposed {
                shorter: None,
                also: Vec::new()
            }
        );
        smart.decision = Decision::Declined;
        assert_eq!(shown(&smart, &live(&settings)), Shown::Declined);
        // Turned on by hand meanwhile.
        smart.decision = Decision::Proposed;
        settings.labs.smart_spacing = true;
        assert_eq!(shown(&smart, &live(&settings)), Shown::Already);
        // Applied, then Undo for as long as the session holds it.
        smart.decision = applied(
            vec![("labs.smart_spacing".into(), json!(false), json!(true))],
            0,
        );
        assert_eq!(
            shown(&smart, &live(&settings)),
            Shown::Applied { undo: true }
        );
        let later = Live {
            undoable: false,
            ..live(&settings)
        };
        assert_eq!(shown(&smart, &later), Shown::Applied { undo: false });
        // Changed by hand after applying: no Undo.
        settings.labs.smart_spacing = false;
        assert_eq!(
            shown(&smart, &live(&settings)),
            Shown::ChangedSince { now: json!(false) }
        );
    }

    #[test]
    fn a_value_moved_since_the_suggestion_is_named() {
        let mut settings = AppSettings::default();
        settings.dictation_processing.server.runs_on =
            crate::app_settings::CleanupRunsOn::Openrouter;
        let runs_on = card(
            "dictation_processing.server.runs_on",
            json!("opencode"),
            json!("codex"),
        );
        assert_eq!(
            shown(&runs_on, &live(&settings)),
            Shown::ChangedSince {
                now: json!("openrouter")
            }
        );
    }

    #[test]
    fn codex_signed_out_asks_for_a_sign_in() {
        let settings = AppSettings::default();
        let from = current(&settings, "dictation_processing.server.runs_on").unwrap();
        let runs_on = card("dictation_processing.server.runs_on", from, json!("codex"));
        let signed_out = Live {
            codex_signed_in: Some(false),
            ..live(&settings)
        };
        assert_eq!(shown(&runs_on, &signed_out), Shown::SignIn);
    }

    #[test]
    fn a_download_shows_its_progress_and_a_permission_its_ask() {
        let settings = AppSettings::default();
        let mut model = card(
            "transcription.model",
            json!("parakeet_v3"),
            json!("gigaam_v3"),
        );
        model.items[0].class = Class::Download;
        let downloading = Live {
            preparing: Some(("gigaam_v3".into(), 115, 274)),
            ..live(&settings)
        };
        assert_eq!(
            shown(&model, &downloading),
            Shown::Downloading {
                done: 115,
                total: 274
            }
        );
        let mut settings = AppSettings::default();
        settings.meetings.enabled = true;
        let mut meetings = card("meetings.enabled", json!(false), json!(true));
        meetings.decision = applied(
            vec![("meetings.enabled".into(), json!(false), json!(true))],
            0,
        );
        let not_allowed = Live {
            system_audio: false,
            ..live(&settings)
        };
        assert_eq!(
            shown(&meetings, &not_allowed),
            Shown::Permission(Permission::SystemAudio)
        );
        meetings.later = true;
        assert_eq!(
            shown(&meetings, &not_allowed),
            Shown::Applied { undo: true }
        );
    }

    #[test]
    fn a_shorter_retention_warns_and_what_it_needs_shows() {
        let mut settings = AppSettings::default();
        settings.meetings.enabled = false;
        let from = current(&settings, "meetings.retention").unwrap();
        let mut retention = card("meetings.retention", from, json!("day"));
        retention.items[0].requires = vec![("meetings.enabled".into(), json!(true))];
        let Shown::Proposed { shorter, also } = shown(&retention, &live(&settings)) else {
            panic!("not proposed");
        };
        assert_eq!(shorter, Some(Shorter::Meetings));
        assert_eq!(also, [("meetings.enabled".to_owned(), json!(true))]);
    }
}
