//! Cleanup on this computer as scenes (`built_in`): setup's plan after
//! "On this Mac", made by the app's own rules on a fresh install, with
//! the built-in models to pick from on a 16 GB Mac with none downloaded;
//! the same on an 8 GB Mac with Gemma 4 E2B downloaded and little disk
//! left; the plan applied with its model downloading; and a card in an
//! answer moving cleanup to this Mac. The memory and the disk are the
//! scene's, never this computer's.

use serde_json::json;

use super::super::Ui;
use super::changes::At;
use super::fixture::{AT, MINUTE, open};
use super::fixture_cards::conversation;
use crate::assistant::changes::built_in::{self, Machine};
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::{self, Card};
use crate::assistant::facts::{Fact, State};
use crate::assistant::offer::{Gate, Origin};
use crate::assistant::setup;
use crate::assistant::store::Turn;
use crate::desktop::i18n::tr;

const GB: u64 = 1 << 30;

/// A Mac with `memory` GB, `free` bytes of disk, and `installed` on disk.
fn machine(memory: u64, free: Option<u64>, installed: &[&'static str]) -> Machine {
    Machine {
        memory: Some(memory * GB),
        free,
        installed: installed.to_vec(),
    }
}

/// The state a scene plays: a Mac, Codex signed in, and `machine`.
fn state(machine: Machine) -> State {
    State {
        os: "macos",
        codex_signed_in: Fact::Known(true),
        opencode_running: Fact::Known(false),
        openrouter_key: Fact::Known(false),
        machine,
        ..State::default()
    }
}

/// Round one answered: dictation into any app, in English, cleaned up on
/// this Mac.
fn asks() -> Vec<setup::Ask> {
    let mut asks = setup::questions("macos");
    for ask in &mut asks {
        let chosen = match ask.id.as_str() {
            "use" => "dictation_any",
            "language" => "en",
            "cleanup" => "local",
            _ => continue,
        };
        ask.chosen = vec![chosen.into()];
        ask.done = true;
    }
    asks.retain(|ask| ask.done);
    asks
}

/// The plan the app's rules make for those answers on `machine`.
fn planned(ui: &mut Ui, machine: Machine) -> (Vec<setup::Ask>, Card) {
    open(ui);
    ui.platform.system_audio = true;
    ui.assistant.machine = machine.clone();
    let asks = asks();
    let answers = setup::answers(&asks);
    let card = setup::plan::with_terms(Vec::new(), &answers, &[], &ui.settings, &state(machine))
        .expect("a plan");
    (asks, card)
}

/// The conversation: round one answered, then the plan.
fn show(ui: &mut Ui, asks: Vec<setup::Ask>, card: Card) -> u32 {
    let answered = Turn {
        asks: asks.clone(),
        created_ms: AT,
        ..Turn::default()
    };
    let plan = Turn {
        answer: tr("Here is what fits how you work. Untick what you don't need.").into(),
        answers: setup::answers(&asks),
        cards: vec![card],
        created_ms: AT + MINUTE,
        ..Turn::default()
    };
    conversation(ui, "Set up for me", vec![answered, plan], &[])
}

/// A 16 GB Mac with no built-in model: the recommended one picked, and
/// "Needs a download 2.5 GB".
pub(in super::super) fn setup_local_model(ui: &mut Ui) {
    let (asks, card) = planned(ui, machine(16, None, &[]));
    show(ui, asks, card);
}

/// An 8 GB Mac with Gemma 4 E2B downloaded and 4 GB of disk free: E2B
/// picked, E4B too big for its memory and its disk.
pub(in super::super) fn setup_local_model_low(ui: &mut Ui) {
    let (asks, card) = planned(ui, machine(8, Some(4_000_000_000), &["gemma-4-e2b-it"]));
    show(ui, asks, card);
}

/// The plan applied: its model downloading, two fifths in, with Cancel.
pub(in super::super) fn setup_local_model_downloading(ui: &mut Ui) {
    let (asks, mut card) = planned(ui, machine(16, None, &[]));
    let before = ui.settings.clone();
    let mut after = before.clone();
    for item in card.items.iter().filter(|item| item.included) {
        let one = Card {
            items: vec![item.clone()],
            ..Card::default()
        };
        let values = changes::values(&one, &after).expect("values");
        let saved =
            crate::control::settings::check(&after, &values, &super::super::shortcut_rules())
                .expect("a save");
        after = saved.settings;
    }
    card.decision = changes::applied(
        crate::control::settings::diff(&before, &after),
        AT + 2 * MINUTE,
    );
    ui.settings = after;
    let model = card
        .items
        .iter()
        .find_map(|item| built_in::target(item, &ui.settings))
        .expect("a built-in model");
    super::super::output::local_cleanup::fixture_download(ui, model, model.bytes * 2 / 5);
    let id = show(ui, asks, card);
    ui.assistant.undo = vec![At {
        conversation: id,
        turn: 1,
        card: 0,
    }];
}

/// An answer that suggests cleaning up on this Mac: its card with the
/// models to pick from, on an 8 GB Mac.
pub(in super::super) fn card_local_model(ui: &mut Ui) {
    open(ui);
    let here = machine(8, None, &[]);
    ui.assistant.machine = here.clone();
    let state = state(here);
    let gate = Gate::new(&ui.settings, &state);
    let proposal = Proposal {
        key: built_in::RUNS_ON.into(),
        value: Some(json!("local")),
        why: "Cleans up on this Mac, offline and with no plan".into(),
        line: built_in::RUNS_ON.into(),
        ..Proposal::default()
    };
    let cards = changes::resolve(&[proposal], &gate, Origin::Answer).cards;
    let turn = Turn {
        question: "Can cleanup run without the internet?".into(),
        answer: "Yes. VibeHex can clean up dictation with a model it downloads and runs on \
                 this Mac, so nothing leaves it."
            .into(),
        model: "GPT-6.1-Sol".into(),
        seconds: 4.0,
        cards,
        created_ms: AT + MINUTE,
        ..Turn::default()
    };
    conversation(ui, "Offline cleanup", vec![turn], &[]);
}
