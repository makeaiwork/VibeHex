//! Setup's questions as option chips (the Setup board): the Start screen's
//! "Set VibeHex up for me" draws the app's own at once, with no model call,
//! after a paragraph of what is set now; a typed request gets the
//! model's, in the message's language.
//!
//! A round's questions show together, every round-one chip preselected
//! from the settings, so the person only changes what's wrong and presses
//! Continue. The follow-up rounds after it (yes or no about the Labs
//! features that fit) also offer "Skip to the plan". Answered rounds stay
//! as their chosen chips, with no bubble.
//!
//! Which round or plan comes next is the setup machine's
//! (`assistant::setup::flow`). This file draws where the setup stands
//! (`flow::step`), sends it what the person pressed ([`send`]), and
//! performs what it asks for ([`perform`]): turns and answers into the
//! store, the setup's record kept, a model asked.

use rune::{El, TextEdit, col, el, icon, row, text};
use rune_kit::TextArea;
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::Paint;
use super::composer;
use crate::assistant::setup::flow::{self, Effect, Env, Event, Flow, Step, first};
use crate::assistant::setup::since::{self, Since};
use crate::assistant::setup::{self, Ask, Choice, about};
use crate::assistant::store::Turn;
use crate::assistant::worker::Recall;
use crate::desktop::i18n::{tr, tr_fill, tr_window};

/// Where a round is: its conversation and turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct At {
    pub(super) conversation: u32,
    pub(super) turn: usize,
}

/// The OS a question names: a scene plays a Mac.
pub(super) fn os(ui: &Ui) -> &'static str {
    if ui.assistant.fixture {
        "macos"
    } else {
        std::env::consts::OS
    }
}

/// "Set VibeHex up for me": what is set now, and round one's questions
/// preselected from it, in a new conversation (or the empty one open). A
/// returning setup says what came since and folds round one to a row.
pub(super) fn begin(ui: &mut Ui) {
    if ui.assistant.answering.is_some() {
        return;
    }
    let since = super::returning::since(ui);
    start(ui, since);
}

/// Opens setup in a conversation of its own: a first one, or a returning
/// one with what came `since`.
pub(super) fn start(ui: &mut Ui, since: Option<Since>) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let pane = &mut ui.assistant;
    pane.consent = None;
    let id = match pane.conversation() {
        Some(conversation) if conversation.turns.is_empty() => conversation.id,
        _ => pane
            .store
            .start(tr("Set up for me"), pane.engine.take(), pane.effort.take()),
    };
    pane.open = Some(id);
    pane.error = None;
    send(ui, id, Event::Begin { since });
}

/// Sends `event` to the setup in `conversation` (`setup::flow`) and does
/// what it asks for: the one way the pane moves a setup on.
pub(super) fn send(ui: &mut Ui, conversation: u32, event: Event) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let Some(open) = ui.assistant.store.get(conversation) else {
        return;
    };
    let kept = ui.assistant.store.setups().to_vec();
    let mut flow = Flow::new(conversation, open.turns.clone(), kept);
    let mut events = vec![event];
    while let Some(event) = events.pop() {
        let (next, effects) = on(ui, flow, event);
        flow = next;
        events.extend(perform(ui, conversation, effects));
    }
}

/// The machine's transition on `event`, with what it may know of this
/// install now.
pub(super) fn on(ui: &Ui, flow: Flow, event: Event) -> (Flow, Vec<Effect>) {
    knowing(ui, flow.conversation, |env| flow.on(event, env))
}

/// `then` with what the machine may know of this install now.
fn knowing<T>(ui: &Ui, conversation: u32, then: impl FnOnce(&Env) -> T) -> T {
    let context = super::changes::context(ui);
    let unreachable = unreachable(ui, conversation);
    let env = Env {
        context: &context,
        saved: &ui.settings,
        now_ms: ui.assistant.now_ms(),
        on_device: unreachable || on_device(ui, conversation),
        unreachable,
        busy: ui.assistant.answering.is_some(),
        scene: ui.assistant.fixture,
        changed: &since::changed_since,
    };
    then(&env)
}

/// Whether round `at`'s Continue and its "Skip to the plan" wait for a
/// model that is answering in another conversation, and why, in words: it
/// answers there, or its answer waits there on the consent card.
fn waits(ui: &Ui, at: At) -> (bool, bool, &'static str) {
    let pane = &ui.assistant;
    let open = pane.store.get(at.conversation);
    let why = super::composer::elsewhere(ui).map_or(tr(flow::BUSY), |(why, _)| why);
    let Some(open) = open.filter(|_| pane.answering.is_some()) else {
        return (false, false, why);
    };
    let flow = Flow::new(
        at.conversation,
        open.turns.clone(),
        pane.store.setups().to_vec(),
    );
    knowing(ui, at.conversation, |env| {
        (
            flow.waits(at.turn, false, env),
            flow.waits(at.turn, true, env),
            why,
        )
    })
}

/// Whether the open conversation shows a setup round whose Continue waits
/// for a model answering elsewhere: the round says why itself.
pub(super) fn round_waits(ui: &Ui) -> bool {
    let pane = &ui.assistant;
    let Some(open) = pane.conversation() else {
        return false;
    };
    let at = match flow::step(&open.turns, pane.store.setups()) {
        Step::Basics {
            turn,
            folded: false,
        }
        | Step::Follow { turn, .. } => At {
            conversation: open.id,
            turn,
        },
        _ => return false,
    };
    let waits = waits(ui, at);
    waits.0 || waits.1
}

/// Performs a transition's effects in `conversation`, in order: turns and
/// answers into the store, the setup's record kept, a model asked. What
/// couldn't be asked comes back as an event for the machine.
pub(super) fn perform(ui: &mut Ui, conversation: u32, effects: Vec<Effect>) -> Vec<Event> {
    let mut back = Vec::new();
    for effect in effects {
        let store = &mut ui.assistant.store;
        match effect {
            Effect::Push(turn) => store.push(conversation, *turn),
            Effect::Asks { turn, asks } => {
                for (number, ask) in asks.into_iter().enumerate() {
                    store.set_ask(conversation, turn, number, ask);
                }
            }
            Effect::Join { turn, asks } => store.push_asks(conversation, turn, asks),
            Effect::Unfold { turn } => store.unfold(conversation, turn),
            Effect::Keep(setup) => store.finish(setup),
            Effect::Ask { line, stage } => {
                let asked = stage.clone();
                if !composer::ask_with(ui, conversation, line, Recall::Guide, asked) {
                    back.push(Event::Unasked(stage));
                }
            }
        }
    }
    back
}

/// Whether this conversation asks a built-in model, which never writes
/// setup's questions or plan: the app does.
fn on_device(ui: &Ui, conversation: u32) -> bool {
    if ui.is_fixture() || ui.assistant.fixture {
        return true;
    }
    built_in(ui, conversation)
}

/// Whether this conversation's engine is a built-in model.
fn built_in(ui: &Ui, conversation: u32) -> bool {
    asks_on(ui, conversation).is_local()
}

/// Whether this conversation's model can't be reached now: OpenCode isn't
/// running, Codex isn't installed, or Codex is signed out as Output last
/// read it. Setup then writes itself, as on a built-in model, and no
/// request goes out to fail.
pub(super) fn unreachable(ui: &Ui, conversation: u32) -> bool {
    let engine = asks_on(ui, conversation);
    super::connect::unreachable(ui, &engine)
        || (engine.codex_model().is_some()
            && super::super::output::codex_signed_in(ui) == Some(false))
}

/// The engine `conversation` asks on.
fn asks_on(ui: &Ui, conversation: u32) -> crate::files::rewrite::Engine {
    let open = ui.assistant.store.get(conversation);
    super::engine(
        &ui.settings,
        open.and_then(|open| open.engine.as_deref()),
        open.and_then(|open| open.effort.as_deref()),
    )
}

/// Whether a model writes this setup's follow-ups and plan, and so reads
/// round one's own words: the conversation asks a remote engine (Codex,
/// OpenCode, OpenRouter, your own server) that can be reached. A built-in
/// model, or one that can't be reached (OpenCode not running, Codex signed
/// out: `connect::needed`), leaves them to the app's own rules, and the
/// field hides. It follows the engine as it is now: switched after round
/// one shows, the field shows or hides with it, and what was typed stays
/// with the round. A scene plays the engine it is set to, though the app
/// writes its rounds.
pub(super) fn model_writes(ui: &Ui, conversation: u32) -> bool {
    !built_in(ui, conversation) && !unreachable(ui, conversation) && !super::connect::needed(ui)
}

/// A chip pressed.
pub(super) fn pick(ui: &mut Ui, at: At, number: usize, id: &str) {
    let (turn, id) = (at.turn, id.to_owned());
    send(ui, at.conversation, Event::Pick { turn, number, id });
}

/// Continue, or "Skip to the plan" (`skip`).
pub(super) fn answered(ui: &mut Ui, at: At, skip: bool) {
    let turn = at.turn;
    send(ui, at.conversation, Event::Answered { turn, skip });
}

/// A card was applied, put off, or undone: a setup's plan finishes the
/// setup and its record follows it, the preset's leads on to its
/// shortcuts.
pub(super) fn decided(ui: &mut Ui, at: super::changes::At) {
    let (turn, card) = (at.turn, at.card);
    send(ui, at.conversation, Event::Decided { turn, card });
}

/// Asks turn `turn` again: a returning setup's look as itself, else the
/// answers' follow-ups or plan.
pub(super) fn again(ui: &mut Ui, conversation: u32, turn: usize) {
    let look = ui
        .assistant
        .store
        .get(conversation)
        .and_then(|open| open.turns.get(turn))
        .is_some_and(flow::is_look);
    let event = if look {
        Event::LookAgain { turn }
    } else {
        Event::Again { turn }
    };
    send(ui, conversation, event);
}

/// A question's words as the person sees them.
fn words<'a>(ask: &Ask, words: &'a str) -> &'a str {
    ask.words(words)
}

/// A question's hint: a follow-up's is its Labs feature's title, a place
/// in the window, so in the window's words.
fn hint(ask: &Ask) -> String {
    if ask.own && ask.id.starts_with("labs.") {
        tr_window(&ask.hint).to_owned()
    } else {
        words(ask, &ask.hint).to_owned()
    }
}

/// The turn's paragraph of what is set now, then its round: answered, as
/// the chips chosen; open, while nothing came after it, as a card.
pub(super) fn view(ui: &Ui, p: Paint, conversation: u32, index: usize, turn: &Turn) -> Vec<El> {
    let pane = &ui.assistant;
    let mut shown = Vec::new();
    if !turn.now.trim().is_empty() {
        shown.push(super::recall::answer(p, &turn.now, None, None, (false, false)).key("now"));
    }
    if !turn.since.trim().is_empty() {
        shown.push(super::recall::answer(p, &turn.since, None, None, (false, false)).key("since"));
    }
    if turn.asks.is_empty() {
        return shown;
    }
    let at = At {
        conversation,
        turn: index,
    };
    let unfolded = pane.before_open.contains(&at);
    if turn.asks.iter().all(|ask| ask.done) {
        shown.extend(past(p, at, &turn.asks, unfolded));
        return shown;
    }
    // A round to answer shows only where the setup stands, and not while
    // a model is answering here.
    let stands = pane
        .store
        .get(conversation)
        .filter(|_| pane.answering_in(Some(conversation)).is_none())
        .map(|open| flow::step(&open.turns, pane.store.setups()));
    match stands {
        Some(Step::Basics { turn: here, folded }) if here == index => {
            let field = about(ui, p, at, &turn.about, folded);
            shown.push(if folded {
                super::returning::folded(p, at, &turn.asks, field)
            } else {
                open(p, at, &turn.asks, unfolded, waits(ui, at), field)
            });
        }
        Some(Step::Follow { turn: here, .. }) if here == index => {
            shown.push(open(p, at, &turn.asks, unfolded, waits(ui, at), None));
        }
        _ => {}
    }
    shown
}

/// Round one's own words (`setup::about`): one optional field under its
/// questions, "what you do and where you dictate most", only while a model
/// writes the follow-ups ([`model_writes`]). Typing keeps the text in the
/// round's turn as a draft, at most `about::MAX` characters; Continue
/// sends and keeps it. `small` beside a returning setup's folded answers.
fn about(ui: &Ui, p: Paint, at: At, written: &str, small: bool) -> Option<El> {
    if !model_writes(ui, at.conversation) {
        return None;
    }
    let pane = &ui.assistant;
    let fresh;
    let edit = match pane.about_at == Some(at) && pane.about_edit.text == written {
        true => &pane.about_edit,
        false => {
            fresh = TextEdit::new(written);
            &fresh
        }
    };
    let label = tr(about::LABEL);
    let font = match small {
        true => rune::TextStyle {
            weight: 560.0,
            ..p.k.ty.ui
        },
        false => p.k.ty.body_medium,
    };
    let written = written.to_owned();
    let field = TextArea::new(edit)
        .label(label)
        .placeholder(tr(about::PLACEHOLDER))
        .auto_grow(2, 6)
        .width(None)
        .wrap_anywhere()
        .on_edit(move |ui: &mut Ui| {
            let pane = &mut ui.assistant;
            if pane.about_at != Some(at) || pane.about_edit.text != written {
                pane.about_at = Some(at);
                pane.about_edit = TextEdit::new(written.clone());
            }
            &mut pane.about_edit
        })
        .on_change(move |ui: &mut Ui, _: &mut rune::Tx| {
            let pane = &mut ui.assistant;
            let capped = about::cap(&pane.about_edit.text);
            if capped.len() < pane.about_edit.text.len() {
                let capped = capped.to_owned();
                pane.about_edit.replace_text(capped);
            }
            let text = pane.about_edit.text.clone();
            pane.store.write_about(at.conversation, at.turn, &text);
        })
        .build(p.k)
        .key("field")
        .w_full()
        .min_w(0.0);
    Some(
        col()
            .key("own-words")
            .role("group")
            .label(label)
            .gap(6.0)
            .w_full()
            .min_w(0.0)
            .child(
                text(label)
                    .key("q")
                    .font(font)
                    .color(p.text())
                    .wrap_anywhere()
                    .w_full(),
            )
            .child(field),
    )
}

/// Unfolds round `at`'s "Answered before", for a scene.
pub(super) fn unfold_before(ui: &mut Ui, at: At) {
    ui.assistant.before_open.push(at);
}

/// "Answered before: 4 ›": the follow-ups an earlier setup answered,
/// not asked again; unfolded, each with its answer, changeable while the
/// round is `open`.
fn before(p: Paint, at: At, asks: &[Ask], unfolded: bool, open: bool) -> Option<El> {
    let earlier: Vec<(usize, &Ask)> = asks
        .iter()
        .enumerate()
        .filter(|(_, ask)| ask.before)
        .collect();
    if earlier.is_empty() {
        return None;
    }
    let c = &p.k.c;
    let words = tr_fill("Answered before: {}", &earlier.len().to_string());
    let toggle = row()
        .key("before-toggle")
        .role("button")
        .label(words.clone())
        .checked(unfolded)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .gap(6.0)
        .h(24.0)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .child(text(words).key("words").font(p.k.ty.small).color(p.muted()))
        .child(
            icon(
                if unfolded {
                    &theme::CHEVRON_DOWN
                } else {
                    &theme::CHEVRON_RIGHT
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_faint),
        )
        .on_press(move |ui: &mut Ui| {
            let shown = &mut ui.assistant.before_open;
            match shown.iter().position(|known| *known == at) {
                Some(index) => {
                    shown.remove(index);
                }
                None => shown.push(at),
            }
        });
    let mut fold = col()
        .key("before")
        .gap(10.0)
        .child(row().key("head").child(toggle));
    if unfolded {
        for (number, ask) in earlier {
            fold = fold.child(if open {
                question(p, at, number, ask)
            } else {
                answered_line(p, number, ask)
            });
        }
    }
    Some(fold)
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
    // The question fills the card's width and wraps; its hint always sits
    // on its own line under it, whatever the question's length.
    let mut line = col().key("title").w_full().min_w(0.0).gap(2.0).child(
        text(words(ask, &ask.q).to_owned())
            .key("q")
            .font(font)
            .color(p.text())
            .wrap_anywhere()
            .w_full(),
    );
    if !ask.hint.is_empty() && !small {
        line = line.child(
            text(hint(ask))
                .key("hint")
                .font(p.k.ty.small)
                .color(p.faint())
                .wrap_anywhere()
                .w_full(),
        );
    }
    line
}

/// A round to answer: each question that shows with its chips, then
/// Continue, and "Skip to the plan" after round one. Either `waits` (Continue,
/// the skip) while the model it would ask is answering in another
/// conversation, and the round says so.
fn open(
    p: Paint,
    at: At,
    asks: &[Ask],
    unfolded: bool,
    waits: (bool, bool, &str),
    field: Option<El>,
) -> El {
    let c = &p.k.c;
    let mut card = col()
        .key(format!("round{}", at.turn))
        .role("group")
        .label(tr("Setup questions"))
        .gap(10.0)
        .px(14.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .fill(c.layer_01)
        .border(p.hair, c.border_base);
    for (number, ask) in asks.iter().enumerate() {
        if !setup::shows(asks, ask) || ask.before {
            continue;
        }
        card = card.child(question(p, at, number, ask));
    }
    card = card.children(field);
    card = card.children(before(p, at, asks, unfolded, true));
    let small = super::super::controls::PushSize::build_small_push;
    let go = rune_kit::Button::new(tr("Continue"))
        .variant(rune_kit::ButtonVariant::Neutral)
        .disabled(waits.0)
        .on_press(move |ui: &mut Ui| answered(ui, at, false));
    let mut foot = row().key("continue").gap(8.0).center_y();
    if !first(asks) {
        let skip = rune_kit::Button::new(tr("Skip to the plan"))
            .variant(rune_kit::ButtonVariant::Ghost)
            .disabled(waits.1)
            .on_press(move |ui: &mut Ui| answered(ui, at, true));
        foot = foot.child(small(skip, p.k).key("skip"));
    }
    if waits.0 || waits.1 {
        let why = text(waits.2.to_owned()).key("waits").font(p.k.ty.small);
        card = card.child(why.color(p.muted()).wrap_anywhere().w_full());
    }
    card.child(
        foot.child(el().key("space").grow(1.0))
            .child(small(go, p.k).key("go")),
    )
}

/// One question: its title and chips. Options with captions go three to
/// a line, at one width.
fn question(p: Paint, at: At, number: usize, ask: &Ask) -> El {
    let c = &p.k.c;
    let captions = ask.options.iter().any(|option| !option.note.is_empty());
    let chip_of = |index: usize, option: &Choice| {
        let on = ask.chosen.contains(&option.id);
        let id = option.id.clone();
        chip(p, &format!("option{index}"), ask, option, on, false)
            .role(if ask.multi { "checkbox" } else { "radio" })
            .checked(on)
            .focusable(true)
            .focus_ring(2.0, 0.0, c.border_focus)
            .hover(c.hover)
            .press(c.pressed)
            .on_press(move |ui: &mut Ui| pick(ui, at, number, &id))
    };
    let chips = if captions {
        let mut grid = col().key("chips").gap(6.0);
        for (line, three) in ask.options.chunks(3).enumerate() {
            let mut cells = row().key(format!("line{line}")).gap(6.0);
            for (offset, option) in three.iter().enumerate() {
                cells = cells.child(chip_of(line * 3 + offset, option).w(0.0).grow(1.0));
            }
            for pad in three.len()..3 {
                cells = cells.child(el().key(format!("pad{pad}")).w(0.0).grow(1.0));
            }
            grid = grid.child(cells);
        }
        grid.role(if ask.multi { "group" } else { "radiogroup" })
    } else {
        let mut chips = row()
            .key("chips")
            .role(if ask.multi { "group" } else { "radiogroup" })
            .gap(6.0)
            .flex_wrap();
        for (index, option) in ask.options.iter().enumerate() {
            chips = chips.child(chip_of(index, option));
        }
        chips
    };
    col()
        .key(format!("ask{number}"))
        .role("group")
        .label(words(ask, &ask.q).to_owned())
        .gap(6.0)
        .child(title(p, ask, false))
        .child(chips)
}

/// A round answered: each question with an answer, and its chosen chips;
/// what was answered before, folded.
fn past(p: Paint, at: At, asks: &[Ask], unfolded: bool) -> Option<El> {
    let mut lines = col()
        .key(format!("asked{}", at.turn))
        .role("group")
        .label(tr("Setup questions"))
        .gap(8.0)
        .px(14.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak);
    let mut any = false;
    for (number, ask) in asks.iter().enumerate() {
        if ask.chosen.is_empty() || ask.before {
            continue;
        }
        any = true;
        lines = lines.child(answered_line(p, number, ask));
    }
    let earlier = before(p, at, asks, unfolded, false);
    any |= earlier.is_some();
    any.then(|| lines.children(earlier))
}

/// An answered question: its title and the chips chosen.
fn answered_line(p: Paint, number: usize, ask: &Ask) -> El {
    let mut chips = row().key("chips").gap(6.0).flex_wrap();
    for (option_index, option) in ask
        .options
        .iter()
        .enumerate()
        .filter(|(_, option)| ask.chosen.contains(&option.id))
    {
        chips = chips.child(
            chip(p, &format!("option{option_index}"), ask, option, true, true)
                .role("note")
                .checked(true),
        );
    }
    col()
        .key(format!("q{number}"))
        .gap(6.0)
        .child(title(p, ask, true))
        .child(chips)
}

/// One option: outlined, or on a soft accent tint with a check when
/// chosen, its caption under its label. `small` for an answered round's.
pub(super) fn chip(p: Paint, key: &str, ask: &Ask, option: &Choice, on: bool, small: bool) -> El {
    let c = &p.k.c;
    let label = ask.label(option);
    let font = rune::TextStyle {
        size: if small { 12.0 } else { 12.5 },
        ..p.k.ty.ui
    };
    let ink = match (on, small) {
        (true, _) => p.accent(),
        (false, true) => p.faint(),
        (false, false) => p.text(),
    };
    let note = words(ask, &option.note);
    let captioned = !small && !note.is_empty();
    let mut chip = row()
        .key(key.to_owned())
        .label(label.clone())
        .px(10.0)
        .gap(6.0)
        .radius(p.k.radius.sm);
    chip = if captioned {
        chip.py(5.0).items_start()
    } else {
        chip.h(if small { 24.0 } else { 28.0 }).center_y()
    };
    chip = if on {
        chip.fill(p.k.alpha(p.accent(), 0.12))
            .border(p.hair, p.k.alpha(p.accent(), 0.45))
    } else if small {
        chip.border(p.hair, c.border_weak)
    } else {
        chip.fill(c.layer_02).border(p.hair, c.border_weak)
    };
    let mut words = col()
        .key("words")
        .gap(1.0)
        .shrink()
        .min_w(0.0)
        // Beside a caption the chip is as tall as its words: a long label
        // takes a second line rather than losing its end.
        .child(if captioned {
            text(label)
                .key("label")
                .font(font)
                .color(ink)
                .wrap_anywhere()
                .max_lines(2)
        } else {
            text(label).key("label").font(font).color(ink).max_lines(1)
        });
    if captioned {
        words = words.child(
            text(note.to_owned())
                .key("note")
                .font(p.k.ty.small)
                .color(if on { p.muted() } else { p.faint() })
                .wrap_anywhere(),
        );
    }
    chip.children((on && !small).then(|| {
        row()
            .key("mark")
            .h(16.0)
            .center_y()
            .child(icon(&theme::CHECK, 12.0).key("check").color(ink))
    }))
    .child(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::setup::follow;

    fn at(ui: &Ui, turn: usize) -> At {
        At {
            conversation: ui.assistant.open.unwrap(),
            turn,
        }
    }

    fn turns(ui: &Ui) -> Vec<Turn> {
        ui.assistant.conversation().unwrap().turns.clone()
    }

    fn number(asks: &[Ask], id: &str) -> usize {
        asks.iter().position(|ask| ask.id == id).unwrap()
    }

    /// The Start button's round comes at once, with no model, after what
    /// is set now; its chips are preselected, and Continue goes on to the
    /// follow-ups the app writes itself, then its plan.
    #[test]
    fn the_start_buttons_rounds_lead_to_the_apps_plan() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        begin(&mut ui);
        assert!(ui.assistant.answering.is_none(), "no model was asked");
        assert_eq!(ui.assistant.conversation().unwrap().name, "Set up for me");
        let first = &turns(&ui)[0];
        assert!(first.now.contains("Now:"), "{}", first.now);
        let ids: Vec<&str> = first.asks.iter().map(|ask| ask.id.as_str()).collect();
        assert_eq!(ids, ["use", "language", "cleanup", "notes"]);
        assert!(first.asks.iter().take(3).all(|ask| !ask.chosen.is_empty()));
        // Code and calls: an untouched language follows, and the notes
        // question shows.
        let here = at(&ui, 0);
        let use_at = number(&first.asks, "use");
        for id in ["dictation_code", "meetings"] {
            if !turns(&ui)[0].asks[use_at].chosen.iter().any(|on| on == id) {
                pick(&mut ui, here, use_at, id);
            }
        }
        let asks = turns(&ui)[0].asks.clone();
        assert!(setup::shows(&asks, &asks[number(&asks, "notes")]));
        let language_at = number(&asks, "language");
        pick(&mut ui, here, language_at, "ru_tech");
        answered(&mut ui, here, false);
        assert!(turns(&ui)[0].asks.iter().all(|ask| ask.done));
        // Follow-up rounds of the app's own, at most four questions each,
        // then the plan with Skip.
        let mut guard = 0;
        while turns(&ui).last().unwrap().cards.is_empty() {
            let last = turns(&ui).len() - 1;
            let round = turns(&ui)[last].asks.clone();
            assert!(!round.is_empty() && round.len() <= follow::ROUND);
            assert!(round.iter().all(|ask| ask.own && ask.chosen.is_empty()));
            let here = at(&ui, last);
            pick(&mut ui, here, 0, "yes");
            answered(&mut ui, here, guard == 1);
            guard += 1;
            assert!(guard <= follow::ROUNDS + 1);
        }
        let plan = turns(&ui).last().unwrap().clone();
        assert!(!plan.answers.is_empty() && plan.question.is_empty());
        let card = &plan.cards[0];
        let yes = plan
            .answers
            .iter()
            .flat_map(|answer| &answer.changes)
            .find(|(key, _)| key.starts_with("labs."))
            .map(|(key, _)| key.clone());
        if let Some(key) = yes {
            let line = card.items.iter().find(|item| item.key == key).unwrap();
            assert!(line.included, "a yes starts checked");
        }
    }

    /// The setup's record follows its plan's card in the window too: a
    /// save that failed decides nothing, the one that goes through is
    /// recorded, and Undo takes what it applied out of the record.
    #[test]
    fn the_kept_setup_follows_its_plans_card() {
        use super::super::changes as act;
        use crate::assistant::changes::Decision;
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        begin(&mut ui);
        let here = at(&ui, 0);
        let language_at = number(&turns(&ui)[0].asks, "language");
        pick(&mut ui, here, language_at, "ru_tech");
        answered(&mut ui, here, false);
        while turns(&ui).last().unwrap().cards.is_empty() {
            let here = at(&ui, turns(&ui).len() - 1);
            pick(&mut ui, here, 0, "yes");
            answered(&mut ui, here, true);
        }
        let plan = act::At {
            conversation: here.conversation,
            turn: turns(&ui).len() - 1,
            card: 0,
        };
        let kept = |ui: &Ui| {
            let setup = ui.assistant.store.setups().last().unwrap();
            (setup.plan_ms != 0, setup.applied.len())
        };
        let ticked: Vec<bool> = act::card(&ui, plan)
            .unwrap()
            .items
            .iter()
            .map(|item| item.included)
            .collect();
        // Nothing ticked: the save is refused.
        act::set(&mut ui, plan, |card| {
            card.items.iter_mut().for_each(|item| item.included = false)
        });
        act::apply(&mut ui, plan);
        assert!(matches!(
            act::card(&ui, plan).unwrap().decision,
            Decision::Failed { .. }
        ));
        assert_eq!(kept(&ui), (false, 0), "a failed save decides nothing");
        act::set(&mut ui, plan, |card| {
            for (item, on) in card.items.iter_mut().zip(&ticked) {
                item.included = *on;
            }
        });
        act::apply(&mut ui, plan);
        assert!(matches!(
            act::card(&ui, plan).unwrap().decision,
            Decision::Applied { .. }
        ));
        let (decided, applied) = kept(&ui);
        assert!(
            decided && applied > 0,
            "the save that went through is recorded"
        );
        act::undo(&mut ui, plan);
        assert_eq!(act::card(&ui, plan).unwrap().decision, Decision::Undone);
        assert_eq!(kept(&ui), (true, 0), "undone, it no longer says applied");
        assert_eq!(ui.assistant.store.setups().len(), 1);
    }

    /// What the model reads of the answers: ids and the words the person
    /// saw, in the window's language for the app's own.
    #[test]
    fn the_answers_line_names_what_was_seen() {
        let mut shown = setup::questions("macos");
        shown.truncate(2);
        shown[1].chosen = vec!["mixed".into()];
        let line = setup::answers_line(&shown, Ask::label);
        assert_eq!(line, "language: mixed (Other or mixed)");
        let mut server = setup::questions("macos").remove(2);
        server.chosen = vec!["custom".into()];
        server.options[4].extra = "LM Studio".into();
        assert_eq!(
            setup::answers_line(&[server], Ask::label),
            "cleanup: custom (My own server (LM Studio))"
        );
    }

    /// Round one answered as preselected, with code and calls.
    fn round_one(ui: &mut Ui) {
        let index = turns(ui).iter().position(|turn| first(&turn.asks)).unwrap();
        let here = at(ui, index);
        let asks = turns(ui)[index].asks.clone();
        let use_at = number(&asks, "use");
        for id in ["dictation_code", "meetings"] {
            if !turns(ui)[index].asks[use_at]
                .chosen
                .iter()
                .any(|on| on == id)
            {
                pick(ui, here, use_at, id);
            }
        }
        answered(ui, here, false);
    }

    /// Round one's own words show only where a model reads them: on a
    /// remote engine that can be reached, never on a built-in model, nor on
    /// Codex signed out or OpenCode not running.
    #[test]
    fn the_own_words_field_shows_only_where_a_model_reads_it() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        begin(&mut ui);
        let here = at(&ui, 0);
        let p = Paint::new(false, 1.0);
        let shows = |ui: &Ui| about(ui, p, here, "", false).is_some();
        let choose = |ui: &mut Ui, engine: &str| {
            let engine = Some(engine.to_owned());
            ui.assistant.store.choose(here.conversation, engine, None);
        };
        choose(&mut ui, "codex");
        assert!(model_writes(&ui, here.conversation));
        assert!(shows(&ui), "Codex reads it");
        choose(&mut ui, "local:gemma-4-e4b-it");
        assert!(!shows(&ui), "a built-in model reads nothing");
        choose(&mut ui, "codex");
        ui.assistant.reach.codex = Some(false);
        assert!(!shows(&ui), "Codex signed out reads nothing");
        ui.assistant.reach.codex = None;
        choose(&mut ui, "opencode");
        ui.assistant.reach.opencode = Some(false);
        assert!(!shows(&ui), "OpenCode not running reads nothing");
        ui.assistant.reach.opencode = Some(true);
        assert!(shows(&ui));
    }

    /// What is typed in the field is round one's, capped; Continue keeps it
    /// with the setup, and the next setup shows it again, folded with the
    /// answers, to change.
    #[test]
    fn the_own_words_are_kept_and_shown_again() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        ui.meetings.meetings.clear();
        ui.history.entries.clear();
        begin(&mut ui);
        let here = at(&ui, 0);
        let typed = format!("  Пишу код, диктую в Cursor {}", "и ".repeat(300));
        ui.assistant
            .store
            .write_about(here.conversation, here.turn, &typed);
        assert_eq!(turns(&ui)[0].about.chars().count(), about::MAX);
        answered(&mut ui, here, false);
        let kept = ui.assistant.store.setups().last().unwrap().about.clone();
        assert!(kept.starts_with("Пишу код, диктую в Cursor"), "{kept}");
        // What the field held, trimmed.
        assert_eq!(kept, about::clean(about::cap(&typed)));
        // The next setup returns, folded, with the words as they were.
        ui.assistant.open = None;
        begin(&mut ui);
        let opening = turns(&ui)[0].clone();
        assert!(opening.folded);
        assert_eq!(opening.about, kept);
        assert!(opening.facts.contains(about::THEN), "{}", opening.facts);
        // "Change answers" keeps them.
        let here = at(&ui, 0);
        send(&mut ui, here.conversation, Event::Change { turn: 0 });
        assert!(!turns(&ui)[0].folded);
        assert_eq!(turns(&ui)[0].about, kept);
    }

    /// A follow-up round answered and the conversation left: the answers
    /// are kept at once, and the next setup doesn't ask them again but
    /// shows them folded, as answered, changeable; Continue keeps them.
    #[test]
    fn answered_follow_ups_are_kept_and_not_asked_again() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        begin(&mut ui);
        round_one(&mut ui);
        let last = turns(&ui).len() - 1;
        let round = turns(&ui)[last].asks.clone();
        assert!(round.len() >= 2, "{round:?}");
        let here = at(&ui, last);
        pick(&mut ui, here, 0, "no");
        pick(&mut ui, here, 1, "yes");
        answered(&mut ui, here, false);
        let kept = ui.assistant.store.setups().to_vec();
        assert_eq!(kept.len(), 1, "one record, updated round by round");
        let ids: Vec<&str> = kept[0]
            .asked
            .iter()
            .map(|answered| answered.ask.id.as_str())
            .collect();
        assert_eq!(ids, [round[0].id.as_str(), round[1].id.as_str()]);
        // Left half-way: a new setup.
        ui.assistant.open = None;
        begin(&mut ui);
        round_one(&mut ui);
        let next = turns(&ui).last().unwrap().clone();
        let asked_again: Vec<&Ask> = next.asks.iter().filter(|ask| !ask.before).collect();
        assert!(
            asked_again
                .iter()
                .all(|ask| ask.id != round[0].id && ask.id != round[1].id),
            "{asked_again:?}"
        );
        let before: Vec<(&str, &[String])> = next
            .asks
            .iter()
            .filter(|ask| ask.before)
            .map(|ask| (ask.id.as_str(), ask.chosen.as_slice()))
            .collect();
        assert!(before.contains(&(round[0].id.as_str(), &["no".to_owned()][..])));
        assert!(before.contains(&(round[1].id.as_str(), &["yes".to_owned()][..])));
        // Skipped to the plan, the earlier Yes still stands.
        let here = at(&ui, turns(&ui).len() - 1);
        answered(&mut ui, here, true);
        let plan = turns(&ui).last().unwrap().clone();
        assert!(
            plan.answers
                .iter()
                .any(|answer| answer.id == round[1].id && answer.chosen == ["yes"])
        );
    }

    /// Setup from the Start row to its plan, every round as preselected
    /// (calls picked so a round one is never empty), follow-ups skipped.
    fn to_the_plan(ui: &mut Ui) -> Turn {
        begin(ui);
        for _ in 0..8 {
            let open = ui.assistant.conversation().unwrap();
            match flow::step(&open.turns, ui.assistant.store.setups()) {
                Step::Basics { turn, .. } => {
                    let asks = turns(ui)[turn].asks.clone();
                    let use_at = number(&asks, "use");
                    if asks[use_at].chosen.is_empty() {
                        pick(ui, at(ui, turn), use_at, "dictation_any");
                    }
                    answered(ui, at(ui, turn), false);
                }
                Step::Follow { turn, .. } => answered(ui, at(ui, turn), true),
                Step::Plan { .. } | Step::Done => break,
                other => panic!("setup stopped at {other:?}"),
            }
        }
        assert!(ui.assistant.answering.is_none(), "no model was asked");
        let turns = turns(ui);
        assert!(
            turns.iter().all(|turn| turn.error.is_none()),
            "nothing failed: {turns:?}"
        );
        turns.last().unwrap().clone()
    }

    const CONNECT: &str = "To ask Assistant questions, connect a model below: Codex works best if you have ChatGPT Plus or Pro.";

    /// A fresh install's Assistant runs on AI cleanup's OpenCode, which
    /// isn't running: setup needs no model. It runs on the app's own rules
    /// to its plan, which ends pointing at the Connect panel.
    #[test]
    fn with_opencode_unreachable_setup_runs_to_the_apps_plan() {
        let mut ui = Ui::with(
            crate::app_settings::AppSettings::default(),
            super::super::super::Service::Fixture(Vec::new()),
        );
        super::super::fixture::connect(&mut ui);
        assert!(super::super::connect::needed(&ui));
        let plan = crate::desktop::i18n::speaking(Some("en"), || to_the_plan(&mut ui));
        let id = ui.assistant.open.unwrap();
        assert!(unreachable(&ui, id));
        assert!(plan.answer.ends_with(CONNECT), "{}", plan.answer);
        assert!(!plan.cards.is_empty(), "the app's plan");
    }

    /// Codex chosen for Assistant and signed out, as Output read it: the
    /// field still shows, and setup runs on the app's own rules the same.
    #[test]
    fn with_codex_signed_out_setup_runs_to_the_apps_plan() {
        let mut ui = Ui::with(
            crate::app_settings::AppSettings::default(),
            super::super::super::Service::Fixture(Vec::new()),
        );
        super::super::fixture::open(&mut ui);
        ui.assistant.reach.codex = Some(true);
        super::super::super::output::codex_signed_out(&mut ui);
        assert!(!super::super::connect::needed(&ui), "Codex is installed");
        let plan = crate::desktop::i18n::speaking(Some("en"), || to_the_plan(&mut ui));
        assert!(unreachable(&ui, ui.assistant.open.unwrap()));
        assert!(plan.answer.ends_with(CONNECT), "{}", plan.answer);
        // Signed in, a model writes the rounds: nothing points anywhere.
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        let plan = crate::desktop::i18n::speaking(Some("en"), || to_the_plan(&mut ui));
        assert!(!unreachable(&ui, ui.assistant.open.unwrap()));
        assert!(!plan.answer.contains("connect a model"), "{}", plan.answer);
    }
}
