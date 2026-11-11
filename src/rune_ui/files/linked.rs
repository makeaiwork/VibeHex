//! Requests' questions that reach past their recording, in the chat: a
//! meeting's or a file's Requests session is the Assistant tied to one item,
//! and a question there may also read the person's other meetings, Files
//! texts, and dictations (`assistant::intent::beyond`). This holds what
//! both panes share, as the Assistant draws it (`assistant::recall`):
//!
//! - before anything is read, while the person hasn't allowed reading
//!   recordings, the Assistant's consent card naming the model, with Allow
//!   (remembered in `assistant.read_recordings`) and Not now (this item
//!   only, and the answer says so in one line);
//! - while it works, the search's steps, then the draft with its `[n]`;
//! - a finished answer's steps folded into one grey line over it ("Read 2
//!   other meetings and 1 dictation in 6 s ›"), its `[n]` as marks, and
//!   "From your other recordings" chips under it. A mark or a chip opens its
//!   place with the Assistant's jump light, and Back to Requests returns to
//!   this session where it was scrolled (`jump::Origin::Requests`).
//!
//! Files' own part: its questions' reach, the card over the field, and
//! the way back to a file's session.

use std::rc::Rc;

use rune::{El, Tx, col, text};

use super::super::Ui;
use super::super::assistant::jump::{self, Origin};
use super::super::assistant::recall::{self, Open};
use super::super::controls::Paint;
use super::super::sidebar::Pane;
use super::session::{self, Answered};
use super::{Tab, View};
use crate::assistant::intent::beyond::{self, Item};
use crate::assistant::recordings::{Allowed, Kind, Ref, Sources};
use crate::assistant::worker::{Recall, Search};
use crate::desktop::i18n::tr;
use crate::files::rewrite::Engine;
use crate::files::store::{FileRecord, Linked, VersionKind};
use crate::files::worker::Task;

/// A question waiting for leave to read the person's other recordings.
#[derive(Clone, Debug, PartialEq)]
pub(in super::super) struct Pending {
    /// The session it goes to; none starts one (on a file, from version
    /// `from`).
    pub(in super::super) session: Option<u32>,
    pub(in super::super) from: Option<u32>,
    pub(in super::super) question: String,
    /// It has been scrolled into view once.
    pub(in super::super) shown: bool,
}

/// What `question`, asked about `item` (recording `id` of `kind`) after
/// `earlier` questions, may read past it. None: it surely reaches past
/// it and the person hasn't allowed reading their recordings, so the
/// card asks first. A question about the item alone reads nothing else.
pub(in super::super) fn recall(
    ui: &Ui,
    (kind, id): (Kind, &str),
    item: &Item<'_>,
    question: &str,
    earlier: &[&str],
) -> Option<Recall> {
    let now = crate::desktop::events::now_ms();
    let Some(intent) = beyond::carried(question, earlier, item, now) else {
        return Some(Recall::Guide);
    };
    match Allowed::from_settings(&ui.settings.assistant) {
        Some(allowed) => Some(match Sources::here() {
            Some(sources) => Recall::Search(Box::new(Search {
                allowed,
                sources: sources.without(kind, id),
                intent,
                vocabulary: ui.settings.vocabulary.clone(),
                now_ms: now,
            })),
            None => Recall::Guide,
        }),
        None if intent.sure => None,
        None => Some(Recall::Guide),
    }
}

/// Remembers Allow (`assistant.read_recordings`); Not now changes nothing
/// and answers from the item alone.
pub(in super::super) fn remember(ui: &mut Ui, allow: bool) {
    if allow {
        ui.update(|settings| settings.assistant.read_recordings = Some(true));
    }
}

/// The question waiting and the Assistant's card, the passages going to
/// `engine`'s model; `decide` takes Allow (true) or Not now. The first time
/// it is laid out it scrolls to the top of the pane (`held` reaches the pane's
/// [`Pending`]).
pub(in super::super) fn consent(
    p: Paint,
    pending: &Pending,
    engine: &Engine,
    decide: impl Fn(&mut Ui, bool) + Clone + 'static,
    held: fn(&mut Ui) -> Option<&mut Pending>,
) -> El {
    col()
        .key("waiting")
        .gap(14.0)
        .child(session::bubble(p, "question", &pending.question))
        .child(card(p, engine, decide))
        .on_layout_tx(move |ui: &mut Ui, _, tx: &mut Tx| {
            if let Some(pending) = held(ui).filter(|pending| !pending.shown) {
                pending.shown = true;
                tx.scroll_key_to("waiting", 24.0);
            }
        })
}

/// The card alone, with the line saying why.
pub(in super::super) fn card(
    p: Paint,
    engine: &Engine,
    decide: impl Fn(&mut Ui, bool) + Clone + 'static,
) -> El {
    col()
        .key("reach")
        .gap(10.0)
        .child(
            text(tr(
                "This question reaches past this recording. To answer, I need to read your other recordings.",
            ))
            .key("why")
            .font(p.k.ty.ui)
            .color(p.reading())
            .wrap(),
        )
        .child(recall::consent_card(p, engine, tr("Not now"), decide))
}

/// Where one answer of a session is: `origin`'s turn `index` (a file's
/// first answer is 0).
pub(in super::super) type At = (Origin, usize);

/// An answer and its row; one that read other recordings also folds its
/// steps over it (`unfolded` lists them, `toggle` opens or closes them),
/// marks its `[n]`, and lists the places it cites.
pub(in super::super) fn answer(
    p: Paint,
    answered: Answered<'_>,
    (linked, seconds): (&Linked, u64),
    (at, unfolded, toggle): (At, bool, impl Fn(&mut Ui) + 'static),
    again: impl Fn(&mut Ui, String) + 'static,
) -> El {
    if linked.refs.is_empty() && linked.steps.is_empty() {
        return session::answer(p, answered, again);
    }
    let open = opener(at);
    let body = if answered.answer.trim().is_empty() {
        text(tr("This version is empty."))
            .key("empty")
            .font(p.k.ty.ui)
            .color(p.faint())
    } else {
        recall::answer_with(p, answered.answer, Some(open.clone()), None, (false, false))
    };
    let chips = recall::chips_with(
        p,
        &linked.refs,
        answered.now_ms,
        tr("From your other recordings"),
        open,
    );
    col()
        .key(answered.key.clone())
        .gap(6.0)
        .children(recall::steps_folded(
            p,
            (&linked.steps, true),
            None,
            seconds as f32,
            unfolded,
            toggle,
        ))
        .child(body)
        .children(chips.map(|chips| chips.mt(2.0)))
        .child(session::answer_row(p, &answered, again))
}

/// While a question that reaches past its recording is answered: its
/// steps, then the draft with its marks. None for any other question.
pub(in super::super) fn coming(p: Paint, task: &Task) -> Option<Vec<El>> {
    if task.steps.is_empty() && !task.searching {
        return None;
    }
    let draft = task
        .draft
        .as_ref()
        .map(|draft| draft.text.trim_end())
        .filter(|draft| !draft.is_empty());
    let mut shown = Vec::new();
    if let Some(question) = &task.question {
        shown.push(session::bubble(p, "asking", question));
    }
    shown.extend(recall::live_steps(
        p,
        &task.steps,
        task.searching,
        draft.is_some(),
    ));
    if let Some(draft) = draft {
        shown.push(recall::answer_with(
            p,
            &format!("{draft}▌"),
            None,
            None,
            (true, true),
        ));
    }
    Some(shown)
}

/// Opens the places of the answer at `at`, from the session as its pane
/// holds it; Back returns there.
fn opener((origin, index): At) -> Open {
    Rc::new(move |ui: &mut Ui, n, mark, tx: &mut Tx| {
        let Origin::Requests { kind, id, session } = &origin else {
            return;
        };
        let found = match kind {
            Kind::Meeting => super::super::meeting_tabs::linked::cited(ui, id, *session, index),
            Kind::File => cited(ui, id, *session, index),
            Kind::Dictation => None,
        };
        if let Some((question, refs, scroll_y)) = found {
            jump::open_from(
                ui,
                (origin.clone(), scroll_y),
                (&question, &refs),
                n,
                mark,
                tx,
            );
        }
    })
}

/// Answer `index` of session `session` in file `id`: what it was asked,
/// the places it cites, and how far the session is scrolled.
fn cited(ui: &Ui, id: &str, session: u32, index: usize) -> Option<(String, Vec<Ref>, f32)> {
    let version = ui.files.record(id)?.version(session)?;
    let (question, linked) = match index {
        0 => (
            version
                .prompt
                .clone()
                .unwrap_or_else(|| version.name.clone()),
            &version.linked,
        ),
        index => {
            let asked = version.follow_ups.get(index - 1)?;
            (asked.question.clone(), &asked.linked)
        }
    };
    Some((question, linked.refs.clone(), ui.files.scroll_y.get()))
}

/// Back to file `id`'s session `session`, scrolled to `scroll_y`.
pub(in super::super) fn reopen(ui: &mut Ui, id: &str, session: u32, scroll_y: f32, tx: &mut Tx) {
    ui.select_pane(Pane::Files, tx);
    ui.files.view = View::default();
    ui.files.select(id.to_owned());
    ui.files.tab = Some(Tab::Version(session));
    tx.scroll_to_y("file-text", scroll_y);
}

/// File `record` asked about from version `from`'s text: what the reach
/// reads it against.
pub(in super::super) fn item(record: &FileRecord, from: u32) -> Item<'_> {
    let text = record
        .version(from)
        .map(|version| version.text.as_str())
        .unwrap_or_default();
    Item {
        kind: Kind::File,
        start_ms: record.added_ms,
        end_ms: record.added_ms,
        title: &record.name,
        text,
    }
}

/// What a question on file `record` may read past it, asked in `session`
/// (or starting one from version `from`); None: the card asks first, and
/// the question waits in `ui.files.consent`.
pub(in super::super) fn files_recall(
    ui: &mut Ui,
    record: &FileRecord,
    (session, from): (Option<u32>, u32),
    question: &str,
) -> Option<Recall> {
    let earlier: Vec<&str> = session
        .and_then(|id| record.version(id))
        .map(|version| {
            let first = version.prompt.as_deref().unwrap_or(version.name.as_str());
            std::iter::once(first)
                .chain(
                    version
                        .follow_ups
                        .iter()
                        .map(|asked| asked.question.as_str()),
                )
                .collect()
        })
        .unwrap_or_default();
    let reach = recall(
        ui,
        (Kind::File, &record.id),
        &item(record, from),
        question,
        &earlier,
    );
    if reach.is_none() {
        ui.files.consent = Some(Pending {
            session,
            from: Some(from),
            question: question.to_owned(),
            shown: false,
        });
    }
    reach
}

/// The question waiting for leave in `session` (none: a new session) of
/// the file shown, and the card.
pub(in super::super) fn waiting(ui: &Ui, p: Paint, session: Option<u32>) -> Vec<El> {
    let Some(pending) = ui
        .files
        .consent
        .as_ref()
        .filter(|pending| pending.session == session)
    else {
        return Vec::new();
    };
    let (engine, effort) = match session.and_then(|id| ui.files.selected()?.version(id)) {
        Some(version) => (version.engine.clone(), version.effort.clone()),
        None => (ui.files.ask_engine.clone(), ui.files.ask_effort.clone()),
    };
    let engine = crate::files::worker::Config::from_settings(&ui.settings)
        .request_engine(engine.as_deref(), effort.as_deref());
    match session {
        Some(_) => vec![consent(p, pending, &engine, decide, |ui| {
            ui.files.consent.as_mut()
        })],
        // Over the field: the question isn't a session's yet.
        None => vec![card(p, &engine, decide)],
    }
}

/// The card's choice on a file: the question waiting asks, reading the
/// person's other recordings once allowed, or this file alone.
fn decide(ui: &mut Ui, allow: bool) {
    let Some(pending) = ui.files.consent.take() else {
        return;
    };
    if ui.is_fixture() || ui.files.fixture {
        return;
    }
    remember(ui, allow);
    let Some(record) = ui.files.selected().cloned() else {
        return;
    };
    let from = pending.from.unwrap_or_default();
    let recall = if allow {
        files_recall(ui, &record, (pending.session, from), &pending.question)
            .unwrap_or(Recall::Guide)
    } else {
        Recall::Declined
    };
    match pending.session {
        Some(session) => {
            let job = crate::files::worker::Job::FollowUp {
                session,
                question: pending.question,
                recall,
            };
            super::submit(ui, &record.id, job);
            ui.files.stale = true;
        }
        None => {
            let shown = match ui.files.tab {
                Some(Tab::Version(shown)) => shown,
                _ => from,
            };
            ui.files.ask = rune::TextEdit::default();
            super::composer::start_reaching(ui, &record, shown, (pending.question, None), recall);
        }
    }
}

/// The version a question on `record` reads: a session's source, or the
/// version itself.
pub(in super::super) fn source(record: &FileRecord, version: u32) -> u32 {
    match record.version(version) {
        Some(found) if found.kind == VersionKind::Request => found.from.unwrap_or(version),
        _ => version,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(text: &str) -> Item<'_> {
        Item {
            kind: Kind::Meeting,
            start_ms: 1_700_000_000_000,
            end_ms: 1_700_000_600_000,
            title: "Arcade sync",
            text,
        }
    }

    #[test]
    fn a_question_past_its_recording_waits_for_leave_to_read_others() {
        let mut ui = Ui::fixture_base();
        ui.settings.assistant.read_recordings = None;
        let text = "We agreed to build one login for every Arcade game first.";
        let at = (Kind::Meeting, "1700-a");
        let past = "Что мы решили про логин на прошлой встрече?";
        // Not allowed yet: the card asks first.
        assert!(recall(&ui, at, &item(text), past, &[]).is_none());
        // About this meeting alone: nothing else is read, nothing asked.
        let here = "Переведи это на английский";
        assert!(matches!(
            recall(&ui, at, &item(text), here, &[]),
            Some(Recall::Guide)
        ));
        // Allowed: it searches, sure it reaches past, leaving this out.
        ui.settings.assistant.read_recordings = Some(true);
        match recall(&ui, at, &item(text), past, &[]) {
            Some(Recall::Search(search)) => assert!(search.intent.sure),
            Some(Recall::Guide) => {} // no data folder where the test runs
            other => panic!("expected a search, got {other:?}"),
        }
        // A follow-up keeps the reach of the question before it.
        assert!(matches!(
            recall(&ui, at, &item(text), "ещё", &[past]),
            Some(Recall::Search(_) | Recall::Guide)
        ));
        ui.settings.assistant.read_recordings = None;
        assert!(recall(&ui, at, &item(text), "ещё", &[past]).is_none());
    }
}
