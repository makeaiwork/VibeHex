//! What a card shows now, derived each frame from what was decided and
//! what is true now: the settings (changed by hand since?), the model
//! being downloaded, the Codex sign-in, the OS permissions, and whether
//! Undo still holds this session. Nothing here is stored.

use serde_json::Value;

use super::catalog::{self, Class, Permission, Shorter};
use super::{Card, CardKind, Decision, current, vocab};
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
    /// This OS's and organization's limits apply: false in a scene, which
    /// plays a Mac whatever it renders on.
    pub here: bool,
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
    // A vocabulary card's rows, like a plan's, show the rest.
    if matches!(
        card.kind,
        CardKind::Plan | CardKind::Vocabulary | CardKind::VocabularyEdit
    ) {
        return plan(card, live);
    }
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
            if let Some(blocked) = blocked(card, live.settings).filter(|_| live.here) {
                return blocked;
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

/// A plan as a whole: its rows show the rest (`row`). A row another
/// computer or an organization holds was left out when it was made.
fn plan(card: &Card, live: &Live) -> Shown {
    match &card.decision {
        Decision::Proposed => Shown::Proposed {
            shorter: None,
            also: Vec::new(),
        },
        Decision::Declined => Shown::Declined,
        Decision::Undone => Shown::Undone,
        Decision::Failed { why } => Shown::Failed(why.clone()),
        Decision::Applied { after, .. } => {
            let moved = after
                .iter()
                .find_map(|(key, value)| current(live.settings, key).filter(|now| now != value));
            match moved {
                Some(now) => Shown::ChangedSince { now },
                None => Shown::Applied {
                    undo: live.undoable,
                },
            }
        }
    }
}

/// What one row of a plan shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// Still to decide. `sign_in` while it runs on Codex, signed out;
    /// `shorter` names what a shorter retention removes.
    Open {
        checked: bool,
        sign_in: bool,
        shorter: Option<Shorter>,
    },
    Applied,
    /// Its own check failed, so the save left it out.
    Failed(String),
    /// Not checked, or the plan turned down or put back.
    Left,
    /// A speech model downloading; it switches when done.
    Downloading {
        done: u64,
        total: u64,
    },
}

/// What row `index` of the plan `card` shows with `live`.
pub fn row(card: &Card, index: usize, live: &Live) -> Row {
    let Some(item) = card.items.get(index) else {
        return Row::Left;
    };
    match &card.decision {
        Decision::Proposed => Row::Open {
            checked: item.included,
            sign_in: catalog::needs_codex(&item.key, &item.to)
                && live.codex_signed_in == Some(false),
            shorter: catalog::shorter(&item.key, &item.from, &item.to),
        },
        Decision::Declined | Decision::Undone => Row::Left,
        _ if !item.included => Row::Left,
        Decision::Failed { why } => Row::Failed(item.failed.clone().unwrap_or(why.clone())),
        Decision::Applied { .. } => {
            if let Some(why) = &item.failed {
                return Row::Failed(why.clone());
            }
            match downloading(item.class, &item.key, &item.to, live) {
                Some(Shown::Downloading { done, total }) => Row::Downloading { done, total },
                _ => Row::Applied,
            }
        }
    }
}

/// A plan's Vocabulary terms: one row to look at, however many they are
/// (the owner's preset brings 95); the plan counts them apart from its
/// settings (`counts`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Terms {
    /// Their rows in the plan, in order.
    pub rows: Vec<usize>,
    /// How many of them are checked.
    pub checked: usize,
}

impl Terms {
    /// With this many terms or fewer the row starts unfolded.
    pub const FEW: usize = 5;

    /// The terms `card` adds; none when it adds no term.
    pub fn of(card: &Card) -> Option<Self> {
        let rows: Vec<usize> = card
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.key == vocab::KEY)
            .map(|(index, _)| index)
            .collect();
        let checked = rows
            .iter()
            .filter(|index| card.items[**index].included)
            .count();
        (!rows.is_empty()).then_some(Self { rows, checked })
    }

    /// Unfolded before anyone presses: few enough to read at a glance, or
    /// one of them failed and says why.
    pub fn starts_open(&self, card: &Card) -> bool {
        self.rows.len() <= Self::FEW
            || self
                .rows
                .iter()
                .any(|index| card.items[*index].included && card.items[*index].failed.is_some())
    }

    /// What the one row shows: checked while any term is, and after the
    /// save how the checked ones went.
    pub fn row(&self, card: &Card, live: &Live) -> Row {
        let rows: Vec<Row> = self
            .rows
            .iter()
            .map(|index| row(card, *index, live))
            .collect();
        if let Some(Row::Open { .. }) = rows.first() {
            return Row::Open {
                checked: self.checked > 0,
                sign_in: false,
                shorter: None,
            };
        }
        if rows.contains(&Row::Applied) {
            Row::Applied
        } else if let Some(failed) = rows.iter().find(|row| matches!(row, Row::Failed(_))) {
            failed.clone()
        } else {
            Row::Left
        }
    }
}

/// How many of a card's checked rows its save applied, of how many.
pub fn applied_count(card: &Card) -> (usize, usize) {
    // A kept row whose companion was picked anew is one change too.
    let kept = card
        .kept
        .iter()
        .filter(|item| item.companion_changed().is_some());
    let checked = card.items.iter().filter(|item| item.included).chain(kept);
    let total = checked.clone().count();
    let failed = checked.filter(|item| item.failed.is_some()).count();
    (total - failed, total)
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
            here: true,
        }
    }

    #[test]
    fn a_card_follows_its_decision_and_the_settings() {
        let mut settings = AppSettings::default();
        settings.labs.spoken_formatting = false;
        let mut smart = card("labs.spoken_formatting", json!(false), json!(true));
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
        settings.labs.spoken_formatting = true;
        assert_eq!(shown(&smart, &live(&settings)), Shown::Already);
        // Applied, then Undo for as long as the session holds it.
        smart.decision = applied(
            vec![("labs.spoken_formatting".into(), json!(false), json!(true))],
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
        settings.labs.spoken_formatting = false;
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
    fn a_plans_rows_follow_its_save() {
        let settings = AppSettings::default();
        let mut plan = card("labs.files", json!(false), json!(true));
        plan.kind = CardKind::Plan;
        plan.items.push(Item {
            key: "dictation_processing.server.runs_on".into(),
            from: json!("opencode"),
            to: json!("codex"),
            included: false,
            ..Item::default()
        });
        plan.items.push(Item {
            key: "transcription.model".into(),
            from: json!("parakeet_v3"),
            to: json!("gigaam_v3"),
            included: true,
            class: Class::Download,
            ..Item::default()
        });
        let signed_out = Live {
            codex_signed_in: Some(false),
            ..live(&settings)
        };
        assert_eq!(
            row(&plan, 1, &signed_out),
            Row::Open {
                checked: false,
                sign_in: true,
                shorter: None
            }
        );
        plan.items[0].failed = Some("no".into());
        plan.decision = applied(Vec::new(), 0);
        let downloading = Live {
            preparing: Some(("gigaam_v3".into(), 1, 2)),
            ..live(&settings)
        };
        assert_eq!(row(&plan, 0, &downloading), Row::Failed("no".into()));
        assert_eq!(row(&plan, 1, &downloading), Row::Left);
        assert_eq!(
            row(&plan, 2, &downloading),
            Row::Downloading { done: 1, total: 2 }
        );
        assert_eq!(row(&plan, 2, &live(&settings)), Row::Applied);
        assert_eq!(applied_count(&plan), (1, 2));
        assert_eq!(
            shown(&plan, &live(&settings)),
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

    /// A plan of `settings` setting rows and `terms` term rows, each as
    /// `(checked, failed)`.
    fn plan_of(settings: &[(bool, bool)], terms: &[(bool, bool)]) -> Card {
        let item = |key: &str, (included, failed): (bool, bool)| Item {
            key: key.into(),
            to: json!(true),
            included,
            failed: failed.then(|| "no".to_owned()),
            ..Item::default()
        };
        Card {
            kind: CardKind::Plan,
            items: settings
                .iter()
                .map(|row| item("labs.smart_spacing", *row))
                .chain(terms.iter().map(|row| item(vocab::KEY, *row)))
                .collect(),
            ..Card::default()
        }
    }

    /// A vocabulary card's footer counts row by row, terms and all.
    #[test]
    fn a_cards_applied_rows_are_counted_one_by_one() {
        const ON: (bool, bool) = (true, false);
        const FAILED: (bool, bool) = (true, true);
        assert_eq!(applied_count(&plan_of(&[ON], &[ON, FAILED])), (2, 3));
    }

    #[test]
    fn a_plans_terms_fold_above_a_handful_and_show_how_they_went() {
        const ON: (bool, bool) = (true, false);
        const OFF: (bool, bool) = (false, false);
        const FAILED: (bool, bool) = (true, true);
        let settings = AppSettings::default();
        let live = live(&settings);
        assert_eq!(Terms::of(&plan_of(&[ON], &[])), None);

        let few = plan_of(&[ON], &[ON; Terms::FEW]);
        let terms = Terms::of(&few).unwrap();
        assert_eq!(
            (terms.rows.clone(), terms.checked),
            (vec![1, 2, 3, 4, 5], 5)
        );
        assert!(terms.starts_open(&few), "a handful starts unfolded");

        let mut many = plan_of(&[ON], &[ON, OFF, ON, ON, ON, ON]);
        let terms = Terms::of(&many).unwrap();
        assert_eq!(terms.checked, 5);
        assert!(!terms.starts_open(&many), "six start folded");
        // Checked while any term is, and not once none is.
        let open = |checked| Row::Open {
            checked,
            sign_in: false,
            shorter: None,
        };
        assert_eq!(terms.row(&many, &live), open(true));
        let none = plan_of(&[ON], &[OFF; 6]);
        assert_eq!(Terms::of(&none).unwrap().row(&none, &live), open(false));

        // Saved: applied when any went in, and unfolded by a failure.
        let saved = |card: &mut Card| {
            card.decision = Decision::Applied {
                before: Default::default(),
                after: Default::default(),
                at_ms: 1,
            };
        };
        saved(&mut many);
        assert_eq!(terms.row(&many, &live), Row::Applied);
        let mut mixed = plan_of(&[ON], &[ON, FAILED, ON, ON, ON, ON]);
        saved(&mut mixed);
        let terms = Terms::of(&mixed).unwrap();
        assert_eq!(terms.row(&mixed, &live), Row::Applied);
        assert!(terms.starts_open(&mixed), "a failure says why, unfolded");
        let mut failed = plan_of(&[ON], &[FAILED; 6]);
        saved(&mut failed);
        let terms = Terms::of(&failed).unwrap();
        assert_eq!(terms.row(&failed, &live), Row::Failed("no".into()));
        let mut left = plan_of(&[ON], &[OFF; 6]);
        saved(&mut left);
        assert_eq!(Terms::of(&left).unwrap().row(&left, &live), Row::Left);
    }
}
