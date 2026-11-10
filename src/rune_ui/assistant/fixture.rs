//! The Assistant's scenes: fixed conversations on a Mac, rendered at time
//! zero, so nothing here reads the clock, the disk, or a model. Times are a
//! fixed day long past, so a golden reads the same on any day.

use std::time::Instant;

use super::super::Ui;
use super::super::files::FilesPane;
use super::super::scenes::mac_fixture;
use super::super::sidebar::Pane;
use super::View;
use crate::assistant::store::Turn;
use crate::assistant::worker::Answering;

/// 14 March 2025, 10:40 UTC.
const AT: u64 = 1_741_948_800_000;
const MINUTE: u64 = 60_000;

/// Assistant's pane on a Mac, answering on Codex.
fn open(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.pane = Pane::Assistant;
    ui.assistant.fixture = true;
    ui.assistant.fixed_now = Some(AT + 2 * MINUTE);
    ui.settings.assistant.engine = Some("codex:gpt-6.1-sol".into());
    ui.files.codex_models = FilesPane::fixture_codex_models();
}

fn turn(question: &str, answer: &str, sources: &[&str], minutes: u64) -> Turn {
    Turn {
        question: question.into(),
        answer: answer.into(),
        sources: sources.iter().map(|id| (*id).into()).collect(),
        model: "GPT-6.1-Sol".into(),
        seconds: 7.0,
        created_ms: AT + minutes * MINUTE,
        ..Turn::default()
    }
}

/// Two earlier conversations and the open one: a question answered from
/// the guide, with the articles it drew on.
pub(in super::super) fn conversation(ui: &mut Ui) {
    open(ui);
    let store = &mut ui.assistant.store;
    let older = store.start("Which speech model is best for Russian?", None, None);
    store.push(
        older,
        turn(
            "Which speech model is best for Russian?",
            "Parakeet v3 handles Russian well and runs on this Mac.",
            &["topics/speech-models"],
            0,
        ),
    );
    let id = store.start(
        "Can Zoom calls record by themselves, without asking?",
        None,
        None,
    );
    store.push(
        id,
        turn(
            "Can Zoom calls record by themselves, without asking?",
            "Yes. **Meetings** does it, and it's off in Labs now. Two steps:\n\n1. Turn on **Labs › Meetings**.\n2. In **Meetings › Settings › Detected calls**, set Zoom to **Always record** instead of Ask.\n\nRecordings and notes stay on this Mac. The first time, macOS asks once to allow recording system audio.",
            &["labs/meetings", "topics/meetings"],
            1,
        ),
    );
    ui.assistant.open = Some(id);
}

/// The same conversation with a follow-up being answered: the draft so
/// far, the face reading, and Stop in the field.
pub(in super::super) fn answering(ui: &mut Ui) {
    conversation(ui);
    let id = ui.assistant.open.unwrap_or_default();
    ui.assistant.answering = Some(Answering {
        conversation: id,
        question: "Who writes the notes? Are they good in Russian?".into(),
        draft: "Notes are written by the model set in **Meetings › Settings › Notes**. Apple Intelligence works offline, but".into(),
        started: Instant::now(),
        steps: Vec::new(),
        searching: false,
    });
}

/// A new conversation: the greeting and the example questions.
pub(in super::super) fn start(ui: &mut Ui) {
    open(ui);
}

/// Assistant › Settings.
pub(in super::super) fn settings(ui: &mut Ui) {
    conversation(ui);
    ui.assistant.view = View::Settings;
}

/// No model connected: nothing chosen for Assistant, AI cleanup on
/// OpenCode, and OpenCode not running. Codex is installed.
pub(in super::super) fn connect(ui: &mut Ui) {
    open(ui);
    ui.settings.assistant.engine = None;
    ui.settings.labs.ai_cleanup_server = false;
    ui.assistant.reach.codex = Some(true);
    ui.assistant.reach.opencode = Some(false);
}
