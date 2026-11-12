//! Guided setup's behaviour, pinned: the first-run path, the returning
//! path, and the preset, each as the presses a person makes and what the
//! conversation and the kept setups hold after every one. The lines were
//! written down from the pane before setup became one state machine
//! (`assistant::setup::flow`); a line that changes is a behaviour change.
//!
//! They hold on a Mac alone: which follow-ups are asked and which of the
//! preset's rows exist depend on the OS. The machine's own tables
//! (`setup/flow/tests.rs`) hold on every OS.

use super::super::Ui;
use super::ask::{self, At};
use super::changes;
use crate::assistant::changes::{CardKind, Decision};
use crate::assistant::store::Turn;

/// One turn as a line: what it shows and what it carries.
fn shape(turn: &Turn) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut flag = |on: bool, name: &str| {
        if on {
            parts.push(name.to_owned());
        }
    };
    flag(!turn.question.is_empty(), "question");
    flag(!turn.now.is_empty(), "now");
    flag(!turn.since.is_empty(), "since");
    flag(!turn.facts.is_empty(), "facts");
    flag(turn.folded, "folded");
    flag(turn.follow, "follow");
    flag(!turn.answer.is_empty(), "answer");
    if !turn.preset.is_empty() {
        parts.push(format!("preset={}", turn.preset));
    }
    if !turn.asks.is_empty() {
        let asks: Vec<String> = turn
            .asks
            .iter()
            .map(|ask| {
                let mut line = format!("{}={}", ask.id, ask.chosen.join("+"));
                for (on, mark) in [(ask.touched, '*'), (ask.before, '~'), (ask.done, '!')] {
                    if on {
                        line.push(mark);
                    }
                }
                line
            })
            .collect();
        parts.push(format!("asks({})", asks.join(" ")));
    }
    if !turn.more.is_empty() {
        parts.push(format!("more({})", turn.more.len()));
    }
    if !turn.answers.is_empty() {
        let ids: Vec<&str> = turn
            .answers
            .iter()
            .map(|answer| answer.id.as_str())
            .collect();
        parts.push(format!("answers({})", ids.join(" ")));
    }
    if !turn.terms.is_empty() {
        parts.push(format!("terms({})", turn.terms.len()));
    }
    for card in &turn.cards {
        let kind = match card.kind {
            CardKind::Plan => "plan",
            _ => "card",
        };
        let decision = match &card.decision {
            Decision::Applied { .. } => "applied",
            Decision::Declined => "declined",
            Decision::Undone => "undone",
            Decision::Failed { .. } => "failed",
            _ => "open",
        };
        let included = card.items.iter().filter(|item| item.included).count();
        parts.push(format!(
            "{kind}({included}/{} {decision})",
            card.items.len()
        ));
    }
    parts.join(" ")
}

/// The open conversation's turns and the setups kept, as lines.
fn trace(ui: &Ui) -> Vec<String> {
    let mut lines: Vec<String> = ui
        .assistant
        .conversation()
        .map(|open| open.turns.iter().map(shape).collect())
        .unwrap_or_default();
    for setup in ui.assistant.store.setups() {
        let ids: Vec<&str> = setup
            .answers
            .iter()
            .map(|answer| answer.id.as_str())
            .collect();
        lines.push(format!(
            "kept: answers({}) asked({}) applied({}) declined({}){}",
            ids.join(" "),
            setup.asked.len(),
            setup.applied.len(),
            setup.declined.len(),
            if setup.plan_ms == 0 { "" } else { " decided" }
        ));
    }
    lines
}

/// Holds `ui` to `want` after `step`; with `HEX_PRINT_PINS` set, prints
/// what it is instead, to write the lines down from.
fn pin(ui: &Ui, step: &str, want: &[&str]) {
    let got = trace(ui);
    if std::env::var_os("HEX_PRINT_PINS").is_some() {
        println!("    // {step}");
        println!("    &[");
        for line in &got {
            println!("        {line:?},");
        }
        println!("    ],");
        return;
    }
    assert_eq!(got, want, "after: {step}");
}

/// A Mac with nothing recorded: a first setup, the app writing every
/// round and the plan itself.
fn fresh() -> Ui {
    crate::desktop::i18n::apply(Some("en"));
    let mut ui = super::super::scenes::mac_fixture();
    super::fixture::open(&mut ui);
    ui.meetings.meetings.clear();
    ui.history.entries.clear();
    ui
}

fn last(ui: &Ui) -> At {
    let open = ui.assistant.conversation().unwrap();
    At {
        conversation: open.id,
        turn: open.turns.len() - 1,
    }
}

fn plan_at(ui: &Ui) -> changes::At {
    let at = last(ui);
    changes::At {
        conversation: at.conversation,
        turn: at.turn,
        card: 0,
    }
}

fn number(ui: &Ui, id: &str) -> usize {
    let at = last(ui);
    let open = ui.assistant.conversation().unwrap();
    open.turns[at.turn]
        .asks
        .iter()
        .position(|ask| ask.id == id)
        .unwrap()
}

/// The first asked follow-up of the open round that isn't from before.
fn first_new(ui: &Ui) -> usize {
    let at = last(ui);
    let open = ui.assistant.conversation().unwrap();
    open.turns[at.turn]
        .asks
        .iter()
        .position(|ask| !ask.before)
        .unwrap()
}

#[test]
fn a_first_setup_then_a_returning_one() {
    let mut ui = fresh();

    ask::begin(&mut ui);
    pin(&ui, "Set VibeHex up for me", FIRST[0]);

    // Code too, and calls: the language follows the code, the notes
    // question shows.
    let at = last(&ui);
    let uses = number(&ui, "use");
    ask::pick(&mut ui, at, uses, "dictation_code");
    ask::pick(&mut ui, at, uses, "meetings");
    pin(&ui, "code and calls picked", FIRST[1]);

    ask::answered(&mut ui, at, false);
    pin(&ui, "Continue", FIRST[2]);

    // Two follow-ups yes, one no, one left; Continue.
    let at = last(&ui);
    let one = first_new(&ui);
    ask::pick(&mut ui, at, one, "yes");
    ask::pick(&mut ui, at, one + 1, "no");
    ask::pick(&mut ui, at, one + 2, "yes");
    ask::answered(&mut ui, at, false);
    pin(&ui, "yes, no, yes, Continue", FIRST[3]);

    // Whatever round came: skip to the plan.
    let at = last(&ui);
    ask::answered(&mut ui, at, true);
    pin(&ui, "Skip to the plan", FIRST[4]);

    // One row unticked, the plan applied.
    let plan = plan_at(&ui);
    changes::toggle(&mut ui, plan, 0);
    changes::apply(&mut ui, plan);
    pin(&ui, "a row unticked, Apply", FIRST[5]);

    // The next setup returns: round one folded, as answered.
    ui.assistant.open = None;
    ask::begin(&mut ui);
    pin(&ui, "Set VibeHex up for me, again", FIRST[6]);

    // "Change answers" opens the questions; files too now.
    let at = last(&ui);
    ui.assistant.store.unfold(at.conversation, at.turn);
    let uses = number(&ui, "use");
    ask::pick(&mut ui, at, uses, "files");
    ask::answered(&mut ui, at, false);
    pin(&ui, "Change answers, files too, Continue", FIRST[7]);

    // The round kept as answered before, Continue until the plan.
    for _ in 0..4 {
        let at = last(&ui);
        let open = ui.assistant.conversation().unwrap();
        if !open.turns[at.turn].cards.is_empty() || open.turns[at.turn].asks.is_empty() {
            break;
        }
        ask::answered(&mut ui, at, false);
    }
    pin(&ui, "Continue to the plan", FIRST[8]);

    if !ui.assistant.conversation().unwrap().turns[last(&ui).turn]
        .cards
        .is_empty()
    {
        let plan = plan_at(&ui);
        changes::decline(&mut ui, plan);
    }
    pin(&ui, "Not now", FIRST[9]);
}

#[test]
fn the_preset_then_its_shortcuts() {
    let mut ui = fresh();
    ui.settings.transcription.language = "ru".into();
    crate::desktop::i18n::apply(Some("en"));

    super::preset::begin(&mut ui, String::new());
    pin(&ui, "Or set it up like Denis", PRESET[0]);

    let plan = plan_at(&ui);
    changes::decline(&mut ui, plan);
    pin(&ui, "Not now", PRESET[1]);

    let id = ui.assistant.open.unwrap();
    super::preset::use_these(&mut ui, id);
    pin(&ui, "Use these", PRESET[2]);

    // Another conversation: the plan applied, then Keep mine.
    ui.assistant.open = None;
    super::preset::begin(&mut ui, String::new());
    let plan = plan_at(&ui);
    changes::apply(&mut ui, plan);
    let id = ui.assistant.open.unwrap();
    super::preset::keep_mine(&mut ui, id);
    pin(&ui, "again: Apply, Keep mine", PRESET[3]);
}

const FIRST: [&[&str]; 10] = [
    // Set VibeHex up for me
    &["now asks(use=dictation_any+meetings language=en cleanup=none notes=apple)"],
    // code and calls picked
    &["now asks(use=dictation_any+dictation_code* language=en cleanup=none notes=apple)"],
    // Continue
    &[
        "now asks(use=dictation_any+dictation_code*! language=en! cleanup=none! notes=!)",
        "asks(labs.caps_lock_dictation= labs.failure_notices= labs.filler_words= labs.live_text=) more(8)",
        "kept: answers(use language cleanup) asked(0) applied(0) declined(0)",
    ],
    // yes, no, yes, Continue
    &[
        "now asks(use=dictation_any+dictation_code*! language=en! cleanup=none! notes=!)",
        "asks(labs.caps_lock_dictation=yes*! labs.failure_notices=no*! labs.filler_words=yes*! labs.live_text=!) more(8)",
        "asks(labs.recent_dictations= labs.spoken_symbols= labs.stop_shortcut= labs.vocabulary_biasing=) more(4)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(0) declined(1)",
    ],
    // Skip to the plan
    &[
        "now asks(use=dictation_any+dictation_code*! language=en! cleanup=none! notes=!)",
        "asks(labs.caps_lock_dictation=yes*! labs.failure_notices=no*! labs.filler_words=yes*! labs.live_text=!) more(8)",
        "asks(labs.recent_dictations=! labs.spoken_symbols=! labs.stop_shortcut=! labs.vocabulary_biasing=!) more(4)",
        "answer answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) plan(2/2 open)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(0) declined(1)",
    ],
    // a row unticked, Apply
    &[
        "now asks(use=dictation_any+dictation_code*! language=en! cleanup=none! notes=!)",
        "asks(labs.caps_lock_dictation=yes*! labs.failure_notices=no*! labs.filler_words=yes*! labs.live_text=!) more(8)",
        "asks(labs.recent_dictations=! labs.spoken_symbols=! labs.stop_shortcut=! labs.vocabulary_biasing=!) more(4)",
        "answer answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) plan(1/2 applied)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(1) declined(2) decided",
    ],
    // Set VibeHex up for me, again
    &[
        "now since facts folded asks(use=dictation_any+dictation_code* language=en* cleanup=none* notes=apple)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(1) declined(2) decided",
    ],
    // Change answers, files too, Continue
    &[
        "now since facts asks(use=dictation_any+dictation_code+files*! language=en*! cleanup=none*! notes=apple!)",
        "asks(labs.cleanup_progress= labs.live_text= labs.recent_dictations= labs.spoken_symbols= labs.caps_lock_dictation=yes~ labs.failure_notices=no~) more(8)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(1) declined(2) decided",
        "kept: answers(use language cleanup notes) asked(0) applied(0) declined(0)",
    ],
    // Continue to the plan
    &[
        "now since facts asks(use=dictation_any+dictation_code+files*! language=en*! cleanup=none*! notes=apple!)",
        "asks(labs.cleanup_progress=! labs.live_text=! labs.recent_dictations=! labs.spoken_symbols=! labs.caps_lock_dictation=yes~! labs.failure_notices=no~!) more(8)",
        "asks(labs.stop_shortcut=! labs.vocabulary_biasing=! labs.vocabulary_quick_add=! labs.screenshots=!) more(4)",
        "asks(paste_with_shift=! labs.playback=! labs.keep_on_clipboard=! labs.keep_recordings=!)",
        "answer answers(use language cleanup notes labs.caps_lock_dictation labs.failure_notices) plan(1/1 open)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(1) declined(2) decided",
        "kept: answers(use language cleanup notes labs.caps_lock_dictation labs.failure_notices) asked(2) applied(0) declined(1)",
    ],
    // Not now
    &[
        "now since facts asks(use=dictation_any+dictation_code+files*! language=en*! cleanup=none*! notes=apple!)",
        "asks(labs.cleanup_progress=! labs.live_text=! labs.recent_dictations=! labs.spoken_symbols=! labs.caps_lock_dictation=yes~! labs.failure_notices=no~!) more(8)",
        "asks(labs.stop_shortcut=! labs.vocabulary_biasing=! labs.vocabulary_quick_add=! labs.screenshots=!) more(4)",
        "asks(paste_with_shift=! labs.playback=! labs.keep_on_clipboard=! labs.keep_recordings=!)",
        "answer answers(use language cleanup notes labs.caps_lock_dictation labs.failure_notices) plan(1/1 declined)",
        "kept: answers(use language cleanup labs.caps_lock_dictation labs.failure_notices labs.filler_words) asked(3) applied(1) declined(2) decided",
        "kept: answers(use language cleanup notes labs.caps_lock_dictation labs.failure_notices) asked(2) applied(0) declined(1)",
    ],
];

const PRESET: [&[&str]; 4] = [
    // Or set it up like Denis
    &["answer preset=settings plan(119/120 open)"],
    // Not now
    &[
        "answer preset=settings plan(119/120 declined)",
        "answer preset=shortcuts",
    ],
    // Use these
    &[
        "answer preset=settings plan(119/120 declined)",
        "answer preset=shortcuts",
        "question answer preset=shortcuts-plan plan(3/3 open)",
    ],
    // again: Apply, Keep mine
    &[
        "answer preset=settings plan(119/120 applied)",
        "answer preset=shortcuts",
        "question answer preset=done",
    ],
];
