//! A switch's companion value, offered in the switch's own row: a plan
//! row that turns on Unload model when idle reads "Off → On, after
//! [15 min ▾]", since the switch alone silently keeps whatever minutes
//! were saved. Its choice starts at the value saved now (never at another
//! one, even one the person set themselves); picking another is part of
//! the same change and the same save (`changes::values`). A row the plan
//! keeps as it is ("Already set") carries it too: changed there, it is one
//! change; untouched, none (`counts`).
//!
//! The pairs are one list, [`PAIRS`]: what the registry's rules imply
//! (`control::settings::requires`), a choice that does nothing without
//! one switch whatever it is. A test holds the list to the rules and its
//! choices to what a save takes. It is the app's rule, for a plan or a
//! card from any engine.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::current;
use crate::app_settings::AppSettings;
use crate::control::settings;
use crate::desktop::i18n::{tr, tr_fill};

/// A switch and the choice that goes with it.
#[derive(Clone, Copy, Debug)]
pub struct Pair {
    pub switch: &'static str,
    pub value: &'static str,
    /// The words around the choice, in English for `tr`: `{}` is where
    /// it stands ("after {}"; another language may put words after it).
    pub says: &'static str,
}

/// Every switch with a companion.
pub const PAIRS: [Pair; 4] = [
    Pair {
        switch: "labs.unload_idle",
        value: "labs.unload_idle_minutes",
        says: "after {}",
    },
    Pair {
        switch: "meetings.calendar.remind",
        value: "meetings.calendar.lead_minutes",
        says: "remind me {}",
    },
    Pair {
        switch: "labs.caps_lock_dictation",
        value: "labs.caps_lock_light",
        says: "light {}",
    },
    Pair {
        switch: "experimental.hex_microphone",
        value: "experimental.hex_microphone_mode",
        says: "voice {}",
    },
];

/// A row's companion: its choices, the value saved now, and the one
/// picked.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Companion {
    pub key: String,
    pub from: Value,
    pub options: Vec<Value>,
    pub chosen: usize,
}

impl Companion {
    /// The value it saves.
    pub fn to(&self) -> &Value {
        self.options.get(self.chosen).unwrap_or(&self.from)
    }

    /// Picked other than the value saved now.
    pub fn changed(&self) -> bool {
        *self.to() != self.from
    }

    /// Its pair.
    pub fn pair(&self) -> Option<&'static Pair> {
        PAIRS.iter().find(|pair| pair.value == self.key)
    }
}

/// The companion of `key` going to (or staying at) `to`: only for a
/// switch turned on, with its value now among the choices a save takes.
pub fn of(settings: &AppSettings, key: &str, to: &Value) -> Option<Companion> {
    if *to != Value::Bool(true) {
        return None;
    }
    let pair = PAIRS.iter().find(|pair| pair.switch == key)?;
    settings::unavailable(pair.value).is_none().then_some(())?;
    let (_, _, choices, writable) = settings::kinds().find(|(known, ..)| *known == pair.value)?;
    let from = current(settings, pair.value)?;
    let options: Vec<Value> = choices
        .iter()
        .map(|choice| Value::String((*choice).into()))
        .collect();
    // A value no choice is (an older file's) has nothing to start at.
    let chosen = options.iter().position(|option| *option == from)?;
    writable.then(|| Companion {
        key: pair.value.to_owned(),
        from,
        options,
        chosen,
    })
}

/// How a companion's value reads, in the window's language: "15 min",
/// "At start", "Slow blink", "Muffled".
pub fn label(key: &str, value: &Value) -> String {
    let text = value.as_str().unwrap_or_default();
    if key.ends_with("_minutes")
        && let Ok(minutes) = text.parse::<u32>()
    {
        return match minutes {
            0 => tr("At start").to_owned(),
            minutes => tr_fill("{} min", &minutes.to_string()),
        };
    }
    let mut words = text.replace('_', " ");
    if let Some(first) = words.get(..1) {
        words = first.to_uppercase() + &words[1..];
    }
    tr(&words).to_owned()
}

/// The words before and after the choice: "after", "" for "after {}".
pub fn around(pair: &Pair) -> (String, String) {
    let says = tr(pair.says);
    match says.split_once("{}") {
        Some((before, after)) => (before.trim().to_owned(), after.trim().to_owned()),
        None => (says.trim().to_owned(), String::new()),
    }
}

/// The words around a choice, Assistant's own, for the translation
/// tables' test. The choices' own names ("15 min", "Slow blink",
/// "Muffled") are the window's, which its panes translate already.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    PAIRS.iter().map(|pair| pair.says).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The list is what the rules imply, every pair names a switch and a
    /// choice the registry has, and each choice is one a save takes,
    /// with the switch.
    #[test]
    fn every_pair_is_a_rule_and_a_choice_a_save_takes() {
        let listed: Vec<(&str, &str)> =
            PAIRS.iter().map(|pair| (pair.switch, pair.value)).collect();
        let mut derived = settings::companions();
        derived.sort();
        let mut sorted = listed.clone();
        sorted.sort();
        assert_eq!(sorted, derived, "PAIRS differs from what the rules imply");
        let settings = AppSettings::default();
        let rules = settings::ShortcutRules {
            key_named: |name| Ok(crate::app_settings::HotkeyKey::named(name)),
            rule: |_, _| Ok(()),
        };
        for pair in PAIRS {
            let kinds = |key: &str| settings::kinds().find(|(known, ..)| *known == key);
            let (_, switch, ..) = kinds(pair.switch).expect("a switch the registry has");
            assert_eq!(switch, "bool", "{}", pair.switch);
            let (_, kind, choices, writable) = kinds(pair.value).expect("a choice it has");
            assert!(
                kind == "choice" && !choices.is_empty() && writable,
                "{}",
                pair.value
            );
            assert!(pair.says.contains("{}"), "{}", pair.says);
            assert!(!super::super::catalog::never(pair.switch));
            assert!(!super::super::catalog::never(pair.value));
            // A pair this OS lacks (calendar reminders off macOS) is never
            // offered here, and a save would refuse it.
            if settings::unavailable(pair.value).is_some() {
                assert!(of(&settings, pair.switch, &json!(true)).is_none());
                continue;
            }
            for choice in choices {
                let values = serde_json::Map::from_iter([
                    (pair.switch.to_owned(), json!(true)),
                    (pair.value.to_owned(), json!(choice)),
                ]);
                let mut values = values;
                for (needs, want) in settings::missing(&settings, pair.switch, &json!(true)) {
                    values.insert(needs, want);
                }
                let saved = settings::check(&settings, &values, &rules)
                    .unwrap_or_else(|error| panic!("{} = {choice}: {}", pair.value, error.message));
                let now = current(&saved.settings, pair.value).unwrap();
                assert_eq!(now, json!(choice), "{}", pair.value);
            }
        }
    }

    /// A switch turned on carries its companion at the value saved now; a
    /// switch turned off, or one with none, carries nothing.
    #[test]
    fn a_switch_turned_on_starts_at_the_value_saved() {
        let mut settings = AppSettings::default();
        settings.labs.unload_idle_minutes = 30;
        let companion = of(&settings, "labs.unload_idle", &json!(true)).unwrap();
        assert_eq!(companion.key, "labs.unload_idle_minutes");
        assert_eq!(companion.from, json!("30"));
        assert_eq!(*companion.to(), json!("30"));
        assert!(!companion.changed());
        assert_eq!(companion.options.len(), 4);
        assert!(of(&settings, "labs.unload_idle", &json!(false)).is_none());
        assert!(of(&settings, "labs.smart_spacing", &json!(true)).is_none());
        // Calendar reminders are macOS's: elsewhere the switch has none.
        let lead = of(&settings, "meetings.calendar.remind", &json!(true));
        match settings::unavailable("meetings.calendar.lead_minutes") {
            None => assert_eq!(lead.unwrap().from, json!("1")),
            Some(_) => assert!(lead.is_none()),
        }
    }

    #[test]
    fn a_choice_reads_as_the_window_names_it() {
        assert_eq!(label("labs.unload_idle_minutes", &json!("15")), "15 min");
        assert_eq!(
            label("meetings.calendar.lead_minutes", &json!("0")),
            "At start"
        );
        assert_eq!(
            label("labs.caps_lock_light", &json!("slow_blink")),
            "Slow blink"
        );
        assert_eq!(
            label("experimental.hex_microphone_mode", &json!("muffled")),
            "Muffled"
        );
        let (before, after) = around(&PAIRS[0]);
        assert_eq!((before.as_str(), after.as_str()), ("after", ""));
        let (before, after) = around(&PAIRS[1]);
        assert_eq!((before.as_str(), after.as_str()), ("remind me", ""));
    }

    /// A follow-up's Yes to Unload model when idle is a row that carries its
    /// minutes at the value saved; with it on already, the row kept as it is
    /// carries them too. Nothing preselects another value, the person's own
    /// included.
    #[test]
    fn a_switch_comes_with_its_companion_at_the_value_saved() {
        use crate::assistant::facts::State;
        use crate::assistant::setup::{Answer, plan};
        let mut settings = AppSettings::default();
        crate::assistant::setup::sample(&mut settings);
        let state = State {
            os: "macos",
            ..State::default()
        };
        settings.labs.unload_idle = false;
        settings.labs.unload_idle_minutes = 30;
        let answers = [Answer {
            id: "labs.unload_idle".into(),
            chosen: vec!["yes".into()],
            changes: vec![("labs.unload_idle".into(), json!(true))],
        }];
        let card = plan::plan(Vec::new(), &answers, &settings, &state).unwrap();
        let row = card
            .items
            .iter()
            .find(|item| item.key == "labs.unload_idle")
            .unwrap();
        let companion = row.companion.as_ref().unwrap();
        assert_eq!(companion.key, "labs.unload_idle_minutes");
        assert_eq!(
            (&companion.from, companion.to()),
            (&json!("30"), &json!("30"))
        );
        settings.labs.unload_idle = true;
        let card = plan::plan(Vec::new(), &answers, &settings, &state).unwrap();
        assert!(card.items.iter().all(|item| item.key != "labs.unload_idle"));
        let kept = card
            .kept
            .iter()
            .find(|item| item.key == "labs.unload_idle")
            .unwrap();
        assert!(!kept.companion.as_ref().unwrap().changed());
    }
}
