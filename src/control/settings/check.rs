//! The one path a change from outside a pane's own control takes before it
//! is saved: `hex settings set` and `apply` on every OS, and Assistant's
//! cards. It refuses what this OS or an organization doesn't allow, then
//! validates the values together, all or none, with the rules and side
//! effects the window applies ([`super::apply_all`]).

use serde_json::{Map, Value};

use super::{Change, ShortcutRules, apply, apply_all, unavailable_under};
use crate::app_settings::AppSettings;
use crate::control::ControlError;
use crate::policy::Policy;

/// Validates every `key: value` in `values` against `settings`, all or
/// none: the settings to save and the effects to carry out.
pub fn check(
    settings: &AppSettings,
    values: &Map<String, Value>,
    shortcuts: &ShortcutRules,
) -> Result<Change, ControlError> {
    check_under(settings, values, shortcuts, Policy::current())
}

/// [`check`] for one key given as typed (`hex settings set`).
pub fn check_one(
    settings: &AppSettings,
    name: &str,
    raw: &str,
    shortcuts: &ShortcutRules,
) -> Result<Change, ControlError> {
    if let Some(error) = unavailable_under(name, Policy::current()) {
        return Err(error);
    }
    apply(settings, name, raw, shortcuts)
}

fn check_under(
    settings: &AppSettings,
    values: &Map<String, Value>,
    shortcuts: &ShortcutRules,
    policy: Policy,
) -> Result<Change, ControlError> {
    // Every key, the model's included, which `apply_all` selects without
    // asking.
    if let Some(error) = values
        .keys()
        .find_map(|name| unavailable_under(name, policy))
    {
        return Err(error);
    }
    apply_all(settings, values, shortcuts)
}

/// What a change does to every `hex settings` key: `(key, before, after)`
/// for each one whose value differs, in registry order. The keys a rule
/// changed beside the ones asked (vibe coding's Labs features) are in it.
/// The API keys are left out: they live in the keychain, not in settings.
pub fn diff(before: &AppSettings, after: &AppSettings) -> Vec<(String, Value, Value)> {
    values(before)
        .into_iter()
        .zip(values(after))
        .filter(|((_, old), (_, new))| old != new)
        .map(|((key, old), (_, new))| (key.to_owned(), old, new))
        .collect()
}

/// Every registry key's value, as `hex settings get` shows it, without the
/// API keys (no keychain is read).
pub fn values(settings: &AppSettings) -> Vec<(&'static str, Value)> {
    let json = serde_json::to_value(settings).unwrap_or(Value::Null);
    super::keys()
        .map(|key| (key.key, super::value_of(key, &json).value))
        .collect()
}

/// Every registry key with its kind (`bool`, `choice`, …), its choices,
/// and whether it can be set at all, without the API keys.
pub fn kinds() -> impl Iterator<Item = (&'static str, &'static str, Vec<&'static str>, bool)> {
    super::keys().map(|key| {
        let choices = match key.kind {
            super::Kind::Choice(choices) => choices.to_vec(),
            _ => Vec::new(),
        };
        (key.key, key.kind.name(), choices, key.read_only.is_none())
    })
}

/// The range a number or whole-number key accepts.
pub fn bounds(name: &str) -> Option<(f64, f64)> {
    match super::keys().find(|key| key.key == name)?.kind {
        super::Kind::Number { min, max } => Some((min, max)),
        super::Kind::Integer { min, max } => Some((min as f64, max as f64)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules() -> ShortcutRules {
        ShortcutRules {
            key_named: |name| Ok(crate::app_settings::HotkeyKey::named(name)),
            rule: |_, _| Ok(()),
        }
    }

    fn values(pairs: &[(&str, Value)]) -> Map<String, Value> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect()
    }

    #[test]
    fn a_change_and_what_it_needs_save_together() {
        let mut settings = AppSettings::default();
        settings.labs.ai_cleanup_server = false;
        let change = check(
            &settings,
            &values(&[
                (
                    "dictation_processing.server.url",
                    json!("http://localhost:1234/v1"),
                ),
                ("labs.ai_cleanup_server", json!(true)),
            ]),
            &rules(),
        )
        .unwrap();
        assert!(change.settings.labs.ai_cleanup_server);
        assert_eq!(
            change.settings.dictation_processing.server.url,
            "http://localhost:1234/v1"
        );
    }

    #[test]
    fn one_bad_value_saves_nothing() {
        let settings = AppSettings::default();
        let refused = check(
            &settings,
            &values(&[
                ("labs.smart_spacing", json!(true)),
                ("recording_duck_volume", json!(1.4)),
            ]),
            &rules(),
        )
        .unwrap_err();
        assert_eq!(refused.code, "invalid-value");
        let unknown = check(&settings, &values(&[("labs.nope", json!(true))]), &rules());
        assert_eq!(unknown.unwrap_err().code, "unknown-setting");
    }

    /// A lock refuses the model keys too, which `apply_all` selects
    /// without asking whether they may change.
    #[test]
    fn a_managed_key_is_refused_before_anything_is_validated() {
        use crate::policy::Lock;
        let policy = Policy::default().with(Lock::Meetings);
        let refused = check_under(
            &AppSettings::default(),
            &values(&[
                ("sound_effects", json!(false)),
                ("meetings.enabled", json!(true)),
            ]),
            &rules(),
            policy,
        )
        .unwrap_err();
        assert_eq!(refused.code, "managed");
    }

    #[test]
    fn one_key_as_typed_takes_the_same_path() {
        let settings = AppSettings::default();
        let change = check_one(&settings, "labs.smart_spacing", "on", &rules()).unwrap();
        assert!(change.settings.labs.smart_spacing);
        let refused = check_one(&settings, "recording_duck_volume", "1.4", &rules());
        assert_eq!(refused.unwrap_err().code, "invalid-value");
    }

    #[test]
    fn the_diff_names_what_a_rule_changed_beside_the_key() {
        let mut before = AppSettings::default();
        before.labs.russian_vibe_coding = false;
        before.labs.smart_spacing = false;
        let change = check(
            &before,
            &values(&[("labs.russian_vibe_coding", json!(true))]),
            &rules(),
        )
        .unwrap();
        let diff = diff(&before, &change.settings);
        let keys: Vec<&str> = diff.iter().map(|(key, _, _)| key.as_str()).collect();
        assert!(keys.contains(&"labs.russian_vibe_coding"));
        assert!(keys.len() > 1, "vibe coding changed only itself: {keys:?}");
        assert!(diff.iter().all(|(_, old, new)| old != new));
        assert!(super::diff(&before, &before).is_empty());
    }
}
