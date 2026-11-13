//! The field: the question, Files' model and reasoning pills, the Web
//! toggle while the engine can search, and Send, which turns into Stop
//! while the open conversation's answer comes. The pills choose the engine
//! for the open conversation (kept with it), or for the next new one;
//! neither ever changes dictation's AI cleanup.

use rune::{El, TextEdit, el, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::files::composer::{self as files, MAX_LINES, SEND_ICON, STOP_ICON};
use crate::assistant::agent::access::Access;
use crate::assistant::intent::{self, Route};
use crate::assistant::prompt;
use crate::assistant::setup::{Stage, flow};
use crate::assistant::worker::{self, Earlier, Question, Recall};
use crate::desktop::i18n::tr;

pub(super) const MODEL_MENU: &str = "assistant-model";
const EFFORT_MENU: &str = "assistant-effort";
/// Why a question waits while an answer in another conversation is
/// parked on the card asking to read the recordings.
pub(super) const PARKED: &str = "An answer in another conversation is waiting for you to say whether it may read your recordings.";

/// Why a question can't be asked here now, while an answer comes in
/// another conversation: the model is answering there, or that answer
/// waits for the person on the consent card (with its conversation, which
/// the line opens). None while nothing answers elsewhere.
pub(super) fn elsewhere(ui: &Ui) -> Option<(&'static str, Option<u32>)> {
    let pane = &ui.assistant;
    let answering = pane.answering.as_ref()?;
    if Some(answering.conversation) == pane.open {
        return None;
    }
    Some(match answering.asking {
        true => (tr(PARKED), Some(answering.conversation)),
        false => (tr(flow::BUSY), None),
    })
}

/// The line over the field that says why Send waits, with a way to the
/// conversation that holds the card; none when a setup round open here
/// already says it beside its Continue.
pub(super) fn waits(ui: &Ui, p: Paint) -> Option<El> {
    let (words, parked) = elsewhere(ui)?;
    if super::ask::round_waits(ui) {
        return None;
    }
    let mut line = row().key("elsewhere").w_full().gap(8.0).center_y().child(
        text(words)
            .key("why")
            .font(p.k.ty.small)
            .color(p.muted())
            .wrap_anywhere()
            .shrink(),
    );
    if let Some(id) = parked {
        line = line
            .child(el().key("space").grow(1.0))
            .child(super::super::lists::small_button(
                p,
                "go",
                tr("Open that conversation"),
                move |ui, _| {
                    ui.open = None;
                    ui.assistant.open = Some(id);
                },
            ));
    }
    Some(line)
}

/// The field under the feed.
pub(super) fn field(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let pane = &ui.assistant;
    let edit = &pane.ask;
    // Under a plan still to decide, the field is for questions about it.
    // Assistant's invitation, in its language.
    let speaking = crate::assistant::language::speak(&ui.settings);
    let placeholder = if super::plan::waiting(ui) {
        tr("Ask about these changes")
    } else {
        tr("Ask about VibeHex or your recordings")
    };
    drop(speaking);
    let input = rune::text_input(edit, p.k.ty.ui)
        .key("input")
        .label(placeholder)
        .w_full()
        .color(c.text_base)
        .placeholder(placeholder, p.placeholder())
        .caret(c.text_base, p.k.selection())
        .on_edit(|ui: &mut Ui| &mut ui.assistant.ask)
        .on_submit(|ui: &mut Ui, _| send(ui));
    let text_area = rune::auto_grow(input, 1, MAX_LINES).max_h(120.0);
    let (engine, effort) = super::chosen(ui);
    let engine = super::engine(&ui.settings, engine.as_deref(), effort.as_deref());
    let group = if pane.open.is_some() {
        tr("Model for this conversation")
    } else {
        tr("Model for the next conversation")
    };
    let mut pills = row()
        .key("pills")
        .gap(2.0)
        .center_y()
        .child(files::model_menu(
            ui,
            p,
            MODEL_MENU,
            group,
            &engine,
            layer,
            |ui: &mut Ui, value: String| choose(ui, Some(value), None),
        ));
    if let Some(effort) = engine.effort() {
        pills = pills.child(files::effort_menu(
            ui,
            p,
            EFFORT_MENU,
            effort,
            layer,
            |ui: &mut Ui, value: String| choose(ui, None, Some(value)),
        ));
    }
    pills = pills.children(super::web::pill(ui, p, &engine));
    let action = if pane.answering_in(pane.open).is_some() {
        files::round_button(p, "stop", tr("Stop"), &STOP_ICON, true, |_| worker::stop())
    } else {
        let ready = !edit.text.trim().is_empty() && pane.answering.is_none();
        files::round_button(p, "send", tr("Send"), &SEND_ICON, ready, send)
    };
    files::shell(
        p,
        "assistant-composer",
        placeholder.to_owned(),
        text_area,
        pills,
        action,
        (edit.focused, false),
    )
}

/// Picks up Codex's models while the field's menu is open.
pub(super) fn poll(ui: &mut Ui) -> bool {
    files::poll_models(ui, MODEL_MENU)
}

/// Saves a pill's choice: with the open conversation, or for the next one.
fn choose(ui: &mut Ui, engine: Option<String>, effort: Option<String>) {
    ui.open = None;
    let pane = &mut ui.assistant;
    let Some(id) = pane.open else {
        if engine.is_some() {
            pane.engine = engine;
        }
        if effort.is_some() {
            pane.effort = effort;
        }
        return;
    };
    let Some(conversation) = pane.store.get(id) else {
        return;
    };
    let engine = engine.or_else(|| conversation.engine.clone());
    let effort = effort.or_else(|| conversation.effort.clone());
    pane.store.choose(id, engine, effort);
}

/// Sends the field's question.
fn send(ui: &mut Ui) {
    let question = ui.assistant.ask.text.trim().to_owned();
    if question.is_empty() {
        return;
    }
    if ask(ui, question) {
        ui.assistant.ask = TextEdit::default();
    }
}

/// Asks `question` in the open conversation, or in a new one named by it.
/// False when it wasn't asked: another answer is coming, or this is a
/// scene. A request to set VibeHex up asks setup's questions; a question
/// about the person's recordings that may not read them yet waits for the
/// consent card (and counts as asked).
pub(super) fn ask(ui: &mut Ui, question: String) -> bool {
    let Some(id) = conversation_for(ui, &question) else {
        return false;
    };
    let earlier = asked_before(ui, id);
    let earlier: Vec<&str> = earlier.iter().map(String::as_str).collect();
    match intent::route(&question, &earlier, &ui.settings, ui.assistant.now_ms()) {
        Route::Preset => {
            super::preset::begin(ui, question);
            true
        }
        Route::Setup => ask_with(ui, id, question, Recall::Guide, Stage::Ask),
        Route::Releases(asked) => ask_with(ui, id, question, Recall::Releases(asked), Stage::None),
        Route::Recordings(intent) => match super::recall::allowed(ui, id, &question, intent) {
            Some(recall) => ask_with(ui, id, question, recall, Stage::None),
            None => true,
        },
        Route::Guide => ask_with(ui, id, question, Recall::Guide, Stage::None),
    }
}

/// The questions `conversation` holds so far, oldest first.
pub(super) fn asked_before(ui: &Ui, conversation: u32) -> Vec<String> {
    ui.assistant
        .store
        .get(conversation)
        .map(|conversation| {
            conversation
                .turns
                .iter()
                .map(|turn| turn.question.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// Asks `question`, in the window's language, about what changed since
/// the version run before, from the release notes whatever its words.
pub(super) fn ask_releases(ui: &mut Ui, question: String) -> bool {
    let Some(id) = conversation_for(ui, &question) else {
        return false;
    };
    let asked = intent::releases(&question).unwrap_or_default();
    ask_with(ui, id, question, Recall::Releases(asked), Stage::None)
}

/// The conversation `question` goes to, opened: the open one, or a new one
/// named by it. None while another answer comes, or in a scene.
fn conversation_for(ui: &mut Ui, question: &str) -> Option<u32> {
    if ui.is_fixture() || ui.assistant.fixture || worker::answering().is_some() {
        return None;
    }
    let pane = &mut ui.assistant;
    pane.consent = None;
    let id = match pane.conversation() {
        Some(conversation) => conversation.id,
        None => {
            let id = pane
                .store
                .start(question, pane.engine.take(), pane.effort.take());
            if let Some(web) = pane.web.take() {
                pane.store.set_web(id, Some(web));
            }
            id
        }
    };
    pane.open = Some(id);
    pane.error = None;
    Some(id)
}

/// Asks `question` in conversation `id`, reading what `recall` allows, at
/// setup's `stage` if it is one.
pub(super) fn ask_with(
    ui: &mut Ui,
    id: u32,
    question: String,
    recall: Recall,
    setup: Stage,
) -> bool {
    let pane = &mut ui.assistant;
    let Some(conversation) = pane.store.get(id) else {
        return false;
    };
    // Earlier turns that answered, for the model to follow the thread, and
    // what became of the changes each suggested.
    let earlier = conversation
        .turns
        .iter()
        .filter(|turn| turn.error.is_none() && !turn.answer.trim().is_empty())
        .map(|turn| Earlier {
            question: turn.question.clone(),
            answer: turn.answer.clone(),
            changes: prompt::changes_line(&turn.cards, &turn.refused),
            recordings: turn.read_recordings(),
        })
        .collect();
    let engine = super::engine(
        &ui.settings,
        conversation.engine.as_deref(),
        conversation.effort.as_deref(),
    );
    let web = super::web::on(&ui.settings, conversation.web, &engine);
    pane.open = Some(id);
    pane.error = None;
    let changes = super::changes::context(ui);
    // What the model may read of the recordings if it asks to itself
    // (`assistant::agent`).
    let access = Access::of(&ui.settings, crate::assistant::recordings::Sources::here());
    let now_ms = ui.assistant.now_ms();
    let asked = worker::ask(
        Question {
            conversation: id,
            question,
            earlier,
            recall,
            changes: Some(changes),
            setup,
            web,
            access,
            now_ms,
        },
        engine,
    );
    let pane = &mut ui.assistant;
    pane.answering = worker::answering();
    asked
}

/// Starts a new conversation: the field empties and the start shows.
pub(super) fn new_conversation(ui: &mut Ui) {
    ui.open = None;
    let pane = &mut ui.assistant;
    pane.open = None;
    pane.ask = TextEdit::default();
    pane.engine = None;
    pane.effort = None;
    pane.web = None;
}
