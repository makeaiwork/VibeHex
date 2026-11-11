//! The field: the question, Files' model and reasoning pills, and Send,
//! which turns into Stop while the open conversation's answer comes. The
//! pills choose the engine for the open conversation (kept with it), or
//! for the next new one; neither ever changes dictation's AI cleanup.

use rune::{El, TextEdit, row};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::files::composer::{self as files, MAX_LINES, SEND_ICON, STOP_ICON};
use crate::assistant::setup::Stage;
use crate::assistant::worker::{self, Earlier, Question, Recall};
use crate::assistant::{intent, prompt};
use crate::desktop::i18n::tr;

pub(super) const MODEL_MENU: &str = "assistant-model";
const EFFORT_MENU: &str = "assistant-effort";

/// The field under the feed.
pub(super) fn field(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let pane = &ui.assistant;
    let edit = &pane.ask;
    // Under a plan still to decide, the field is for questions about it.
    let placeholder = if super::plan::waiting(ui) {
        tr("Ask about these changes")
    } else {
        tr("Ask about VibeHex or your recordings")
    };
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
    if intent::setup(&question) {
        return ask_with(ui, id, question, Recall::Guide, Stage::Ask);
    }
    // What changed in VibeHex: its release notes, never the recordings.
    if let Some(asked) = intent::releases(&question) {
        return ask_with(ui, id, question, Recall::Releases(asked), Stage::None);
    }
    match super::recall::recall(ui, id, &question) {
        Some(recall) => ask_with(ui, id, question, recall, Stage::None),
        None => true,
    }
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
        None => pane
            .store
            .start(question, pane.engine.take(), pane.effort.take()),
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
        })
        .collect();
    let engine = super::engine(
        &ui.settings,
        conversation.engine.as_deref(),
        conversation.effort.as_deref(),
    );
    pane.open = Some(id);
    pane.error = None;
    let changes = super::changes::context(ui);
    let asked = worker::ask(
        Question {
            conversation: id,
            question,
            earlier,
            recall,
            changes: Some(changes),
            setup,
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
}
