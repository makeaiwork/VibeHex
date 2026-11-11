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
use crate::assistant::web::Web;
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
        web: Default::default(),
    });
}

const LM_STUDIO: &str =
    "У меня стоит LM Studio. Как сделать, чтобы VibeHex чистил диктовку через него?";

/// An answer from the guide and the web: its steps open (the guide it
/// read, what it didn't cover, the pages the web gave), and both rows of
/// sources. Web is on in the field.
pub(in super::super) fn web_answer(ui: &mut Ui) {
    open(ui);
    let store = &mut ui.assistant.store;
    let id = store.start(LM_STUDIO, None, None);
    store.push(
        id,
        Turn {
            seconds: 9.0,
            web: Web {
                guide: vec!["topics/ai-cleanup".into(), "labs/ai-cleanup-server".into()],
                searches: vec!["LM Studio start local server".into()],
                pages: vec![
                    "https://lmstudio.ai/docs/developer/core/server".into(),
                    "https://lmstudio.ai/docs/app/basics".into(),
                ],
                sources: vec!["https://lmstudio.ai/docs/developer/core/server".into()],
                going: Vec::new(),
                held_back: false,
            },
            ..turn(
                LM_STUDIO,
                "Со стороны VibeHex всё есть, нужен только адрес сервера: **Output › AI cleanup › Your server**. Со стороны LM Studio, по [его документации](https://lmstudio.ai/docs/developer/core/server):\n\n1. Откройте вкладку **Developer** и включите **Start server**.\n2. Загрузите модель, например Qwen3 4B Instruct.\n3. Сервер будет слушать `localhost:1234`.",
                &["topics/ai-cleanup", "labs/ai-cleanup-server"],
                1,
            )
        },
    );
    ui.assistant.open = Some(id);
    ui.assistant.unfolded.push((id, 0));
}

/// The same answer folded, its conversation's Web turned off in the field.
pub(in super::super) fn web_off(ui: &mut Ui) {
    web_answer(ui);
    ui.assistant.unfolded.clear();
    if let Some(id) = ui.assistant.open {
        ui.assistant.store.set_web(id, Some(false));
    }
}

/// Web on, but the conversation has read the person's recordings: the
/// answer came from the guide alone, and its open steps say why.
pub(in super::super) fn web_held(ui: &mut Ui) {
    open(ui);
    let id = ui.assistant.store.start(LM_STUDIO, None, None);
    ui.assistant.store.push(
        id,
        Turn {
            seconds: 4.0,
            web: Web {
                guide: vec!["topics/ai-cleanup".into()],
                held_back: true,
                ..Web::default()
            },
            ..turn(
                LM_STUDIO,
                "Со стороны VibeHex нужен только адрес сервера: **Output › AI cleanup › Your server**. Как запустить сервер в самом LM Studio, руководство не говорит.",
                &["topics/ai-cleanup", "labs/ai-cleanup-server"],
                1,
            )
        },
    );
    ui.assistant.open = Some(id);
    ui.assistant.unfolded = vec![(id, 0)];
}

/// A web answer coming: a search done, a page being read.
pub(in super::super) fn web_answering(ui: &mut Ui) {
    open(ui);
    let id = ui.assistant.store.start(LM_STUDIO, None, None);
    ui.assistant.store.date(id, AT + MINUTE);
    ui.assistant.open = Some(id);
    ui.assistant.answering = Some(Answering {
        conversation: id,
        question: LM_STUDIO.into(),
        draft: String::new(),
        started: Instant::now(),
        steps: Vec::new(),
        searching: false,
        cards_coming: false,
        web: Web {
            searches: vec!["LM Studio start local server".into()],
            going: vec!["ws_2".into()],
            ..Web::default()
        },
    });
}

/// An answer naming places in the window, each a link that opens it, and
/// other words in bold: Add and the engine's name stay text, brighter.
pub(in super::super) fn places(ui: &mut Ui) {
    open(ui);
    ui.settings.meetings.enabled = true;
    let question = "How do I teach it a word it keeps getting wrong?";
    let id = ui.assistant.store.start(question, None, None);
    ui.assistant.store.push(
        id,
        turn(
            question,
            "Add the word in **General › Vocabulary**: press **Add**, type it as it should be written, and how it sounds.\n\n1. To add a word from any app, turn on **Labs › Add to vocabulary from a selection**.\n2. To fix whole sentences as well, set **Output › AI cleanup** to **Codex**.\n\nMeetings read your words too, when **Meetings › Settings** asks the notes to. **Nothing leaves this Mac** unless you choose a cloud engine.",
            &["topics/vocabulary", "labs/vocabulary-quick-add"],
            1,
        ),
    );
    ui.assistant.open = Some(id);
}

/// General opened from the answer's General › Vocabulary: the row lit,
/// and Back to Assistant before the title.
pub(in super::super) fn place_opened(ui: &mut Ui) {
    places(ui);
    super::places::scene(ui, "General › Vocabulary");
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
        web: Default::default(),
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
        notes: false,
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
    let text = turns_text(ui);
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

/// The fixture meeting's turns, one per line, as citations point into them.
fn turns_text(ui: &Ui) -> String {
    ui.meetings
        .meetings
        .iter()
        .find(|meeting| meeting.id == "preview-notes")
        .map(|meeting| {
            let turns = meeting.turns().iter().map(|turn| turn.text.as_str());
            turns.collect::<Vec<_>>().join("\n")
        })
        .unwrap_or_default()
}

/// An answer citing the meeting's notes as [1] and two places in its
/// transcript as [2] and [3], with [1] clicked: the meeting opens on
/// Summary at the top, and neither [2] nor [3] is current.
pub(in super::super) fn meeting_notes_cited(ui: &mut Ui) {
    *ui = mac_fixture();
    let text = turns_text(ui);
    let lines = |n: u32, start_ms: u64, words: &str| {
        let at = text.find(words).map(|at| at..at + words.len());
        Ref {
            n,
            kind: Kind::Meeting,
            id: "preview-notes".into(),
            title: "Arcade launch plan".into(),
            at_ms: AT,
            offset_ms: Some(start_ms),
            lines: vec![Lines {
                start_ms,
                at: at.unwrap_or_default(),
                ..Lines::default()
            }],
            notes: false,
        }
    };
    let notes = Ref {
        n: 1,
        kind: Kind::Meeting,
        id: "preview-notes".into(),
        title: "Arcade launch plan".into(),
        at_ms: AT,
        notes: true,
        ..Ref::default()
    };
    let refs = vec![
        notes,
        lines(2, 13_000, "we could make twenty to thirty games."),
        lines(3, 95_000, "We need a proposal for it."),
    ];
    let question = "What did we decide about the build screen?";
    let id = ui.assistant.store.start(question, None, None);
    ui.assistant.store.push(
        id,
        Turn {
            refs,
            ..turn(
                question,
                "One login for every game [1], twenty to thirty games [2], and a proposal for the build screen first [3].",
                &[],
                1,
            )
        },
    );
    ui.assistant.open = Some(id);
    super::jump::open(ui, (id, 0), 1, Some(0), &mut rune::Tx::default());
}

/// A minute and a half of one speaker: the passage citations land in.
const MONOLOGUE: &str = "Okay, let me walk through the whole plan once, because I think we keep talking past each other on the order of things. \
The first thing is the shared login, and that one is not negotiable, every game in the arcade signs in the same way or the store makes no sense. \
Then there is the catalogue itself, which is where the numbers come from, and I went back over them twice this week. \
We have four games that are basically done, six that are playable but rough, and a long tail of prototypes that may or may not survive. \
If we only count what we can finish without hiring, the honest answer is that we ship twelve games by the end of the quarter. \
That is not the twenty to thirty everybody quotes in the deck, and I would rather we stop repeating that number to partners. \
The second thing is the build screen, which still looks like an internal tool, and people judge the whole arcade by it. \
I spent an afternoon with it and the problems are not cosmetic, the flow is wrong, you pick a template before you know what it does. \
So before anybody polishes buttons, we need someone to write down how a person should get from an idea to a published game. \
The third thing is money, and here I want to be careful, because the budget we agreed in January assumed the bigger catalogue. \
With twelve games the hosting costs drop a lot, but marketing does not, so the split between the two has to move. \
I would move about a fifth of the hosting budget into marketing and keep the rest as a reserve until we see real traffic. \
And finally the schedule: we said the end of the month for the public beta, and I still think that is possible for the login and the store. \
The build screen will not be ready by then, so the beta ships with the old one and a clear note that a new one is coming. \
So, to sum up: shared login first, an honest catalogue of twelve, a written proposal for the build screen, and the hosting money moved into marketing.";

/// A meeting opened from an answer citing two places deep in one long
/// passage: the first cited words land a third of the way down, not the
/// passage's top.
pub(in super::super) fn meeting_cited_long(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.pane = Pane::Meetings;
    ui.meetings.select("preview-notes".into());
    ui.meetings.tab = Some(super::super::meeting_tabs::MeetingTab::Transcript);
    if let Some(meeting) =
        (ui.meetings.meetings.iter_mut()).find(|meeting| meeting.id == "preview-notes")
        && let super::super::meetings::Transcript::Loaded(turns) = &mut meeting.transcript
        && let Some(turn) = turns.get_mut(4)
    {
        turn.text = MONOLOGUE.into();
        turn.words = Vec::new();
    }
    let text = turns_text(ui);
    ui.came_from = Some(super::jump::scene(
        Kind::Meeting,
        "preview-notes",
        "How many games, and what about the build screen",
        &[
            said(
                2,
                24_000,
                &text,
                "the honest answer is that we ship twelve games by the end of the quarter.",
            ),
            said(3, 24_000, &text, "a written proposal for the build screen"),
        ],
        0,
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
