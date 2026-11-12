//! A meeting's Requests questions that reach past the meeting
//! (`rune_ui::files::linked` draws them): what one may read, the consent
//! card's choice, the places an answer cites as its session holds them, and
//! the way back to the session, where it was scrolled.

use rune::Tx;

use super::super::Ui;
use super::super::files::linked::{self, Pending};
use super::super::meetings::{Meeting, View};
use super::super::sidebar::Pane;
use super::MeetingTab;
use super::requests::ask_with;
use crate::assistant::intent::beyond::Item;
use crate::assistant::recordings::{Kind, Ref};
use crate::assistant::worker::Recall;
use crate::meeting::requests::{self as store, Session};

/// What `question` on `meeting`, asked in `session` (or a new one), may
/// read past it, the meeting's `transcript` its text. None: the card asks
/// first, and the question waits in the pane.
pub(super) fn recall(
    ui: &mut Ui,
    meeting: &Meeting,
    transcript: &str,
    session: Option<u32>,
    question: &str,
) -> Option<Recall> {
    let earlier: Vec<String> = session
        .and_then(|id| {
            let pane = &ui.meetings.requests;
            pane.sessions.iter().find(|session| session.id == id)
        })
        .map(|session| {
            session
                .turns
                .iter()
                .map(|turn| turn.question.clone())
                .collect()
        })
        .unwrap_or_default();
    let earlier: Vec<&str> = earlier.iter().map(String::as_str).collect();
    let item = Item {
        kind: Kind::Meeting,
        start_ms: meeting.start_ms,
        end_ms: meeting.start_ms + meeting.duration_ms,
        title: &meeting.title,
        text: transcript,
    };
    let reach = linked::recall(
        ui,
        (Kind::Meeting, &meeting.id),
        &item,
        question,
        earlier.as_slice(),
    );
    if reach.is_none() {
        ui.meetings.requests.consent = Some(Pending {
            session,
            from: None,
            question: question.to_owned(),
            shown: false,
        });
    }
    reach
}

/// The card's choice: the question waiting asks, reading the person's
/// other recordings once allowed, or this meeting alone.
pub(super) fn decide(ui: &mut Ui, allow: bool) {
    let Some(pending) = ui.meetings.requests.consent.take() else {
        return;
    };
    if ui.is_fixture() || ui.meetings.requests.fixture {
        return;
    }
    linked::remember(ui, allow);
    let meeting = ui
        .meetings
        .selected
        .as_deref()
        .and_then(|id| ui.meetings.meetings.iter().find(|meeting| meeting.id == id))
        .cloned();
    let recall = match meeting {
        Some(meeting) if allow => {
            let transcript = super::requests::transcript(&meeting);
            recall(
                ui,
                &meeting,
                &transcript,
                pending.session,
                &pending.question,
            )
            .unwrap_or(Recall::Guide)
        }
        _ => Recall::Declined,
    };
    if ask_with(ui, pending.session, pending.question.clone(), recall) {
        super::chips::remember(ui, &pending.question);
    }
}

/// Answer `index` of session `session` in meeting `id`: what it was asked,
/// the places it cites, and how far the session is scrolled.
pub(in super::super) fn cited(
    ui: &Ui,
    id: &str,
    session: u32,
    index: usize,
) -> Option<(String, Vec<Ref>, f32)> {
    let pane = &ui.meetings.requests;
    if pane.meeting.as_deref() != Some(id) {
        return None;
    }
    let turn = pane
        .sessions
        .iter()
        .find(|kept| kept.id == session)?
        .turns
        .get(index)?;
    Some((
        turn.question.clone(),
        turn.linked.refs.clone(),
        pane.scroll_y.get(),
    ))
}

/// Back to meeting `id`'s session `session` on Requests, scrolled to
/// `scroll_y`.
pub(in super::super) fn reopen(ui: &mut Ui, id: &str, session: u32, scroll_y: f32, tx: &mut Tx) {
    ui.select_pane(Pane::Meetings, tx);
    ui.meetings.view = View::Recordings;
    ui.meetings.select(id.to_owned());
    ui.meetings.tab = Some(MeetingTab::Requests);
    let pane = &mut ui.meetings.requests;
    if pane.meeting.as_deref() != Some(id) && !pane.fixture {
        pane.sessions = store::load(id);
        pane.meeting = Some(id.to_owned());
    }
    pane.open = Some(session);
    tx.scroll_to_y("meeting-requests", scroll_y);
}

/// The linked scenes' now, a Thursday as Files' scenes have it: the cited
/// standup was that Tuesday, the retro a week before it.
const SCENE_NOW_MS: u64 = 1_793_891_400_000;

/// The scene's: Requests' sessions with a third open, its answer having
/// read two other meetings and a dictation, citing each; with `consent`,
/// a new session's question waits for the card instead.
pub(in super::super) fn fixture(ui: &mut Ui, meeting: &str, consent: bool) {
    // A fixed moment: the cited recordings' weekdays and dates would
    // otherwise draw differently every day.
    ui.meetings.requests.fixed_now = Some(SCENE_NOW_MS);
    super::requests::fixture(ui, meeting);
    let now = SCENE_NOW_MS;
    let pane = &mut ui.meetings.requests;
    pane.sessions.push(session(now));
    pane.open = Some(3);
    if consent {
        pane.sessions.retain(|session| session.id != 3);
        pane.open = None;
        pane.consent = Some(Pending {
            session: None,
            from: None,
            question: "Как мы решили с логином на прошлой встрече?".into(),
            shown: false,
        });
        ui.settings.assistant.read_recordings = None;
    }
}

fn session(now: u64) -> Session {
    use crate::assistant::recordings::Step;
    use crate::files::store::{FollowUp, Linked};
    let day = 86_400_000;
    let cite = |n: u32, kind: Kind, id: &str, title: &str, ago: u64| Ref {
        n,
        kind,
        id: id.into(),
        title: title.into(),
        at_ms: now.saturating_sub(ago),
        offset_ms: (kind == Kind::Meeting).then_some(31_000),
        lines: Vec::new(),
        notes: false,
    };
    let turn = FollowUp {
        question: "Как мы решили с логином на прошлых встречах?".into(),
        answer: "Здесь вы договорились сначала сделать общий логин для всех игр Arcade \
                 (00:01). Это не новое решение: на планёрке во вторник его уже назначили \
                 первым шагом [1], а неделей раньше хотели отложить до прототипа на \
                 WebGPU [2]. Вечером вы надиктовали, что логин держит три игры из пяти [3]."
            .into(),
        model: "GPT-6-Luna".into(),
        seconds: 6,
        created_ms: now.saturating_sub(4 * 60_000),
        stopped: false,
        linked: Linked {
            refs: vec![
                cite(
                    1,
                    Kind::Meeting,
                    "preview-standup",
                    "Планёрка Arcade",
                    2 * day,
                ),
                cite(2, Kind::Meeting, "preview-retro", "Ретро", 9 * day),
                cite(3, Kind::Dictation, "7", "Notes", day),
            ],
            steps: vec![
                Step::Meetings {
                    searched: 9,
                    found: 2,
                },
                Step::Read {
                    notes: 1,
                    passages: 3,
                },
                Step::Dictations {
                    searched: 14,
                    found: 1,
                },
            ],
        },
    };
    Session {
        id: 3,
        name: "Логин на прошлых встречах".into(),
        turns: vec![turn],
        engine: Some("codex:gpt-6-luna".into()),
        effort: Some("medium".into()),
        created_ms: now.saturating_sub(5 * 60_000),
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::assistant::jump::{self, Origin};
    use super::*;

    #[test]
    fn back_from_a_place_an_answer_cites_returns_to_its_requests_session() {
        let mut ui = Ui::fixture_base();
        fixture(&mut ui, "preview-arcade", false);
        ui.meetings.select("preview-arcade".into());
        let (question, refs, scroll_y) = cited(&ui, "preview-arcade", 3, 0).expect("the answer");
        assert_eq!(refs.len(), 3);
        let origin = Origin::Requests {
            kind: Kind::Meeting,
            id: "preview-arcade".into(),
            session: 3,
        };
        let mut tx = Tx::default();
        // [3] is a dictation: History opens, and Back says where it goes.
        jump::open_from(
            &mut ui,
            (origin.clone(), scroll_y),
            (&question, &refs),
            3,
            Some(0),
            &mut tx,
        );
        assert_eq!(ui.pane, Pane::History);
        assert_eq!(jump::from_requests(&ui), Some(&origin));
        ui.meetings.requests.open = Some(1);
        ui.meetings.tab = None;
        jump::back(&mut ui, &mut tx);
        assert_eq!(ui.pane, Pane::Meetings);
        assert_eq!(ui.meetings.selected.as_deref(), Some("preview-arcade"));
        assert_eq!(ui.meetings.tab, Some(MeetingTab::Requests));
        assert_eq!(ui.meetings.requests.open, Some(3));
        assert!(ui.came_from.is_none());
        // An answer that isn't there opens nothing.
        assert!(cited(&ui, "preview-arcade", 9, 0).is_none());
        assert!(cited(&ui, "another", 3, 0).is_none());
    }
}
