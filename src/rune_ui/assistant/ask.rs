//! Setup's questions as option chips (the Setup board): the Start screen's
//! "Set VibeHex up for me" draws the app's own at once, with no model call;
//! a typed request gets the model's, in the message's language.
//!
//! One question shows at a time. A single choice answers on a click; a
//! multiple choice takes a small Continue. Answered questions stay as
//! their chosen chips, with no bubble, and once every one is answered the
//! answers go to the model as one turn (`Stage::Plan`), never as typed
//! text.

use rune::{El, col, icon, row, text};
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::Paint;
use super::composer;
use crate::assistant::setup::{self, Answer, Ask, Choice, Stage};
use crate::assistant::store::Turn;
use crate::assistant::worker::Recall;
use crate::desktop::i18n::tr;

/// Where a question is: its conversation, turn, and place in the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct At {
    conversation: u32,
    turn: usize,
    ask: usize,
}

/// The OS a question names: a scene plays a Mac.
pub(super) fn os(ui: &Ui) -> &'static str {
    if ui.assistant.fixture {
        "macos"
    } else {
        std::env::consts::OS
    }
}

/// "Set VibeHex up for me": the app's three questions in a new
/// conversation (or the empty one open), at once.
pub(super) fn begin(ui: &mut Ui) {
    if ui.assistant.answering.is_some() {
        return;
    }
    let asks = setup::questions(os(ui));
    let now = ui.assistant.now_ms();
    let pane = &mut ui.assistant;
    pane.consent = None;
    let id = match pane.conversation() {
        Some(conversation) if conversation.turns.is_empty() => conversation.id,
        _ => pane
            .store
            .start(tr("Set up for me"), pane.engine.take(), pane.effort.take()),
    };
    pane.store.push(
        id,
        Turn {
            asks,
            created_ms: now,
            ..Turn::default()
        },
    );
    pane.open = Some(id);
    pane.error = None;
}

/// A question's words as the person sees them: the app's own in the
/// window's language, the model's as written.
fn words<'a>(ask: &Ask, words: &'a str) -> &'a str {
    if ask.own { tr(words) } else { words }
}

/// An option's label as the person saw it.
pub(super) fn label(ask: &Ask, option: &Choice) -> String {
    words(ask, &option.label).to_owned()
}

/// The questions of turn `index`: the answered ones as their chips, then
/// the first still open, while nothing came after it.
pub(super) fn view(ui: &Ui, p: Paint, conversation: u32, index: usize, turn: &Turn) -> Vec<El> {
    let pane = &ui.assistant;
    let last = pane
        .store
        .get(conversation)
        .is_some_and(|open| open.turns.len() == index + 1)
        && pane.answering_in(Some(conversation)).is_none();
    let mut shown = Vec::new();
    for (number, ask) in turn.asks.iter().enumerate() {
        let at = At {
            conversation,
            turn: index,
            ask: number,
        };
        if ask.done {
            shown.push(past(p, number, ask));
            continue;
        }
        if last {
            shown.push(open(p, at, ask));
        }
        break;
    }
    shown
}

/// The question and its hint.
fn title(p: Paint, ask: &Ask, small: bool) -> El {
    let font = if small {
        rune::TextStyle {
            weight: 560.0,
            ..p.k.ty.ui
        }
    } else {
        p.k.ty.body_medium
    };
    let mut line = row().key("title").gap(8.0).center_y().flex_wrap().child(
        text(words(ask, &ask.q).to_owned())
            .key("q")
            .font(font)
            .color(p.text())
            .wrap(),
    );
    if !ask.hint.is_empty() {
        line = line.child(
            text(words(ask, &ask.hint).to_owned())
                .key("hint")
                .font(p.k.ty.small)
                .color(p.faint()),
        );
    }
    line
}

/// A question to answer: its chips, and Continue for several.
fn open(p: Paint, at: At, ask: &Ask) -> El {
    let c = &p.k.c;
    let mut chips = row()
        .key("chips")
        .role(if ask.multi { "group" } else { "radiogroup" })
        .gap(6.0)
        .flex_wrap();
    for (index, option) in ask.options.iter().enumerate() {
        let on = ask.chosen.contains(&option.id);
        let id = option.id.clone();
        chips = chips.child(
            chip(p, &format!("option{index}"), &label(ask, option), on, false)
                .role(if ask.multi { "checkbox" } else { "radio" })
                .checked(on)
                .focusable(true)
                .focus_ring(2.0, 0.0, c.border_focus)
                .hover(c.hover)
                .press(c.pressed)
                .on_press(move |ui: &mut Ui| pick(ui, at, &id)),
        );
    }
    let mut card = col()
        .key(format!("ask{}", at.ask))
        .role("group")
        .label(words(ask, &ask.q).to_owned())
        .gap(10.0)
        .px(14.0)
        .py(12.0)
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .border(p.hair, c.border_base)
        .child(title(p, ask, false))
        .child(chips);
    if ask.multi {
        let ready = !ask.chosen.is_empty();
        let button = rune_kit::Button::new(tr("Continue"))
            .variant(rune_kit::ButtonVariant::Neutral)
            .disabled(!ready)
            .on_press(move |ui: &mut Ui| answered(ui, at));
        card = card.child(
            row()
                .key("continue")
                .child(super::super::controls::PushSize::build_small_push(button, p.k).key("go")),
        );
    }
    card
}

/// A question answered: every option, the chosen ones tinted.
fn past(p: Paint, number: usize, ask: &Ask) -> El {
    let mut chips = row().key("chips").gap(6.0).flex_wrap();
    for (index, option) in ask.options.iter().enumerate() {
        let on = ask.chosen.contains(&option.id);
        chips = chips.child(
            chip(p, &format!("option{index}"), &label(ask, option), on, true)
                .role("note")
                .checked(on),
        );
    }
    col()
        .key(format!("asked{number}"))
        .role("group")
        .label(words(ask, &ask.q).to_owned())
        .gap(8.0)
        .px(14.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak)
        .child(title(p, ask, true))
        .child(chips)
}

/// One option: outlined, or on a soft accent tint with a check when
/// chosen. `small` for an answered question's.
fn chip(p: Paint, key: &str, label: &str, on: bool, small: bool) -> El {
    let c = &p.k.c;
    let font = if small {
        rune::TextStyle {
            size: 12.0,
            ..p.k.ty.ui
        }
    } else {
        rune::TextStyle {
            size: 12.5,
            ..p.k.ty.ui
        }
    };
    let ink = match (on, small) {
        (true, _) => p.accent(),
        (false, true) => p.faint(),
        (false, false) => p.text(),
    };
    let mut chip = row()
        .key(key.to_owned())
        .label(label.to_owned())
        .h(if small { 24.0 } else { 28.0 })
        .px(10.0)
        .gap(6.0)
        .center_y()
        .radius(p.k.radius.sm);
    chip = if on {
        chip.fill(p.k.alpha(p.accent(), 0.12))
            .border(p.hair, p.k.alpha(p.accent(), 0.45))
    } else if small {
        chip.border(p.hair, c.border_weak)
    } else {
        chip.fill(c.layer_02).border(p.hair, c.border_weak)
    };
    chip.children((on && !small).then(|| icon(&theme::CHECK, 12.0).key("check").color(ink)))
        .child(
            text(label.to_owned())
                .key("label")
                .font(font)
                .color(ink)
                .max_lines(1),
        )
}

fn ask_at(ui: &Ui, at: At) -> Option<Ask> {
    ui.assistant
        .store
        .get(at.conversation)?
        .turns
        .get(at.turn)?
        .asks
        .get(at.ask)
        .cloned()
}

/// A chip pressed: a single choice is answered, a multiple one toggles.
fn pick(ui: &mut Ui, at: At, id: &str) {
    let Some(mut ask) = ask_at(ui, at) else {
        return;
    };
    if ask.multi {
        match ask.chosen.iter().position(|chosen| chosen == id) {
            Some(index) => {
                ask.chosen.remove(index);
            }
            None => ask.chosen.push(id.to_owned()),
        }
        ui.assistant
            .store
            .set_ask(at.conversation, at.turn, at.ask, ask);
        return;
    }
    ask.chosen = vec![id.to_owned()];
    ui.assistant
        .store
        .set_ask(at.conversation, at.turn, at.ask, ask);
    answered(ui, at);
}

/// The question is answered; after the last, the answers go.
fn answered(ui: &mut Ui, at: At) {
    let Some(mut ask) = ask_at(ui, at) else {
        return;
    };
    if ask.chosen.is_empty() {
        return;
    }
    ask.done = true;
    ui.assistant
        .store
        .set_ask(at.conversation, at.turn, at.ask, ask);
    let all = ui
        .assistant
        .store
        .get(at.conversation)
        .and_then(|conversation| conversation.turns.get(at.turn))
        .is_some_and(|turn| turn.asks.iter().all(|ask| ask.done));
    if all {
        send(ui, at.conversation, at.turn);
    }
}

/// The answers to turn `turn`'s questions, to the model as one turn.
fn send(ui: &mut Ui, conversation: u32, turn: usize) {
    let Some(asks) = ui
        .assistant
        .store
        .get(conversation)
        .and_then(|open| open.turns.get(turn))
        .map(|turn| turn.asks.clone())
    else {
        return;
    };
    let answers = asks
        .iter()
        .map(|ask| Answer {
            id: ask.id.clone(),
            chosen: ask.chosen.clone(),
        })
        .collect();
    let line = setup::answers_line(&asks, label);
    plan(ui, conversation, answers, line);
}

/// Asks for the plan for `answers` in `conversation`.
fn plan(ui: &mut Ui, conversation: u32, answers: Vec<Answer>, line: String) {
    if ui.is_fixture() || ui.assistant.fixture || ui.assistant.answering.is_some() {
        return;
    }
    composer::ask_with(
        ui,
        conversation,
        line.clone(),
        Recall::Guide,
        Stage::Plan { answers, line },
    );
}

/// Asks an answers turn's plan again.
pub(super) fn again(ui: &mut Ui, conversation: u32, turn: &Turn) {
    plan(
        ui,
        conversation,
        turn.answers.clone(),
        turn.question.clone(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ui: &Ui, ask: usize) -> At {
        At {
            conversation: ui.assistant.open.unwrap(),
            turn: 0,
            ask,
        }
    }

    fn asks(ui: &Ui) -> Vec<Ask> {
        let open = ui.assistant.conversation().unwrap();
        open.turns[0].asks.clone()
    }

    /// The Start button's questions come at once, with no model; a
    /// multiple choice waits for Continue, a single one answers on a click.
    #[test]
    fn the_start_buttons_questions_answer_one_by_one() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        begin(&mut ui);
        assert!(ui.assistant.answering.is_none(), "no model was asked");
        let shown = asks(&ui);
        assert_eq!(shown, setup::questions("macos"));
        assert_eq!(ui.assistant.conversation().unwrap().name, "Set up for me");
        let here = at(&ui, 0);
        pick(&mut ui, here, "meetings");
        let here = at(&ui, 0);
        pick(&mut ui, here, "dictation_code");
        let here = at(&ui, 0);
        pick(&mut ui, here, "meetings");
        assert_eq!(asks(&ui)[0].chosen, ["dictation_code"]);
        assert!(!asks(&ui)[0].done, "several wait for Continue");
        let here = at(&ui, 0);
        answered(&mut ui, here);
        assert!(asks(&ui)[0].done);
        let here = at(&ui, 1);
        pick(&mut ui, here, "ru");
        assert!(asks(&ui)[1].done, "one answers on a click");
        assert_eq!(asks(&ui)[1].chosen, ["ru"]);
        // Continue with nothing picked does nothing.
        let mut empty = asks(&ui)[2].clone();
        empty.chosen.clear();
        let here = at(&ui, 2);
        answered(&mut ui, here);
        assert!(!asks(&ui)[2].done);
        assert!(empty.chosen.is_empty());
    }

    /// What the model reads of the answers: ids and the words the person
    /// saw, in the window's language for the app's own.
    #[test]
    fn the_answers_line_names_what_was_seen() {
        let mut shown = setup::questions("macos");
        shown[1].chosen = vec!["mixed".into()];
        let line = setup::answers_line(&shown, label);
        assert_eq!(line, "language: mixed (Both, mixed)");
    }
}
