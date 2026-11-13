//! Dictation cleanup on this computer (Labs › Clean up on your own server ›
//! Built in), as a card or a plan row offers it. `runs_on` going to
//! `local` carries the model it will run as its companion
//! (`companion::Companion` keyed [`KEY`]), picked in the row among
//! [`local_models::MODELS`]; the save writes both, and the window then
//! downloads the model if it isn't on disk yet.
//!
//! What each model costs here is [`fit`]: its download against the free
//! disk where the models go, and what it takes while it runs
//! ([`LocalModel::memory`]) against the memory installed. One that takes
//! more than half the memory may slow the computer, so it is marked and
//! never preselected. The row starts at a model already on disk that fits,
//! else the recommended one if it fits, else the next that does
//! ([`preselect`]).

use serde_json::Value;

use super::companion::Companion;
use super::{Card, CardKind, Item, current};
use crate::app_settings::{AppSettings, CleanupRunsOn};
use crate::dictation::cleanup_server::local_models::{self, LocalModel, MODELS};

/// The built-in model saved for cleanup.
pub const KEY: &str = "dictation_processing.server.local_model";
/// Where dictation is cleaned up.
pub const RUNS_ON: &str = "dictation_processing.server.runs_on";

/// What this computer has for a built-in model: the memory installed, the
/// disk free where the models go, and the models on disk. Each is `None`
/// (or empty) while unknown, and an unknown one warns of nothing.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Machine {
    pub memory: Option<u64>,
    pub free: Option<u64>,
    pub installed: Vec<&'static str>,
}

impl Machine {
    /// This computer now. Cheap: the memory is read once, the disk and
    /// the models' files each time.
    pub fn here() -> Self {
        Self {
            memory: crate::desktop::hardware::memory(),
            free: free_space(),
            installed: MODELS
                .iter()
                .filter(|model| model.installed())
                .map(|model| model.id)
                .collect(),
        }
    }

    pub fn has(&self, model: &LocalModel) -> bool {
        self.installed.contains(&model.id)
    }
}

/// The disk free where the models go: the folder's, or the nearest one
/// above it that exists yet.
fn free_space() -> Option<u64> {
    let dir = local_models::dir().ok()?;
    let existing = dir.ancestors().find(|path| path.exists())?;
    fs2::available_space(existing).ok()
}

/// What a model costs on a computer, each `Some` only when it is too much.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Fit {
    /// It takes more than half the memory: (what it takes, the memory).
    pub memory: Option<(u64, u64)>,
    /// Its download doesn't fit the free disk: (its size, what is free).
    pub disk: Option<(u64, u64)>,
}

impl Fit {
    pub fn fits(self) -> bool {
        self.memory.is_none() && self.disk.is_none()
    }
}

/// What `model` costs on `machine`. A model on disk needs no room; the
/// whole file is counted, even when part of it came already.
pub fn fit(model: &LocalModel, machine: &Machine) -> Fit {
    let takes = model.memory();
    Fit {
        memory: machine
            .memory
            .filter(|total| takes > total / 2)
            .map(|total| (takes, total)),
        disk: machine
            .free
            .filter(|free| !machine.has(model) && model.bytes > *free)
            .map(|free| (model.bytes, free)),
    }
}

/// Whether every model takes more than half of this computer's memory:
/// cleanup in the cloud suits it better.
pub fn none_fits(machine: &Machine) -> bool {
    MODELS
        .iter()
        .all(|model| fit(model, machine).memory.is_some())
}

/// Which model a row starts at: `wanted` (the one saved, or the preset's)
/// while on disk and fitting, else another on disk that fits, else
/// `wanted` while it fits and isn't the saved default, else the first
/// that fits in [`MODELS`]' order (the recommended one first), else the
/// one that takes the least.
pub fn preselect(machine: &Machine, wanted: Option<&str>, prefer_wanted: bool) -> usize {
    let fits = |model: &LocalModel| fit(model, machine).fits();
    let wanted = wanted.and_then(local_models::find);
    let at = |model: &LocalModel| MODELS.iter().position(|known| known.id == model.id);
    if let Some(model) = wanted.filter(|model| machine.has(model) && fits(model)) {
        return at(model).unwrap_or(0);
    }
    if let Some(index) = MODELS
        .iter()
        .position(|model| machine.has(model) && fits(model))
    {
        return index;
    }
    if let Some(model) = wanted.filter(|model| prefer_wanted && fits(model)) {
        return at(model).unwrap_or(0);
    }
    MODELS.iter().position(fits).unwrap_or_else(|| {
        MODELS
            .iter()
            .enumerate()
            .min_by_key(|(_, model)| model.memory())
            .map_or(0, |(index, _)| index)
    })
}

/// The model choice that goes with `key` going to `to`: only dictation's
/// cleanup going to this computer.
pub fn companion(
    settings: &AppSettings,
    machine: &Machine,
    key: &str,
    to: &Value,
) -> Option<Companion> {
    if key != RUNS_ON || to.as_str() != Some("local") {
        return None;
    }
    let from = current(settings, KEY)?;
    let saved = from.as_str().filter(|id| !id.trim().is_empty());
    let chosen = preselect(machine, saved, false);
    Some(Companion {
        key: KEY.to_owned(),
        from,
        options: MODELS.iter().map(|model| Value::from(model.id)).collect(),
        chosen,
    })
}

/// The model choice an item carries, if it is one.
pub fn choice(item: &Item) -> Option<&Companion> {
    item.companion
        .as_ref()
        .filter(|companion| companion.key == KEY)
}

/// The model a saved `item` asks for, which the window downloads if it
/// isn't on disk: cleanup going to this computer (its choice, else the
/// model saved), or the model itself while cleanup runs here (`settings`
/// as they are once saved).
pub fn target(item: &Item, settings: &AppSettings) -> Option<&'static LocalModel> {
    match item.key.as_str() {
        RUNS_ON if item.to.as_str() == Some("local") => Some(match choice(item) {
            Some(companion) => local_models::chosen(companion.to().as_str().unwrap_or_default()),
            None => local_models::chosen(&settings.dictation_processing.server.local_model),
        }),
        KEY if settings.dictation_processing.server.runs_on == CleanupRunsOn::Local => {
            local_models::find(item.to.as_str()?)
        }
        _ => None,
    }
}

/// One row for cleanup on this computer and its model: a model line beside
/// a `runs_on` line with its choice (a model's two lines, the preset's)
/// becomes that choice, `model` when it fits, and goes. `kept` loses its
/// model line too: the row names the model.
pub fn fold(items: &mut Vec<Item>, kept: &mut Vec<Item>, model: Option<&str>, machine: &Machine) {
    if !items.iter().any(|item| choice(item).is_some()) {
        return;
    }
    let wanted = model.map(str::to_owned).or_else(|| {
        items
            .iter()
            .find(|item| item.key == KEY)
            .and_then(|item| item.to.as_str())
            .map(str::to_owned)
    });
    let Some(wanted) = wanted else {
        return;
    };
    if let Some(companion) = items.iter_mut().find_map(choice_mut) {
        companion.chosen = preselect(machine, Some(&wanted), true);
    }
    items.retain(|item| item.key != KEY);
    kept.retain(|item| item.key != KEY);
}

fn choice_mut(item: &mut Item) -> Option<&mut Companion> {
    item.companion
        .as_mut()
        .filter(|companion| companion.key == KEY)
}

/// [`fold`] over an answer's cards: a model's card beside one moving
/// cleanup to this computer, or both lines in one plan.
pub fn fold_cards(cards: &mut Vec<Card>, machine: &Machine) {
    for card in cards.iter_mut().filter(|card| card.kind == CardKind::Plan) {
        fold(&mut card.items, &mut card.kept, None, machine);
    }
    let has_choice = cards.iter().any(|card| {
        card.kind == CardKind::Single && card.items.iter().any(|i| choice(i).is_some())
    });
    let model = cards
        .iter()
        .filter(|card| card.kind == CardKind::Single)
        .flat_map(|card| &card.items)
        .find(|item| item.key == KEY)
        .and_then(|item| item.to.as_str())
        .map(str::to_owned);
    let (true, Some(model)) = (has_choice, model) else {
        return;
    };
    for card in cards
        .iter_mut()
        .filter(|card| card.kind == CardKind::Single)
    {
        if let Some(companion) = card.items.iter_mut().find_map(choice_mut) {
            companion.chosen = preselect(machine, Some(&model), true);
        }
    }
    cards.retain(|card| {
        !(card.kind == CardKind::Single && card.items.iter().any(|item| item.key == KEY))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const GB: u64 = 1 << 30;

    fn machine(memory: u64, installed: &[&'static str]) -> Machine {
        Machine {
            memory: Some(memory * GB),
            free: None,
            installed: installed.to_vec(),
        }
    }

    /// A model taking more than half the memory is marked and never
    /// preselected; on 8 GB only Gemma 4 E4B is, on 4 GB all three.
    #[test]
    fn a_model_too_big_for_the_memory_is_marked() {
        let eight = machine(8, &[]);
        let heavy: Vec<&str> = MODELS
            .iter()
            .filter(|model| fit(model, &eight).memory.is_some())
            .map(|model| model.id)
            .collect();
        assert_eq!(heavy, ["gemma-4-e4b-it"]);
        assert!(!none_fits(&eight));
        assert!(none_fits(&machine(4, &[])));
        assert!(
            MODELS
                .iter()
                .all(|model| fit(model, &machine(16, &[])).fits())
        );
        // Unknown memory warns of nothing.
        assert!(fit(&MODELS[1], &Machine::default()).fits());
    }

    /// A download that doesn't fit the free disk is marked; one on disk
    /// needs no room.
    #[test]
    fn a_download_too_big_for_the_disk_is_marked() {
        let mut low = machine(16, &[]);
        low.free = Some(3_000_000_000);
        assert_eq!(
            fit(&MODELS[1], &low).disk,
            Some((MODELS[1].bytes, 3_000_000_000))
        );
        assert!(fit(&MODELS[0], &low).fits());
        low.installed = vec![MODELS[1].id];
        assert!(fit(&MODELS[1], &low).disk.is_none());
    }

    /// On disk and fitting first, then the recommended one, then the next
    /// that fits; with none fitting, the one that takes least.
    #[test]
    fn the_row_starts_at_a_model_on_disk_else_the_recommended() {
        assert_eq!(preselect(&machine(16, &[]), None, false), 0);
        let e2b = MODELS
            .iter()
            .position(|model| model.id == "gemma-4-e2b-it")
            .unwrap();
        let e4b = MODELS
            .iter()
            .position(|model| model.id == "gemma-4-e4b-it")
            .unwrap();
        assert_eq!(
            preselect(&machine(16, &["gemma-4-e2b-it"]), None, false),
            e2b
        );
        // On disk but too big here: not preselected.
        assert_eq!(preselect(&machine(8, &["gemma-4-e4b-it"]), None, false), 0);
        // The saved one on disk wins over another on disk.
        let both = machine(16, &["gemma-4-e2b-it", "gemma-4-e4b-it"]);
        assert_eq!(preselect(&both, Some("gemma-4-e4b-it"), false), e4b);
        // The preset's model while it fits, else the rule.
        assert_eq!(
            preselect(&machine(16, &[]), Some("gemma-4-e4b-it"), true),
            e4b
        );
        assert_eq!(preselect(&machine(8, &[]), Some("gemma-4-e4b-it"), true), 0);
        assert_eq!(preselect(&machine(4, &[]), None, false), e2b);
    }

    #[test]
    fn only_cleanup_on_this_computer_carries_the_choice() {
        let settings = AppSettings::default();
        let here = machine(16, &[]);
        let choice = companion(&settings, &here, RUNS_ON, &json!("local")).unwrap();
        assert_eq!(choice.key, KEY);
        assert_eq!(choice.options.len(), MODELS.len());
        assert_eq!(*choice.to(), json!(MODELS[0].id));
        assert!(companion(&settings, &here, RUNS_ON, &json!("codex")).is_none());
        assert!(companion(&settings, &here, "labs.files", &json!("local")).is_none());
    }

    /// The model a save asks for: the row's choice; a model line only while
    /// cleanup runs here.
    #[test]
    fn a_saved_row_names_the_model_to_download() {
        let mut settings = AppSettings::default();
        let mut runs_on = Item {
            key: RUNS_ON.into(),
            to: json!("local"),
            companion: companion(&settings, &machine(16, &[]), RUNS_ON, &json!("local")),
            ..Item::default()
        };
        runs_on.companion.as_mut().unwrap().chosen = 2;
        assert_eq!(target(&runs_on, &settings), Some(&MODELS[2]));
        let model = Item {
            key: KEY.into(),
            to: json!(MODELS[1].id),
            ..Item::default()
        };
        assert_eq!(target(&model, &settings), None);
        settings.dictation_processing.server.runs_on = CleanupRunsOn::Local;
        assert_eq!(target(&model, &settings), Some(&MODELS[1]));
    }

    /// The preset's model line becomes the row's choice and goes.
    #[test]
    fn a_model_line_folds_into_the_rows_choice() {
        let settings = AppSettings::default();
        let here = machine(16, &[]);
        let mut items = vec![
            Item {
                key: RUNS_ON.into(),
                to: json!("local"),
                companion: companion(&settings, &here, RUNS_ON, &json!("local")),
                ..Item::default()
            },
            Item {
                key: KEY.into(),
                to: json!("gemma-4-e4b-it"),
                ..Item::default()
            },
        ];
        let mut kept = Vec::new();
        fold(&mut items, &mut kept, None, &here);
        assert_eq!(items.len(), 1);
        assert_eq!(*choice(&items[0]).unwrap().to(), json!("gemma-4-e4b-it"));
        // Two cards of an answer: the same.
        let card = |item: Item| Card {
            items: vec![item],
            ..Card::default()
        };
        let mut cards = vec![
            card(Item {
                key: RUNS_ON.into(),
                to: json!("local"),
                companion: companion(&settings, &here, RUNS_ON, &json!("local")),
                ..Item::default()
            }),
            card(Item {
                key: KEY.into(),
                to: json!("gemma-4-e2b-it"),
                ..Item::default()
            }),
        ];
        fold_cards(&mut cards, &here);
        assert_eq!(cards.len(), 1);
        assert_eq!(
            *choice(&cards[0].items[0]).unwrap().to(),
            json!("gemma-4-e2b-it")
        );
    }

    /// Setup's "On this Mac" makes the `runs_on` row carry the models, at
    /// the one on disk; nothing else names the model.
    #[test]
    fn setups_plan_offers_the_models_in_its_row() {
        use crate::assistant::facts::State;
        use crate::assistant::setup::{Answer, plan};
        let answer = |id: &str, chosen: &str| Answer {
            id: id.into(),
            chosen: vec![chosen.into()],
            changes: Vec::new(),
        };
        let answers = [
            answer("use", "dictation_any"),
            answer("language", "en"),
            answer("cleanup", "local"),
        ];
        let state = State {
            os: "macos",
            machine: machine(16, &["gemma-4-e2b-it"]),
            ..State::default()
        };
        let card =
            plan::with_terms(Vec::new(), &answers, &[], &AppSettings::default(), &state).unwrap();
        let row = card.items.iter().find(|item| item.key == RUNS_ON).unwrap();
        assert_eq!(*choice(row).unwrap().to(), json!("gemma-4-e2b-it"));
        assert!(card.items.iter().all(|item| item.key != KEY));
    }

    /// Denis's preset: one row, at his model while it fits, else the rule.
    #[test]
    fn the_presets_row_starts_at_denis_model_while_it_fits() {
        use crate::assistant::facts::State;
        use crate::assistant::setup::preset;
        let wanted = preset::denis()
            .settings
            .get(KEY)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap();
        for (memory, at) in [(16, wanted.as_str()), (4, "gemma-4-e2b-it")] {
            let state = State {
                os: "macos",
                machine: machine(memory, &[]),
                ..State::default()
            };
            let planned = preset::plan(&preset::denis(), &AppSettings::default(), &state);
            let card = planned.card.unwrap();
            assert!(card.items.iter().all(|item| item.key != KEY));
            let row = card.items.iter().find(|item| item.key == RUNS_ON).unwrap();
            let to = choice(row).unwrap().to().clone();
            assert_eq!(to, json!(at), "{memory} GB");
        }
    }
}
