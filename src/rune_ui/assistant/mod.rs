//! The Assistant pane: a chat that answers questions about VibeHex from
//! its guide (`crate::assistant`), and its settings. It sits at the bottom
//! of the sidebar on every OS, on by default and not in Labs, an exception
//! the owner chose. What reaches a model is the question, the conversation
//! so far, and guide text; never history, meetings, or files.
//!
//! One question is answered at a time, on the worker's own thread; its
//! draft streams into the feed and the finished turn lands in the store.
//! The model is Assistant's own: choosing one here never changes what
//! dictation's AI cleanup runs on.

mod ask;
mod card;
mod card_diff;
mod changes;
mod composer;
mod connect;
pub(super) mod dots;
pub(super) mod face;
#[cfg(test)]
mod facts_tests;
mod feed;
pub(super) mod fixture;
pub(super) mod fixture_cards;
pub(super) mod jump;
mod knew;
pub(super) mod nav;
mod picker;
pub(super) mod places;
mod plan;
mod preset;
pub(super) mod recall;
mod returning;
mod settings;
pub(super) mod setup_fixture;
pub(super) mod strong;
mod tick;
mod verbs;
mod vocab;
mod vocab_edit;
mod web;
mod whats_new;

use rune::{El, ScrollSpec, TextEdit, Tx, col};

use super::controls::{METRICS, Paint, segmented_tx};
use super::sidebar::Pane;
use super::{Ui, lists};
use crate::app_settings::AppSettings;
use crate::assistant::store::{Conversation, Store};
use crate::assistant::worker::{self, Answering};
use crate::desktop::i18n::tr;
use crate::files::rewrite::Engine;

/// The feed's and the field's width at most, centred in a wide window.
const COLUMN: f32 = 680.0;

/// Which half of the pane shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum View {
    #[default]
    Chat,
    Settings,
}

/// The pane's state.
#[derive(Default)]
pub(in super::super) struct AssistantPane {
    pub(super) view: View,
    /// The conversations, read when the pane first shows.
    pub(super) store: Store,
    /// The open conversation; `None` while a new one is written.
    pub(super) open: Option<u32>,
    pub(super) ask: TextEdit,
    /// The model and reasoning a new conversation starts on; `None`
    /// follows Assistant's settings.
    pub(super) engine: Option<String>,
    pub(super) effort: Option<String>,
    /// Whether a new conversation may search the web; `None` follows
    /// Assistant's settings.
    pub(super) web: Option<bool>,
    /// The answer coming, as the worker last reported it.
    pub(super) answering: Option<Answering>,
    /// Why a question couldn't be asked.
    pub(super) error: Option<String>,
    /// Whether Codex is installed and OpenCode runs.
    pub(super) reach: connect::Reach,
    /// A question about recordings waiting for the person's leave.
    pub(super) consent: Option<recall::Pending>,
    /// Answers whose search steps are open, by conversation and turn.
    pub(super) unfolded: Vec<(u32, usize)>,
    /// Answers whose "What Assistant knew" is open under their steps.
    pub(super) knew_open: Vec<(u32, usize)>,
    /// How far the conversation is scrolled, to come back to.
    pub(super) scroll_y: ScrollAt,
    /// The pane's height as last laid out, where a cited place lands by.
    pub(super) panel_h: f32,
    /// Clear conversations… was pressed once.
    pub(super) clear_armed: bool,
    /// The face's blink, squish, and reading eyes.
    pub(super) face: face::FaceState,
    /// The wave of a live step's three dots, wherever one shows.
    pub(super) dots: dots::Dots,
    /// Cards applied in the open conversation this session: Undo holds
    /// for them until another conversation opens (owner decision 2).
    pub(super) undo: Vec<changes::At>,
    /// Setup plans whose "Already set" is unfolded.
    pub(super) kept_open: Vec<changes::At>,
    /// Setup rounds whose "Answered before" is unfolded.
    before_open: Vec<ask::At>,
    /// The conversation `undo` belongs to.
    pub(super) undo_for: Option<u32>,
    /// The instructions card being edited before it saves, and its text.
    pub(super) editing: Option<changes::At>,
    pub(super) edit: TextEdit,
    /// The vocabulary card row being edited, and its two fields.
    pub(super) vocab_edit: Option<(changes::At, usize)>,
    pub(super) vocab_term: TextEdit,
    pub(super) vocab_sounds: TextEdit,
    /// A scene's card states this computer can't take on: an
    /// organization's lock, Codex signed out.
    pub(super) shown_as: Vec<(changes::At, crate::assistant::changes::state::Shown)>,
    loaded: bool,
    /// Whether the store keeps conversations, as last applied.
    kept: bool,
    woken: bool,
    /// A scene: nothing reads the disk or asks a model.
    pub(super) fixture: bool,
    /// A scene's screens and pointing devices; unknown otherwise.
    pub(super) hardware: crate::desktop::hardware::Hardware,
    /// A scene's clock, so its times read the same on any day.
    pub(super) fixed_now: Option<u64>,
    /// The version this install updated to lately, and when (`whats_new`).
    pub(super) updated: Option<(String, u64)>,
}

/// A scroll offset the feed keeps up to date.
#[derive(Clone)]
pub(super) struct ScrollAt(rune::Live<f32>);

impl Default for ScrollAt {
    fn default() -> Self {
        Self(rune::Live::new(0.0))
    }
}

impl ScrollAt {
    pub(super) fn get(&self) -> f32 {
        self.0.get()
    }

    /// Keeps it at a scroll's offset, for `on_scroll_offset`.
    pub(super) fn follow(&self) -> impl Fn(rune::ScrollPos) + 'static {
        let at = self.0.clone();
        move |pos: rune::ScrollPos| {
            at.set(pos.offset[1]);
        }
    }
}

impl AssistantPane {
    /// The open conversation.
    pub(super) fn conversation(&self) -> Option<&Conversation> {
        self.open.and_then(|id| self.store.get(id))
    }

    /// The time dates are told against: the clock, or a scene's.
    pub(super) fn now_ms(&self) -> u64 {
        self.fixed_now
            .unwrap_or_else(crate::desktop::events::now_ms)
    }

    /// The answer coming in conversation `id`.
    pub(super) fn answering_in(&self, id: Option<u32>) -> Option<&Answering> {
        self.answering
            .as_ref()
            .filter(|answering| Some(answering.conversation) == id)
    }
}

/// The engine a conversation asks on: its own choice (`engine`, `effort`),
/// else Assistant's settings, else what dictation's AI cleanup uses.
pub(super) fn engine(settings: &AppSettings, engine: Option<&str>, effort: Option<&str>) -> Engine {
    crate::assistant::facts::assistant_engine(settings, engine, effort)
}

/// The engine and reasoning the field asks with next: the open
/// conversation's, else the next new one's.
pub(super) fn chosen(ui: &Ui) -> (Option<String>, Option<String>) {
    let pane = &ui.assistant;
    match pane.conversation() {
        Some(conversation) => (conversation.engine.clone(), conversation.effort.clone()),
        None => (pane.engine.clone(), pane.effort.clone()),
    }
}

/// Chat or Settings, beside the title.
pub(super) fn view_switch(ui: &Ui, p: Paint) -> Option<El> {
    let selected = match ui.assistant.view {
        View::Chat => 0,
        View::Settings => 1,
    };
    Some(segmented_tx(
        p,
        "assistant-view",
        &[tr("Chat"), tr("Settings")],
        selected,
        |ui: &mut Ui, index: usize, tx: &mut Tx| {
            let view = if index == 0 {
                View::Chat
            } else {
                View::Settings
            };
            if ui.assistant.view != view {
                tx.settle();
                ui.open = None;
                ui.assistant.view = view;
            }
        },
    ))
}

/// The pane: its header actions and its body.
/// `still`: Reduce Motion is on, so the face holds still.
pub(super) fn pane(ui: &Ui, p: Paint, still: bool, layer: &mut Vec<El>) -> (Vec<El>, El) {
    match ui.assistant.view {
        View::Settings => (
            Vec::new(),
            lists::scroll_body(
                p.k,
                "assistant-settings-scroll",
                settings::view(ui, p, layer),
            ),
        ),
        View::Chat => (picker::actions(ui, p, layer), chat(ui, p, still, layer)),
    }
}

/// The feed over the field, both centred.
fn chat(ui: &Ui, p: Paint, still: bool, layer: &mut Vec<El>) -> El {
    // With no model to ask, the panel stands in for the field (and for the
    // greeting of a new conversation).
    let connect = connect::needed(ui);
    // The conversation is Assistant's: its own words in its language (the
    // dictation language when set); places it names keep the window's.
    let mut feed =
        crate::assistant::language::within(&ui.settings, || feed::feed(ui, p, !connect, still));
    if connect {
        feed = feed.child(connect::panel(ui, p));
    }
    let feed = feed.w_full().max_w(COLUMN);
    let scroll = col()
        .key("assistant-scroll")
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .clip_fade(0.0, 4.0, 0.0, 24.0)
        .scroll(ScrollSpec { stick_bottom: true })
        .on_scroll_offset({
            let at = ui.assistant.scroll_y.0.clone();
            move |pos: rune::ScrollPos| {
                at.set(pos.offset[1]);
            }
        })
        .child(
            col()
                .key("inset")
                .items_center()
                .px(METRICS.gutter)
                .pt(12.0)
                .pb(24.0)
                .child(feed),
        );
    col()
        .key("assistant-chat")
        .grow(1.0)
        .shrink()
        .min_h(0.0)
        .child(scroll)
        .children((!connect).then(|| {
            col()
                .key("assistant-ask")
                .items_center()
                .px(METRICS.gutter)
                .pb(16.0)
                .child(composer::field(ui, p, layer).w_full().max_w(COLUMN))
        }))
}

/// Follows the worker: a finished answer lands in its conversation, and a
/// draft as it streams. The conversations load when the pane first shows.
/// True when what shows changed.
pub(super) fn poll(ui: &mut Ui, tx: &mut Tx) -> bool {
    // The face ends its blink or squish wherever it shows, scenes included;
    // a scene's reading eyes rest where a line starts, off the clock.
    let reading = ui
        .assistant
        .answering
        .as_ref()
        .filter(|_| !ui.is_fixture())
        .map(|answering| answering.started);
    let mut changed =
        ui.assistant.face.poll(reading, tx) | jump::poll(ui, tx) | places::poll(ui, tx);
    // Scenes hold the dots still, so goldens stay the same.
    let live = !ui.is_fixture() && steps_live(ui);
    changed |= ui.assistant.dots.poll(live, tx);
    changes::follow_open(ui);
    if ui.is_fixture() || ui.assistant.fixture {
        return changed;
    }
    let keep = ui.settings.assistant.keep_conversations;
    let pane = &mut ui.assistant;
    if !pane.loaded {
        if ui.pane != Pane::Assistant {
            return changed;
        }
        pane.store = Store::open(keep);
        pane.open = newest(&pane.store);
        pane.updated = crate::assistant::releases::history::fresh(pane.now_ms());
        pane.kept = keep;
        pane.loaded = true;
        changed = true;
    }
    // Keeping turned on or off, here or from `hex settings`.
    if pane.kept != keep {
        pane.store.keep(keep);
        pane.kept = keep;
    }
    if !pane.woken
        && let Some(waker) = ui.modes.waker.clone()
    {
        worker::set_waker(move || waker());
        ui.assistant.woken = true;
    }
    let pane = &mut ui.assistant;
    for (id, turn) in worker::take_finished() {
        match pane.store.get(id) {
            Some(_) => pane.store.push(id, turn),
            None => pane.error = turn.error,
        }
        changed = true;
    }
    let answering = worker::answering();
    let draft = |answering: &Option<Answering>| {
        answering.as_ref().map(|answering| {
            (
                answering.conversation,
                answering.draft.len(),
                answering.cards_coming,
                answering.steps.len(),
                answering.web.progress(),
            )
        })
    };
    if draft(&answering) != draft(&pane.answering) {
        pane.answering = answering;
        changed = true;
    }
    changed | composer::poll(ui) | connect::poll(ui)
}

/// Whether a step line still going on shows anywhere: an answer's in
/// Assistant, or a question's in Files or Meetings' Requests.
pub(super) fn steps_live(ui: &Ui) -> bool {
    // Before the first words come, a line always goes on ("Looking in the
    // guide", a search, "No reply yet, asking again"); so does "Preparing a
    // change" while a card is read.
    let answering = ui.assistant.answering.as_ref().is_some_and(|answering| {
        answering.draft.trim().is_empty()
            || !answering.steps.is_empty()
            || answering.searching
            || answering.web.went_online()
            || answering.cards_coming
    });
    let task = |task: &crate::files::worker::Task| !task.steps.is_empty() || task.searching;
    answering || ui.files.tasks.iter().any(task) || ui.meetings.requests.tasks.iter().any(task)
}

/// The most recently used conversation.
fn newest(store: &Store) -> Option<u32> {
    store
        .conversations()
        .iter()
        .max_by_key(|conversation| conversation.updated_ms())
        .map(|conversation| conversation.id)
}
