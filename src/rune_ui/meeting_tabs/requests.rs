//! A meeting's Requests tab: questions about the meeting, in sessions as
//! Files keeps them. The tab's chevron lists the sessions about this
//! meeting and starts a new one; the open session reads as a chat (each
//! question in a bubble, each answer with Copy, Ask again, its model, and
//! its time), and the field floats under it, as in Files, with the model
//! and, for Codex, the reasoning. Files' worker answers, reading the
//! transcript with the meeting's title and times.

use rune::{El, TextEdit, col, el, icon, row, text};
use rune_kit::{Placement, Popover};

use super::super::controls::{DESCRIPTION, Paint};
use super::super::files::composer;
use super::super::files::session::{self, Answered};
use super::super::list_detail as ld;
use super::super::meetings::{Meeting, Status, clock_offset, transcript_markdown};
use super::super::{Menu, Pane, Ui};
use super::MeetingTab;
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::files::tidy;
use crate::files::worker::{self, Config, Job, Task};
use crate::meeting::requests::{self as store, Session};

const MODEL_MENU: &str = "meeting-ask-model";
const EFFORT_MENU: &str = "meeting-ask-effort";
const SESSIONS_MENU: &str = "meeting-sessions";
/// The chip that cleans the transcript, turn by turn, in a new session.
pub(super) const TIDY: &str = "Clean up transcript";
/// Room under the text for the field, and its height before it is measured.
const FIELD_CLEARANCE: f32 = 12.0;
const FIELD_GUESS: f32 = 110.0;
/// The field's distance from the card's bottom edge, as Files leaves it.
const FIELD_BOTTOM: f32 = 14.0;

/// The tab's state: the shown meeting's sessions and the field.
#[derive(Default)]
pub(in super::super) struct RequestsPane {
    /// The meeting whose sessions are loaded.
    meeting: Option<String>,
    sessions: Vec<Session>,
    /// The open session; `None` while a new one is written.
    open: Option<u32>,
    pub(super) ask: TextEdit,
    /// The model and reasoning a new session starts on; `None` follows
    /// Files › Settings.
    engine: Option<String>,
    effort: Option<String>,
    /// The worker's jobs on meetings.
    pub(super) tasks: Vec<Task>,
    composer_h: f32,
    text_under: bool,
    woken: bool,
    fixture: bool,
}

/// Whether `meeting` has the tab: a finished meeting with a transcript,
/// while Labs › AI cleanup server is on.
pub(in super::super) fn available(ui: &Ui, meeting: &Meeting) -> bool {
    ui.settings.labs.ai_cleanup_server
        && ui.settings.meetings.questions.enabled
        && !meeting.status.is_active()
        && !meeting.turns().is_empty()
}

/// How many sessions the shown meeting has.
pub(in super::super) fn count(ui: &Ui, meeting: &Meeting) -> usize {
    let pane = &ui.meetings.requests;
    if pane.meeting.as_deref() == Some(meeting.id.as_str()) {
        pane.sessions.len()
    } else {
        0
    }
}

/// Whether one of the meeting's questions is being answered.
pub(in super::super) fn working(ui: &Ui, meeting: &Meeting) -> bool {
    let record = store::record(&meeting.id);
    ui.meetings
        .requests
        .tasks
        .iter()
        .any(|task| task.record == record)
}

fn running(ui: &Ui, session: u32) -> Option<&Task> {
    let pane = &ui.meetings.requests;
    let record = store::record(pane.meeting.as_deref()?);
    pane.tasks
        .iter()
        .find(|task| task.record == record && task.session == Some(session))
}

fn newest(sessions: &[Session]) -> Option<u32> {
    sessions
        .iter()
        .max_by_key(|session| session.updated_ms())
        .map(|session| session.id)
}

/// Follows the worker and the shown meeting: its sessions load when it is
/// opened and again when one of its answers lands.
pub(in super::super) fn poll(ui: &mut Ui) -> bool {
    if ui.pane != Pane::Meetings || ui.meetings.requests.fixture || ui.is_fixture() {
        return false;
    }
    if !ui.meetings.requests.woken
        && let Some(waker) = ui.modes.waker.clone()
    {
        worker::set_waker(move || waker());
        ui.meetings.requests.woken = true;
    }
    let mut changed = composer::poll_models(ui, MODEL_MENU);
    let tasks: Vec<Task> = worker::tasks()
        .into_iter()
        .filter(|task| store::meeting_of(&task.record).is_some())
        .collect();
    let pane = &mut ui.meetings.requests;
    let landed = pane
        .tasks
        .iter()
        .any(|was| !tasks.iter().any(|task| task.id == was.id));
    if tasks != pane.tasks {
        pane.tasks = tasks;
        changed = true;
    }
    let shown = ui.meetings.selected.clone();
    if shown != pane.meeting || landed {
        pane.sessions = shown.as_deref().map(store::load).unwrap_or_default();
        if shown != pane.meeting {
            pane.meeting = shown;
            pane.open = newest(&pane.sessions);
            ui.files.recent = crate::files::store::recent();
        }
        changed = true;
    }
    changed
}

/// The tab's chevron and the list of sessions about the meeting.
pub(in super::super) fn sessions_menu(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let pane = &ui.meetings.requests;
    let open = ui.open == Some(Menu::Pick(SESSIONS_MENU));
    let chevron = row()
        .key("chevron")
        .role("button")
        .label(tr("Sessions about this meeting"))
        .h(24.0)
        .w(22.0)
        .center()
        .radius(5.0)
        .hover(c.hover)
        .child(
            icon(
                if open {
                    p.k.glyph.chevron_up
                } else {
                    p.k.glyph.chevron_down
                },
                12.0,
            )
            .color(c.text_base),
        )
        .on_press(|ui: &mut Ui| {
            let menu = Menu::Pick(SESSIONS_MENU);
            ui.open = (ui.open != Some(menu)).then_some(menu);
        });
    let mut popover = Popover::new("meeting-sessions-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .padding(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .trigger(chevron)
        .child(
            text(tr("Sessions about this meeting"))
                .key("heading")
                .font(p.k.ty.small_medium)
                .color(p.faint())
                .px(10.0)
                .pt(4.0)
                .pb(2.0),
        );
    let mut sessions: Vec<&Session> = pane.sessions.iter().collect();
    sessions.sort_by_key(|session| std::cmp::Reverse(session.updated_ms()));
    for session in sessions {
        popover = popover.child(session_row(p, session, pane.open == Some(session.id)));
    }
    let new = row()
        .key("new-session")
        .role("menuitem")
        .label(tr("New session"))
        .px(10.0)
        .py(7.0)
        .gap(8.0)
        .center_y()
        .radius(4.0)
        .hover(c.hover)
        .child(el().key("check").size(14.0, 16.0))
        .child(
            text(format!("+ {}", tr("New session")))
                .key("label")
                .font(p.k.ty.ui)
                .color(c.text_accent),
        )
        .on_press(|ui: &mut Ui| {
            ui.open = None;
            ui.meetings.requests.open = None;
            ui.meetings.requests.ask = TextEdit::default();
        });
    popover
        .child(
            el().key("rule")
                .h(p.hair)
                .mx(4.0)
                .my(4.0)
                .fill(c.border_weak),
        )
        .child(new)
        .build(p.k, layer)
}

/// One session in the list: its name over how many questions and when,
/// ✓ on the open one.
fn session_row(p: Paint, session: &Session, on: bool) -> El {
    let c = &p.k.c;
    let asked = match session.turns.len() {
        1 => tr_fill("1 question, {}", &session::when(session.updated_ms())),
        count => tr_named(
            "{count} questions, {when}",
            &[
                ("count", &count.to_string()),
                ("when", &session::when(session.updated_ms())),
            ],
        ),
    };
    let id = session.id;
    row()
        .key(format!("session{id}"))
        .role("menuitem")
        .label(session.name.clone())
        .w(280.0)
        .px(10.0)
        .py(6.0)
        .gap(8.0)
        .items_start()
        .radius(4.0)
        .fill(if on {
            c.hover
        } else {
            rune::Color::TRANSPARENT
        })
        .hover(c.hover)
        .child(
            el().key("check").size(14.0, 16.0).center().child(
                icon(p.k.glyph.check_small, 14.0)
                    .color(c.icon_base)
                    .opacity(if on { 1.0 } else { 0.0 }),
            ),
        )
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(
                    text(session.name.clone())
                        .key("name")
                        .font(p.k.ty.ui)
                        .color(p.text())
                        .wrap_anywhere()
                        .max_lines(2),
                )
                .child(
                    text(asked)
                        .key("asked")
                        .font(p.k.ty.small)
                        .color(p.muted())
                        .truncate(),
                ),
        )
        .on_press(move |ui: &mut Ui| {
            ui.open = None;
            ui.meetings.requests.open = Some(id);
        })
}

/// The open session as a chat, or a word on what to ask.
pub(in super::super) fn chat(ui: &Ui, p: Paint) -> El {
    let pane = &ui.meetings.requests;
    let mut chat = col().key("meeting-session").gap(14.0);
    let session = pane
        .open
        .and_then(|id| pane.sessions.iter().find(|session| session.id == id));
    if let Some(error) = pane
        .meeting
        .as_deref()
        .filter(|_| !pane.fixture)
        .and_then(|id| worker::error(&store::record(id)))
    {
        chat = chat.child(
            text(tr_fill("The question wasn't answered: {}", &error))
                .key("error")
                .font(DESCRIPTION)
                .color(p.danger())
                .wrap(),
        );
    }
    let Some(session) = session else {
        return chat.child(
            text(tr(
                "Ask about decisions, tasks, or who said what. Answers come from the transcript",
            ))
            .key("hint")
            .font(DESCRIPTION)
            .color(p.faint())
            .wrap(),
        );
    };
    let id = session.id;
    for (index, turn) in session.turns.iter().enumerate() {
        chat = chat
            .child(session::bubble(p, &format!("asked{index}"), &turn.question))
            .child(session::answer(
                p,
                Answered {
                    key: format!("answer{index}"),
                    question: &turn.question,
                    answer: &turn.answer,
                    model: &turn.model,
                    created_ms: turn.created_ms,
                    stopped: turn.stopped,
                },
                move |ui: &mut Ui, question| {
                    ask(ui, Some(id), question);
                },
            ));
    }
    if let Some(task) = running(ui, id) {
        chat = chat.children(session::answering(p, task));
    }
    chat
}

/// The tab's text with the field floating over its bottom, as Files has it.
/// On Transcript and Summary the field always starts a new session, as
/// Files' field does on a version, and the tab moves to Requests.
pub(in super::super) fn stage(
    ui: &Ui,
    p: Paint,
    body: El,
    tab: MeetingTab,
    layer: &mut Vec<El>,
) -> El {
    let session = if tab == MeetingTab::Requests {
        ui.meetings.requests.open
    } else {
        None
    };
    let pane = &ui.meetings.requests;
    let field_h = if pane.composer_h > 0.0 {
        pane.composer_h
    } else {
        FIELD_GUESS
    };
    let room = field_h + FIELD_CLEARANCE + FIELD_BOTTOM;
    let bg = p.k.c.bg_base;
    let body = body.child(el().key("field-room").h(room));
    let text = ld::card_scroll("meeting-requests", body)
        .on_at_bottom(|ui: &mut Ui, at: bool| ui.meetings.requests.text_under = !at);
    let veil = el()
        .key("field-veil")
        .pin(Some(0.0), None, Some(0.0), Some(0.0))
        .h(room + 32.0)
        .linear_gradient(
            std::f32::consts::PI,
            [
                (0.0, bg.alpha(0.0)),
                (0.3, bg.alpha(0.7)),
                (0.55, bg.alpha(0.85)),
                (1.0, bg.alpha(0.95)),
            ],
        );
    let ask = col()
        .key("meeting-ask-area")
        .gap(8.0)
        .child(super::chips::chips(ui, p, layer))
        .child(field(ui, p, session, layer))
        .pin(Some(0.0), None, Some(0.0), Some(FIELD_BOTTOM))
        .on_layout(|ui: &mut Ui, rect: [f32; 4]| {
            if (ui.meetings.requests.composer_h - rect[3]).abs() > 0.5 {
                ui.meetings.requests.composer_h = rect[3];
            }
        });
    col()
        .key("meeting-stage")
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .child(text)
        .child(veil)
        .child(ask)
}

/// Where a new session starts: its pills' choice, else Meetings ›
/// Settings › Questions run on, else (`None`) Files › Settings.
fn next(ui: &Ui) -> (Option<String>, Option<String>) {
    let pane = &ui.meetings.requests;
    let questions = &ui.settings.meetings.questions;
    let effort = Some(questions.effort.trim())
        .filter(|effort| !effort.is_empty())
        .map(str::to_owned);
    (
        pane.engine.clone().or_else(|| questions.engine.clone()),
        pane.effort.clone().or(effort),
    )
}

/// The engine `session` (or a new one) asks on, and its effort.
fn chosen(ui: &Ui, session: Option<u32>) -> (Option<String>, Option<String>) {
    let pane = &ui.meetings.requests;
    match session.and_then(|id| pane.sessions.iter().find(|session| session.id == id)) {
        Some(session) => (session.engine.clone(), session.effort.clone()),
        None => next(ui),
    }
}

/// The field: the question, the model and reasoning pills, and Send, or
/// Stop while `session`'s answer comes; with no session, a new one.
fn field(ui: &Ui, p: Paint, session: Option<u32>, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let pane = &ui.meetings.requests;
    let edit = &pane.ask;
    let task = session.and_then(|id| running(ui, id)).map(|task| task.id);
    let placeholder = tr("Ask about this meeting");
    let input = rune::text_input(edit, p.k.ty.ui)
        .key("input")
        .w_full()
        .color(c.text_base)
        .placeholder(placeholder, p.placeholder())
        .caret(c.text_base, p.k.selection())
        .on_edit(|ui: &mut Ui| &mut ui.meetings.requests.ask)
        .on_submit(move |ui: &mut Ui, _| send(ui, session));
    let text_area = rune::auto_grow(input, 1, composer::MAX_LINES).max_h(120.0);
    let (engine, effort) = chosen(ui, session);
    let engine =
        Config::from_settings(&ui.settings).request_engine(engine.as_deref(), effort.as_deref());
    let group = if session.is_some() {
        tr("Model for this session")
    } else {
        tr("Model for the next session")
    };
    let mut pills = row()
        .key("pills")
        .gap(2.0)
        .center_y()
        .child(composer::model_menu(
            ui,
            p,
            MODEL_MENU,
            group,
            &engine,
            layer,
            move |ui: &mut Ui, value: String| choose(ui, session, Some(value), None),
        ));
    if let Some(effort) = engine.effort() {
        pills = pills.child(composer::effort_menu(
            ui,
            p,
            EFFORT_MENU,
            effort,
            layer,
            move |ui: &mut Ui, value: String| choose(ui, session, None, Some(value)),
        ));
    }
    let action = match task {
        Some(task) => composer::round_button(
            p,
            "stop",
            tr("Stop"),
            &composer::STOP_ICON,
            true,
            move |_ui| worker::cancel(task),
        ),
        None => composer::round_button(
            p,
            "send",
            tr("Send"),
            &composer::SEND_ICON,
            !edit.text.trim().is_empty(),
            move |ui: &mut Ui| send(ui, session),
        ),
    };
    composer::shell(
        p,
        "meeting-composer",
        placeholder.to_owned(),
        text_area,
        pills,
        action,
        (edit.focused, pane.text_under),
    )
}

/// Saves a pill's choice: on `session`, or for the next new one.
fn choose(ui: &mut Ui, session: Option<u32>, engine: Option<String>, effort: Option<String>) {
    ui.open = None;
    let pane = &mut ui.meetings.requests;
    let (Some(id), Some(meeting)) = (session, pane.meeting.clone()) else {
        if engine.is_some() {
            pane.engine = engine;
        }
        if effort.is_some() {
            pane.effort = effort;
        }
        return;
    };
    let change = |sessions: &mut Vec<Session>| {
        if let Some(session) = sessions.iter_mut().find(|session| session.id == id) {
            if engine.is_some() {
                session.engine.clone_from(&engine);
            }
            if effort.is_some() {
                session.effort.clone_from(&effort);
            }
        }
    };
    if pane.fixture {
        change(&mut pane.sessions);
        return;
    }
    match store::update(&meeting, change) {
        Ok(()) => pane.sessions = store::load(&meeting),
        Err(error) => tracing::warn!("couldn't save the session's model: {error:#}"),
    }
}

/// Sends the field's question to `session`, or to a new one.
fn send(ui: &mut Ui, session: Option<u32>) {
    let question = ui.meetings.requests.ask.text.trim().to_owned();
    if question.is_empty() || session.is_some_and(|id| running(ui, id).is_some()) {
        return;
    }
    if ask(ui, session, question.clone()) {
        ui.meetings.requests.ask = TextEdit::default();
        super::chips::remember(ui, &question);
    }
}

/// Asks `question` in session `session`, or in a new session named by it.
pub(super) fn ask(ui: &mut Ui, session: Option<u32>, question: String) -> bool {
    let Some(meeting) = ui
        .meetings
        .selected
        .as_deref()
        .and_then(|id| ui.meetings.meetings.iter().find(|meeting| meeting.id == id))
    else {
        return false;
    };
    if session.is_some_and(|id| running(ui, id).is_some()) {
        return false;
    }
    let id = meeting.id.clone();
    let transcript = transcript(meeting);
    let (engine, effort) = next(ui);
    // The chip's label, also asked again or typed, cleans the turns.
    let tidy = (question.trim() == tr(TIDY)).then(|| lines(meeting));
    let prompt = super::chips::prompt(ui, &question).filter(|_| tidy.is_none());
    let pane = &mut ui.meetings.requests;
    if pane.fixture {
        return false;
    }
    let session = match session {
        Some(session) => session,
        None => {
            let name = if tidy.is_some() || prompt.is_some() {
                question.trim().to_owned()
            } else {
                worker::request_name(&question)
            };
            match store::start(&id, &name, engine, effort) {
                Ok(session) => session,
                Err(error) => {
                    tracing::warn!("couldn't start a session: {error:#}");
                    return false;
                }
            }
        }
    };
    let job = Job::Meeting {
        session,
        question,
        prompt,
        transcript,
        tidy,
    };
    if worker::submit(
        &store::record(&id),
        job,
        Config::from_settings(&ui.settings),
        false,
    ) == 0
    {
        return false;
    }
    let pane = &mut ui.meetings.requests;
    pane.sessions = store::load(&id);
    pane.open = Some(session);
    ui.meetings.tab = Some(MeetingTab::Requests);
    pane.tasks = worker::tasks()
        .into_iter()
        .filter(|task| store::meeting_of(&task.record).is_some())
        .collect();
    true
}

/// The meeting's turns for a tidy: who, when, and the words.
fn lines(meeting: &Meeting) -> Vec<tidy::Line> {
    meeting
        .turns()
        .iter()
        .map(|turn| tidy::Line {
            speaker: tr(if turn.mine { "You" } else { "Them" }).to_owned(),
            clock: clock_offset(turn.start_ms),
            text: turn.text.clone(),
        })
        .collect()
}

/// What the model reads: the meeting's title, when it was, and its
/// transcript with who spoke and when.
fn transcript(meeting: &Meeting) -> String {
    let status = match meeting.status {
        Status::Interrupted | Status::Failed => {
            "\n(A draft recovered from an interrupted recording.)"
        }
        _ => "",
    };
    format!(
        "Meeting: {}\nWhen: {}, {}{status}\n\n{}",
        meeting.title,
        meeting.day,
        meeting.clock,
        transcript_markdown(meeting.turns())
    )
}

/// The scene's sessions: two about the meeting, the newer one open.
pub(in super::super) fn fixture(ui: &mut Ui, meeting: &str) {
    use crate::files::store::FollowUp;
    let now = crate::files::store::now_ms();
    let turn = |question: &str, answer: &str, model: &str, ago: u64| FollowUp {
        question: question.into(),
        answer: answer.into(),
        model: model.into(),
        seconds: 6,
        created_ms: now.saturating_sub(ago * 60_000),
        stopped: false,
    };
    let sessions = vec![
        Session {
            id: 1,
            name: "Action items".into(),
            turns: vec![turn(
                "Action items",
                "- Prototype the gravity well in WebGPU\n- Twenty to thirty games in a couple of months",
                "GPT-6-Luna",
                40,
            )],
            created_ms: now.saturating_sub(41 * 60_000),
            ..Session::default()
        },
        Session {
            id: 2,
            name: "Who is waiting on whom?".into(),
            turns: vec![
                turn(
                    "Who is waiting on whom?",
                    "The games wait on the shared login (00:01). Nobody waits on the gravity well prototype; it can start now (00:31).",
                    "GPT-6-Luna",
                    6,
                ),
                turn(
                    "Draft a follow-up message",
                    "Thanks for today. Next: one login for every Arcade game, then the gravity well prototype in WebGPU. Shout if the timing is tight.",
                    "GPT-6.1-Sol",
                    3,
                ),
            ],
            engine: Some("codex:gpt-6.1-sol".into()),
            effort: Some("medium".into()),
            created_ms: now.saturating_sub(7 * 60_000),
        },
    ];
    ui.meetings.requests = RequestsPane {
        meeting: Some(meeting.into()),
        open: Some(2),
        sessions,
        fixture: true,
        ..RequestsPane::default()
    };
}
