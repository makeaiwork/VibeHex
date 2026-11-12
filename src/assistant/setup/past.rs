//! The setups finished before, kept with the conversations
//! (`store::Store::setups`): when, the answers, and what the plan applied
//! or the person declined. A returning setup starts from them, and a line
//! once declined (a row left unticked, a follow-up's "No") isn't offered
//! again while the setting is still as it was then; once it changed, by
//! hand or by another plan, it may be.
//!
//! A setup is kept from its first answered round on, so a conversation
//! left half-way, or a plan put off, still keeps its answers; a later
//! round or the plan's decision updates the same record. A follow-up
//! answered there, Yes or No, isn't asked again while what its Yes would
//! change is still as it was then and no release since names it
//! (`answered_before`).

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
    /// The follow-ups answered, as asked, with their settings then.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asked: Vec<Answered>,
    /// The conversation it ran in and when its round one was asked: the
    /// record a later round or the plan updates (0 in an older record).
    #[serde(skip_serializing_if = "is_zero")]
    pub conversation: u32,
    #[serde(skip_serializing_if = "is_zero")]
    pub started_ms: u64,
    /// The plan turn it was decided on; 0 until a plan is.
    #[serde(skip_serializing_if = "is_zero")]
    pub plan_ms: u64,
}

fn is_zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

impl Setup {
    /// Whether `other` is the same setup, kept again.
    pub fn same(&self, other: &Setup) -> bool {
        self.conversation != 0
            && self.conversation == other.conversation
            && self.started_ms == other.started_ms
    }
}

/// A follow-up answered: the question as asked with its answer, when, and
/// what each setting its Yes changes was set to then.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Answered {
    pub ask: Ask,
    pub at_ms: u64,
    pub then: Vec<Line>,
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
    for ask in asks.iter().filter(|ask| ask.done && !ask.chosen.is_empty()) {
        if !ask.options.iter().any(|option| option.id == "no") {
            continue;
        }
        let yes = ask.options.iter().filter(|option| option.id != "no");
        setup.asked.push(Answered {
            ask: Ask {
                before: false,
                touched: false,
                ..ask.clone()
            },
            at_ms,
            then: yes
                .flat_map(|option| option.changes.iter())
                .map(|(key, to)| Line {
                    key: key.clone(),
                    to: to.clone(),
                    from: now(settings, key),
                })
                .collect(),
        });
    }
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

/// `setup` as it updates `old`, the same setup kept before: a follow-up
/// answered as it was then keeps when, and the settings it saw then
/// (before the plan changed them).
pub fn merge(old: &Setup, mut setup: Setup) -> Setup {
    for answered in &mut setup.asked {
        let kept = old.asked.iter().find(|known| {
            known.ask.id == answered.ask.id && known.ask.chosen == answered.ask.chosen
        });
        if let Some(kept) = kept {
            answered.at_ms = kept.at_ms;
            answered.then = kept.then.clone();
        }
    }
    // A follow-up's "No" declined then, from the setting it saw then.
    for line in &mut setup.declined {
        let then = old
            .declined
            .iter()
            .find(|known| known.key == line.key && known.to == line.to);
        if let Some(then) = then {
            line.from = then.from.clone();
        }
    }
    setup
}

/// The follow-ups answered in `setups`, the newest answer to each, that
/// aren't asked again: what their Yes changes is as it was then and no
/// release since changed it (`changed(key, at_ms)`). Each comes as asked,
/// with its answer, `before`.
pub fn answered_before(
    setups: &[Setup],
    settings: &AppSettings,
    changed: impl Fn(&str, u64) -> bool,
) -> Vec<Ask> {
    let mut newest: Vec<Answered> = Vec::new();
    for setup in setups {
        let asked = if setup.asked.is_empty() {
            older(setup)
        } else {
            setup.asked.clone()
        };
        for answered in asked {
            newest.retain(|known| known.ask.id != answered.ask.id);
            newest.push(answered);
        }
    }
    newest
        .into_iter()
        .filter(|answered| {
            answered.then.iter().all(|line| {
                now(settings, &line.key) == line.from && !changed(&line.key, answered.at_ms)
            })
        })
        .map(|answered| Ask {
            before: true,
            done: false,
            ..answered.ask
        })
        .collect()
}

/// The follow-ups an older record answered, kept only as its answers and
/// lines: each Labs switch's Yes or No as setup asks it, seen as it was
/// before (the line's `from`, else off).
fn older(setup: &Setup) -> Vec<Answered> {
    let articles = crate::assistant::guide::articles();
    setup
        .answers
        .iter()
        .filter(|answer| answer.chosen == ["yes"] || answer.chosen == ["no"])
        .filter_map(|answer| {
            let key = answer.id.as_str();
            let article = articles.iter().find(|article| {
                article.is_lab() && article.keys.first().is_some_and(|k| k == key)
            })?;
            let line = setup
                .declined
                .iter()
                .chain(&setup.applied)
                .find(|line| line.key == key);
            let from = line.map_or(Value::Bool(false), |line| line.from.clone());
            let mut ask = super::follow::yes_no(key, &article.ask, &article.title);
            ask.chosen = answer.chosen.clone();
            ask.done = true;
            Some(Answered {
                ask,
                at_ms: setup.at_ms,
                then: vec![Line {
                    key: key.to_owned(),
                    to: Value::Bool(true),
                    from,
                }],
            })
        })
        .collect()
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

    fn yes(key: &str) -> Ask {
        Ask {
            chosen: vec!["yes".into()],
            ..no(key)
        }
    }

    /// A follow-up answered, Yes or No, in a setup left half-way or put
    /// off, isn't asked again; until its setting changes by hand, or a
    /// release since changes the feature.
    #[test]
    fn an_answered_follow_up_holds_until_its_setting_or_a_release_changes() {
        let mut settings = AppSettings::default();
        settings.labs.playback = false;
        settings.labs.files = false;
        let asks = [no("labs.playback"), yes("labs.files")];
        // Left after the round: no plan at all.
        let left = finished(&asks, &[], None, &settings, 100);
        // Put off: the plan declined, its rows neither applied nor declined.
        let card = Card {
            kind: CardKind::Plan,
            items: vec![row("labs.files", false)],
            decision: Decision::Declined,
            ..Card::default()
        };
        let put_off = finished(&asks, &[], Some(&card), &settings, 100);
        assert_eq!(put_off.asked, left.asked);
        assert!(put_off.applied.is_empty());
        let ids =
            |setups: &[Setup], settings: &AppSettings, changed: &dyn Fn(&str, u64) -> bool| {
                answered_before(setups, settings, changed)
                    .into_iter()
                    .map(|ask| (ask.id, ask.chosen, ask.before))
                    .collect::<Vec<_>>()
            };
        let never = |_: &str, _: u64| false;
        for setups in [vec![left.clone()], vec![put_off]] {
            assert_eq!(
                ids(&setups, &settings, &never),
                [
                    ("labs.playback".into(), vec!["no".to_owned()], true),
                    ("labs.files".into(), vec!["yes".to_owned()], true),
                ]
            );
        }
        // A release after the answer changed Files: asked again.
        let files = |key: &str, at: u64| key == "labs.files" && at == 100;
        assert_eq!(ids(std::slice::from_ref(&left), &settings, &files).len(), 1);
        // Playback turned on and off again by hand is as it was; turned
        // on, it is asked again (and then isn't, being on).
        settings.labs.playback = true;
        assert_eq!(
            ids(std::slice::from_ref(&left), &settings, &never),
            [("labs.files".into(), vec!["yes".to_owned()], true)]
        );
        // A later setup's answer is the one kept, its settings then too.
        let later = finished(&[yes("labs.playback")], &[], None, &settings, 200);
        assert_eq!(
            ids(&[left.clone(), later], &settings, &never),
            [
                ("labs.files".into(), vec!["yes".to_owned()], true),
                ("labs.playback".into(), vec!["yes".to_owned()], true),
            ]
        );
    }

    /// The same setup kept again (its plan applied after its rounds)
    /// keeps when each follow-up was answered and the settings then.
    #[test]
    fn a_setup_kept_again_keeps_its_answers_then() {
        let mut settings = AppSettings::default();
        let asks = [yes("labs.files")];
        let mut first = finished(&asks, &[], None, &settings, 100);
        first.conversation = 3;
        first.started_ms = 50;
        settings.labs.files = true;
        let mut again = finished(&asks, &[], None, &settings, 300);
        again.conversation = 3;
        again.started_ms = 50;
        assert!(first.same(&again));
        let merged = merge(&first, again);
        assert_eq!(merged.asked[0].at_ms, 100);
        assert_eq!(merged.asked[0].then[0].from, Value::Bool(false));
    }

    /// An older record, with no follow-ups kept as asked, still holds its
    /// answers: a No while the switch is as it was, a Yes the plan applied
    /// isn't asked (it's on).
    #[test]
    fn an_older_records_answers_hold_too() {
        let mut settings = AppSettings::default();
        settings.labs.stop_shortcut = false;
        let answer = |id: &str, chosen: &str| Answer {
            id: id.into(),
            chosen: vec![chosen.into()],
            ..Answer::default()
        };
        let old = Setup {
            at_ms: 7,
            answers: vec![
                answer("use", "meetings"),
                answer("labs.stop_shortcut", "no"),
            ],
            declined: vec![Line {
                key: "labs.stop_shortcut".into(),
                to: Value::Bool(true),
                from: Value::Bool(false),
            }],
            ..Setup::default()
        };
        let held = answered_before(std::slice::from_ref(&old), &settings, |_, _| false);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].id, "labs.stop_shortcut");
        assert_eq!(held[0].chosen, ["no"]);
        assert!(held[0].before && held[0].own && !held[0].q.is_empty());
        settings.labs.stop_shortcut = true;
        assert!(answered_before(&[old], &settings, |_, _| false).is_empty());
    }

    /// The answers a real store holds back from being asked again, for a
    /// local check: `HEX_SETUPS` (conversations.json), `HEX_USAGE_SETTINGS`.
    #[test]
    #[ignore]
    fn owners_answers_before() {
        let read = |name: &str| std::fs::read(std::env::var(name).unwrap()).unwrap();
        let store: serde_json::Value = serde_json::from_slice(&read("HEX_SETUPS")).unwrap();
        let setups: Vec<Setup> = serde_json::from_value(store["setups"].clone()).unwrap();
        let path = std::env::var("HEX_USAGE_SETTINGS").unwrap();
        let (settings, _) = AppSettings::read_from(std::path::Path::new(&path)).unwrap();
        let changed = crate::assistant::setup::since::changed_since;
        for ask in answered_before(&setups, &settings, changed) {
            println!("held: {} {:?}", ask.id, ask.chosen);
        }
    }
}
