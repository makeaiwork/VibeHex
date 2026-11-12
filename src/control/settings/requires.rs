//! What a setting needs before it does anything: a shortcut its Labs
//! feature, a Meetings setting Meetings itself, `double_tap_only` the
//! double-tap lock. A registry fact, so Assistant's cards and, later, the
//! CLI ask for the same thing: a change that names a key also turns on what
//! that key needs, in one all-or-none save ([`super::check`]).

use serde_json::Value;

use crate::app_settings::AppSettings;

/// Which values of a key need the rule's setting.
#[derive(Clone, Copy, Debug)]
enum When {
    /// Every value.
    Any,
    /// A value that turns something on: `true`, a shortcut, a non-empty
    /// text. `false`, `null`, and `""` need nothing.
    Set,
    /// Every value but this choice (`runs_on` opencode works without the
    /// lab).
    Not(&'static str),
}

/// The value a needed setting must hold.
#[derive(Clone, Copy, Debug)]
enum Want {
    On,
    Is(&'static str),
}

impl Want {
    fn value(self) -> Value {
        match self {
            Self::On => Value::Bool(true),
            Self::Is(choice) => Value::String(choice.into()),
        }
    }
}

/// `key` (exact, or every key under a prefix ending in `.`) needs `needs`
/// set to `want` for the values `when` names.
struct Rule {
    key: &'static str,
    when: When,
    needs: &'static str,
    want: Want,
}

const fn rule(key: &'static str, when: When, needs: &'static str) -> Rule {
    Rule {
        key,
        when,
        needs,
        want: Want::On,
    }
}

/// The server settings Labs › Clean up on your own server shows.
const CLEANUP_SERVER: [&str; 7] = [
    "dictation_processing.server.url",
    "dictation_processing.server.openrouter_model",
    "dictation_processing.server.custom_model",
    "dictation_processing.server.local_model",
    "dictation_processing.server.codex_model",
    "dictation_processing.server.codex_effort",
    "dictation_processing.server.codex_wait_seconds",
];

const RULES: &[Rule] = &[
    rule(
        "dictation_processing.server.runs_on",
        When::Not("opencode"),
        "labs.ai_cleanup_server",
    ),
    rule(CLEANUP_SERVER[0], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[1], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[2], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[3], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[4], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[5], When::Any, "labs.ai_cleanup_server"),
    rule(CLEANUP_SERVER[6], When::Any, "labs.ai_cleanup_server"),
    rule("meetings.", When::Any, "meetings.enabled"),
    rule(
        "meetings.audio_retention",
        When::Set,
        "labs.expire_meeting_audio",
    ),
    rule(
        "meetings.questions.enabled",
        When::Set,
        "labs.ai_cleanup_server",
    ),
    rule("stop_hotkey", When::Set, "labs.stop_shortcut"),
    rule(
        "second_dictation_hotkey",
        When::Set,
        "labs.second_dictation_shortcut",
    ),
    rule(
        "vocabulary_add_hotkey",
        When::Set,
        "labs.vocabulary_quick_add",
    ),
    rule("typed_text_hotkey", When::Set, "labs.typed_text"),
    rule(
        "terminal_paste_hotkey",
        When::Set,
        "labs.terminal_paste_shortcut",
    ),
    rule("screenshots.", When::Any, "labs.screenshots"),
    rule("files.", When::Any, "labs.files"),
    Rule {
        key: "pause_browser_media",
        when: When::Set,
        needs: "recording_audio_behavior",
        want: Want::Is("pause_media"),
    },
    rule("double_tap_only", When::Set, "double_tap_lock"),
    rule(
        "labs.caps_lock_light",
        When::Set,
        "labs.caps_lock_dictation",
    ),
    rule("labs.unload_idle_minutes", When::Any, "labs.unload_idle"),
    rule(
        "labs.sidebar_collapsed",
        When::Set,
        "labs.collapsible_sidebar",
    ),
    rule(
        "experimental.hex_microphone_mode",
        When::Any,
        "experimental.hex_microphone",
    ),
    rule(
        "experimental.hex_microphone_comfort_noise",
        When::Set,
        "experimental.hex_microphone",
    ),
    rule(
        "experimental.hex_microphone_strength",
        When::Any,
        "experimental.hex_microphone",
    ),
];

/// Annotate's settings, in builds that have it.
#[cfg(hex_capture)]
const CAPTURE_RULES: &[Rule] = &[
    rule("capture.replay_hotkey", When::Set, "labs.annotate_replay"),
    rule("screenshots.record_hotkey", When::Set, "capture.enabled"),
];
#[cfg(not(hex_capture))]
const CAPTURE_RULES: &[Rule] = &[];

fn rules() -> impl Iterator<Item = &'static Rule> {
    RULES.iter().chain(CAPTURE_RULES)
}

impl Rule {
    fn matches(&self, key: &str, value: &Value) -> bool {
        let named = match self.key.strip_suffix('.') {
            Some(_) => key.starts_with(self.key),
            None => key == self.key,
        };
        named && key != self.needs && self.when.holds(value)
    }
}

impl When {
    fn holds(self, value: &Value) -> bool {
        match self {
            Self::Any => true,
            Self::Set => match value {
                Value::Null | Value::Bool(false) => false,
                Value::String(text) => !matches!(
                    text.trim().to_ascii_lowercase().as_str(),
                    "" | "none" | "off"
                ),
                _ => true,
            },
            Self::Not(choice) => value.as_str() != Some(choice),
        }
    }
}

/// What setting `key` to `value` needs, directly: `(key, value)` pairs.
pub fn requires(key: &str, value: &Value) -> Vec<(&'static str, Value)> {
    rules()
        .filter(|rule| rule.matches(key, value))
        .map(|rule| (rule.needs, rule.want.value()))
        .collect()
}

/// What setting `key` to `value` needs that `settings` doesn't hold yet,
/// with what those need in turn, each once.
pub fn missing(settings: &AppSettings, key: &str, value: &Value) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = Vec::new();
    let mut queue = vec![(key.to_owned(), value.clone())];
    while let Some((key, value)) = queue.pop() {
        for (needs, want) in requires(&key, &value) {
            if out.iter().any(|(known, _)| known == needs) || holds(settings, needs, &want) {
                continue;
            }
            out.push((needs.to_owned(), want.clone()));
            queue.push((needs.to_owned(), want));
        }
    }
    out
}

/// Whether `settings` has `key` at `want`.
fn holds(settings: &AppSettings, key: &str, want: &Value) -> bool {
    super::get(settings, key)
        .ok()
        .and_then(|found| found.into_iter().find(|setting| setting.key == key))
        .is_some_and(|setting| setting.value == *want)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys() -> Vec<&'static str> {
        super::super::keys().map(|key| key.key).collect()
    }

    /// A rule names keys the registry has, and needs a key it has.
    #[test]
    fn every_rule_names_real_keys() {
        let keys = keys();
        for rule in rules() {
            assert!(keys.contains(&rule.needs), "{} needs no key", rule.needs);
            let named = match rule.key.strip_suffix('.') {
                Some(_) => keys.iter().any(|key| key.starts_with(rule.key)),
                None => keys.contains(&rule.key),
            };
            assert!(named, "{} names no key", rule.key);
        }
    }

    /// An `about` that says "with labs.x" or "while labs.x is on" has its
    /// rule, so a card turns on what the key needs.
    #[test]
    fn every_with_labs_in_an_about_has_a_rule() {
        let mut checked = 0;
        for key in super::super::keys() {
            let about = key.about.replace(['\n', '\\'], " ");
            for marker in ["with labs.", "while labs."] {
                for (at, _) in about.match_indices(marker) {
                    let rest = &about[at + marker.len() - "labs.".len()..];
                    let needs: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                        .collect();
                    let needs = needs.trim_end_matches('.');
                    let named = requires(key.key, &json!(true));
                    assert!(
                        named.iter().any(|(known, _)| *known == needs),
                        "{} says it needs {needs}, and no rule does",
                        key.key
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 8, "only {checked} mentions found");
    }

    #[test]
    fn a_value_decides_what_it_needs() {
        let codex = requires("dictation_processing.server.runs_on", &json!("codex"));
        assert_eq!(codex, [("labs.ai_cleanup_server", json!(true))]);
        assert!(requires("dictation_processing.server.runs_on", &json!("opencode")).is_empty());
        assert_eq!(
            requires("stop_hotkey", &json!("return")),
            [("labs.stop_shortcut", json!(true))]
        );
        assert!(requires("stop_hotkey", &Value::Null).is_empty());
        assert!(requires("double_tap_only", &json!(false)).is_empty());
        assert_eq!(
            requires("pause_browser_media", &json!(true)),
            [("recording_audio_behavior", json!("pause_media"))]
        );
        // Meetings itself needs nothing; its settings need it.
        assert!(requires("meetings.enabled", &json!(true)).is_empty());
        assert_eq!(
            requires("meetings.summaries", &json!("opencode")),
            [("meetings.enabled", json!(true))]
        );
    }

    #[test]
    fn missing_leaves_out_what_holds_and_follows_what_it_needs() {
        let mut settings = AppSettings::default();
        settings.meetings.enabled = false;
        settings.labs.ai_cleanup_server = false;
        let missing = missing(&settings, "meetings.questions.enabled", &json!(true));
        let names: Vec<&str> = missing.iter().map(|(key, _)| key.as_str()).collect();
        assert_eq!(names, ["meetings.enabled", "labs.ai_cleanup_server"]);
        settings.meetings.enabled = true;
        settings.labs.ai_cleanup_server = true;
        assert!(super::missing(&settings, "meetings.questions.enabled", &json!(true)).is_empty());
    }
}
