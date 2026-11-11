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
pub(super) mod face;
mod feed;
pub(super) mod fixture;
pub(super) mod fixture_cards;
pub(super) mod jump;
pub(super) mod nav;
mod picker;
mod plan;
mod recall;
mod settings;
pub(super) mod setup_fixture;
mod verbs;
mod vocab;

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
    /// How far the conversation is scrolled, to come back to.
    pub(super) scroll_y: ScrollAt,
    /// The pane's height as last laid out, where a cited place lands by.
    pub(super) panel_h: f32,
    /// Clear conversations… was pressed once.
    pub(super) clear_armed: bool,
    /// The face's blink, squish, and reading eyes.
    pub(super) face: face::FaceState,
    /// Cards applied in the open conversation this session: Undo holds
    /// for them until another conversation opens (owner decision 2).
    pub(super) undo: Vec<changes::At>,
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
    /// A scene's clock, so its times read the same on any day.
    pub(super) fixed_now: Option<u64>,
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
    let effort = effort.filter(|effort| !effort.trim().is_empty()).unwrap_or(
        match settings.assistant.effort.trim() {
            "" => "medium",
            effort => effort,
        },
    );
    let processing = &settings.dictation_processing;
    Engine::resolve(
        engine.or(settings.assistant.engine.as_deref()),
        &processing.server,
        settings.labs.ai_cleanup_server,
        &processing.default_mode,
    )
    .with_effort(effort)
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
    let mut feed = feed::feed(ui, p, !connect, still);
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
    let mut changed = ui.assistant.face.poll(reading, tx) | jump::poll(ui, tx);
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
            )
        })
    };
    if draft(&answering) != draft(&pane.answering) {
        pane.answering = answering;
        changed = true;
    }
    changed | composer::poll(ui) | connect::poll(ui)
}

/// The most recently used conversation.
fn newest(store: &Store) -> Option<u32> {
    store
        .conversations()
        .iter()
        .max_by_key(|conversation| conversation.updated_ms())
        .map(|conversation| conversation.id)
}
