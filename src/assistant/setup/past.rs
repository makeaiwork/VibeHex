//! The setups finished before, kept with the conversations
//! (`store::Store::setups`): when, the answers, and what the plan applied
//! or the person declined. A returning setup starts from them, and a line
//! once declined (a row left unticked, a follow-up's "No") isn't offered
//! again while the setting is still as it was then; once it changed, by
//! hand or by another plan, it may be.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Answer, Ask};
use crate::app_settings::AppSettings;
use crate::assistant::changes::{self, Card, CardKind, Decision};

/// The most setups kept; the oldest go first.
pub const MAX: usize = 20;

/// One finished setup.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Setup {
    pub at_ms: u64,
    pub answers: Vec<Answer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub applied: Vec<Line>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub declined: Vec<Line>,
}

/// One setting a setup changed or was offered: from what, to what.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Line {
    pub key: String,
    pub to: Value,
    pub from: Value,
}

/// The setup a decided plan finishes. Applied, its ticked rows are
/// applied and the unticked ones declined; put off, none is either. A
/// follow-up answered "No" declines what its "Yes" would have changed.
pub fn finished(
    asks: &[Ask],
    answers: &[Answer],
    card: Option<&Card>,
    settings: &AppSettings,
    at_ms: u64,
) -> Setup {
    let mut setup = Setup {
        at_ms,
        answers: answers.to_vec(),
        ..Setup::default()
    };
    for ask in asks.iter().filter(|ask| ask.done) {
        if ask.chosen != ["no"] {
            continue;
        }
        let yes = ask.options.iter().filter(|option| option.id != "no");
        for (key, to) in yes.flat_map(|option| option.changes.iter()) {
            setup.declined.push(Line {
                key: key.clone(),
                to: to.clone(),
                from: now(settings, key),
            });
        }
    }
    let applied = card.filter(|card| {
        card.kind == CardKind::Plan && matches!(card.decision, Decision::Applied { .. })
    });
    for item in applied.iter().flat_map(|card| card.items.iter()) {
        let line = Line {
            key: item.key.clone(),
            to: item.to.clone(),
            from: item.from.clone(),
        };
        if !item.included {
            setup.declined.push(line);
        } else if item.failed.is_none() {
            setup.applied.push(line);
        }
    }
    setup
}

/// What `key` is set to now; `null` for one the settings don't name.
fn now(settings: &AppSettings, key: &str) -> Value {
    changes::current(settings, key).unwrap_or(Value::Null)
}

/// The lines declined and not applied since, oldest setup first.
pub fn declined(setups: &[Setup]) -> Vec<Line> {
    let mut declined: Vec<Line> = Vec::new();
    for setup in setups {
        declined.retain(|line| !setup.applied.iter().any(|done| done.key == line.key));
        for line in &setup.declined {
            if !declined.contains(line) {
                declined.push(line.clone());
            }
        }
    }
    declined
}

/// Whether `key` to `to` was declined and the setting is still as it was
/// then: it isn't offered again.
pub fn held(declined: &[Line], key: &str, to: &Value, settings: &AppSettings) -> bool {
    declined
        .iter()
        .any(|line| line.key == key && line.to == *to && now(settings, key) == line.from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes::Item;
    use crate::assistant::setup::Choice;

    fn no(key: &str) -> Ask {
        Ask {
            id: key.into(),
            options: vec![
                Choice {
                    id: "yes".into(),
                    changes: vec![(key.into(), Value::Bool(true))],
                    ..Choice::default()
                },
                Choice {
                    id: "no".into(),
                    ..Choice::default()
                },
            ],
            chosen: vec!["no".into()],
            done: true,
            ..Ask::default()
        }
    }

    fn row(key: &str, included: bool) -> Item {
        Item {
            key: key.into(),
            from: Value::Bool(false),
            to: Value::Bool(true),
            included,
            ..Item::default()
        }
    }

    /// Applied: ticked rows applied, unticked declined, a "No" declined;
    /// put off, only the "No".
    #[test]
    fn a_finished_setup_keeps_what_it_applied_and_declined() {
        let settings = AppSettings::default();
        let asks = [no("labs.playback")];
        let mut card = Card {
            kind: CardKind::Plan,
            items: vec![row("labs.files", true), row("labs.history_stats", false)],
            decision: Decision::Applied {
                before: Default::default(),
                after: Default::default(),
                at_ms: 5,
            },
            ..Card::default()
        };
        let setup = finished(&asks, &[], Some(&card), &settings, 5);
        let keys = |lines: &[Line]| {
            lines
                .iter()
                .map(|line| line.key.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(&setup.applied), ["labs.files"]);
        assert_eq!(
            keys(&setup.declined),
            ["labs.playback", "labs.history_stats"]
        );
        card.decision = Decision::Declined;
        let put_off = finished(&asks, &[], Some(&card), &settings, 5);
        assert!(put_off.applied.is_empty());
        assert_eq!(keys(&put_off.declined), ["labs.playback"]);
    }

    /// Declined stays declined while the setting is as it was; a later
    /// setup applying it, or the setting changing, lifts it.
    #[test]
    fn a_declined_line_is_held_until_something_about_it_changes() {
        let mut settings = AppSettings::default();
        let line = Line {
            key: "labs.playback".into(),
            to: Value::Bool(true),
            from: Value::Bool(false),
        };
        let first = Setup {
            at_ms: 1,
            declined: vec![line.clone()],
            ..Setup::default()
        };
        let lines = declined(std::slice::from_ref(&first));
        assert!(held(&lines, "labs.playback", &Value::Bool(true), &settings));
        assert!(!held(
            &lines,
            "labs.playback",
            &Value::Bool(false),
            &settings
        ));
        settings.labs.playback = true;
        assert!(!held(
            &lines,
            "labs.playback",
            &Value::Bool(true),
            &settings
        ));
        let second = Setup {
            at_ms: 2,
            applied: vec![line],
            ..Setup::default()
        };
        assert!(declined(&[first, second]).is_empty());
    }
}
