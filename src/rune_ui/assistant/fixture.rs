//! The Assistant's scenes: fixed conversations on a Mac, rendered at time
//! zero, so nothing here reads the clock, the disk, or a model. Times are a
//! fixed day long past, so a golden reads the same on any day.

use std::ops::Range;
use std::time::Instant;

use super::super::Ui;
use super::super::files::FilesPane;
use super::super::scenes::mac_fixture;
use super::super::sidebar::Pane;
use super::View;
use crate::assistant::recordings::{Kind, Lines, Ref, Step};
use crate::assistant::store::Turn;
use crate::assistant::worker::Answering;

/// 14 March 2025, 10:40 UTC.
pub(super) const AT: u64 = 1_741_948_800_000;
pub(super) const MINUTE: u64 = 60_000;

/// Assistant's pane on a Mac, answering on Codex.
pub(super) fn open(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.pane = Pane::Assistant;
    ui.assistant.fixture = true;
    ui.assistant.fixed_now = Some(AT + 2 * MINUTE);
    ui.settings.assistant.engine = Some("codex:gpt-6.1-sol".into());
    ui.files.codex_models = FilesPane::fixture_codex_models();
}

pub(super) fn turn(question: &str, answer: &str, sources: &[&str], minutes: u64) -> Turn {
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
        question: "Who writes the meeting notes, and are they any good in Russian when half of the call is in English?".into(),
        draft: "Notes are written by the model set in **Meetings › Settings › Notes**. Apple Intelligence works offline, but".into(),
        started: Instant::now(),
        steps: Vec::new(),
        searching: false,
        cards_coming: false,
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

const DAY: u64 = 24 * 60 * MINUTE;

/// A question about meetings waiting for leave to read them.
pub(in super::super) fn consent(ui: &mut Ui) {
    open(ui);
    let question = "Что мы решили по бюджету на встречах в марте?";
    let id = ui.assistant.store.start(question, None, None);
    ui.assistant.store.date(id, AT + MINUTE);
    ui.assistant.open = Some(id);
    ui.assistant.consent = Some(super::recall::Pending {
        conversation: id,
        question: question.into(),
    });
}

/// The search's steps as they land: meetings found and read, files being
/// searched.
pub(in super::super) fn steps(ui: &mut Ui) {
    open(ui);
    let question = "Что мы решили по бюджету на встречах в марте?";
    let id = ui.assistant.store.start(question, None, None);
    ui.assistant.store.date(id, AT + MINUTE);
    ui.assistant.open = Some(id);
    ui.assistant.answering = Some(Answering {
        conversation: id,
        question: question.into(),
        draft: String::new(),
        started: Instant::now(),
        steps: vec![
            Step::Meetings {
                searched: 12,
                found: 4,
            },
            Step::Read {
                notes: 4,
                passages: 6,
            },
        ],
        searching: true,
        cards_coming: false,
    });
}

fn place(n: u32, kind: Kind, id: &str, title: &str, at_ms: u64, offset: Option<u64>) -> Ref {
    Ref {
        n,
        kind,
        id: id.into(),
        title: title.into(),
        at_ms,
        offset_ms: offset,
        lines: offset
            .map(|start_ms| Lines {
                start_ms,
                ..Lines::default()
            })
            .into_iter()
            .collect(),
    }
}

/// An answer from four meetings and a file: the steps folded into one
/// line, `[n]` marks, and the places cited.
pub(in super::super) fn recordings(ui: &mut Ui) {
    open(ui);
    let question = "Что мы решили по бюджету на встречах в марте?";
    let id = ui.assistant.store.start(question, None, None);
    ui.assistant.store.date(id, AT);
    let meeting = Kind::Meeting;
    ui.assistant.store.push(
        id,
        Turn {
            steps: vec![
                Step::Meetings {
                    searched: 12,
                    found: 4,
                },
                Step::Read {
                    notes: 4,
                    passages: 6,
                },
                Step::Files {
                    searched: 3,
                    found: 1,
                },
            ],
            refs: vec![
                place(1, meeting, "planerka", "Планёрка", AT - 10 * DAY, Some(60_000)),
                place(2, meeting, "hiring", "Найм бэкенда", AT - 7 * DAY, Some(720_000)),
                place(3, meeting, "budget", "Бюджет Q1", AT - 3 * DAY, Some(300_000)),
                place(4, Kind::File, "draft", "budget-draft.m4a", AT - 2 * DAY, Some(6_000)),
            ],
            seconds: 9.0,
            ..turn(
                question,
                "В марте бюджет обсуждали на трёх встречах, и решение складывалось постепенно.\n\n- **4 марта** утверждение отложили: не хватало цифр по маркетингу [1].\n- **7 марта** решили взять деньги на второго бэкендера из маркетинга и перенести утверждение [2].\n- **11 марта** утвердили лимит на Q1, 1,2 млн рублей, маркетинг урезали на 15 % [3]. В черновике было 1,5 млн [4].",
                &[],
                1,
            )
        },
    );
    ui.assistant.open = Some(id);
}

/// The same answer with its steps open.
pub(in super::super) fn steps_open(ui: &mut Ui) {
    recordings(ui);
    let id = ui.assistant.open.unwrap_or_default();
    ui.assistant.unfolded.push((id, 0));
}

/// Where `said` is in `text`, cited by `[n]` at `start_ms`, as a scene's
/// place: the bytes an answer's cited lines map to.
fn said(n: u32, start_ms: u64, text: &str, said: &str) -> (u32, Option<u64>, Option<Range<usize>>) {
    let at = text.find(said).map(|at| at..at + said.len());
    (n, Some(start_ms), at)
}

/// A meeting opened from an answer citing it twice: Back to Assistant,
/// the bar, the cited lines lit, the second landed on a third of the way
/// down and the first fainter.
pub(in super::super) fn meeting_cited(ui: &mut Ui) {
    meeting_cited_on(ui, 1);
}

/// [`meeting_cited`] with its `current`-th place shown.
pub(in super::super) fn meeting_cited_on(ui: &mut Ui, current: usize) {
    *ui = mac_fixture();
    ui.pane = Pane::Meetings;
    ui.meetings.select("preview-notes".into());
    ui.meetings.tab = Some(super::super::meeting_tabs::MeetingTab::Transcript);
    let text = ui
        .meetings
        .meetings
        .iter()
        .find(|meeting| meeting.id == "preview-notes")
        .map(|meeting| {
            let turns = meeting.turns().iter().map(|turn| turn.text.as_str());
            turns.collect::<Vec<_>>().join("\n")
        })
        .unwrap_or_default();
    ui.came_from = Some(super::jump::scene(
        Kind::Meeting,
        "preview-notes",
        "What did we decide about the build screen and games",
        &[
            said(2, 13_000, &text, "we could make twenty to thirty games."),
            said(3, 95_000, &text, "We need a proposal for it."),
        ],
        current,
    ));
}

/// The Files text the scenes open, and where `said` is in its Original.
fn file_place(
    ui: &Ui,
    n: u32,
    start_ms: u64,
    words: &str,
) -> (u32, Option<u64>, Option<Range<usize>>) {
    let text = ui
        .files
        .record("preview-interview")
        .and_then(|record| record.versions.first())
        .map(|original| crate::files::text::plain(&original.segments))
        .unwrap_or_default();
    said(n, start_ms, &text, words)
}

/// A Files text opened from an answer: its Original, the cited sentence
/// lit in its paragraph.
pub(in super::super) fn file_cited(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.settings.labs.files = true;
    ui.files = FilesPane::fixture();
    ui.files.tab = Some(super::super::files::Tab::Version(1));
    ui.pane = Pane::Files;
    let place = file_place(
        ui,
        1,
        48_000,
        "The client will try it with one team if we show a prototype by the end of the month.",
    );
    ui.came_from = Some(super::jump::scene(
        Kind::File,
        "preview-interview",
        "What does the client want",
        &[place],
        0,
    ));
}

/// The same with Labs › Listen to the recording on and the player in the
/// cited sentence: the player's word lit over the cited one.
pub(in super::super) fn file_cited_listening(ui: &mut Ui) {
    file_cited(ui);
    ui.settings.labs.playback = true;
    let of = super::super::playback::Of::File("preview-interview".into());
    ui.playback = super::super::playback::Playback::still(of, 54_500, 2_530_000, true, true);
}

/// History opened from an answer citing a dictation: its row tinted with
/// the answer's [n], selected, and Back to Assistant.
pub(in super::super) fn history_cited(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.history = super::super::history::HistoryPane::fixture();
    ui.history.selected = Some(1);
    ui.pane = Pane::History;
    ui.came_from = Some(super::jump::scene(
        Kind::Dictation,
        "1",
        "What did I promise to review",
        &[(2, None, None)],
        0,
    ));
}
