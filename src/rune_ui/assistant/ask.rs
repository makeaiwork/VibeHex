//! Setup's questions as option chips (the Setup board): the Start screen's
//! "Set VibeHex up for me" draws the app's own at once, with no model call,
//! after a paragraph of what is set now; a typed request gets the
//! model's, in the message's language.
//!
//! A round's questions show together, every round-one chip preselected
//! from the settings, so the person only changes what's wrong and presses
//! Continue. The follow-up rounds after it (yes or no about the Labs
//! features that fit) also offer "Skip to the plan". Answered rounds stay
//! as their chosen chips, with no bubble; once the last is answered the
//! answers go as one turn (`Stage::Plan`), never as typed text. On a
//! built-in model, or in a scene, the app writes the follow-ups and the
//! plan itself; a remote model rewords the follow-ups first
//! (`Stage::Follow`) and adds to the plan.

use rune::{El, col, el, icon, row, text};
use rune_theme_opencode::icons as theme;

use super::super::Ui;
use super::super::controls::Paint;
use super::composer;
use crate::assistant::setup::{self, Answer, Ask, Choice, Stage, follow};
use crate::assistant::store::Turn;
use crate::assistant::worker::Recall;
use crate::desktop::i18n::tr;

/// Where a round is: its conversation and turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct At {
    conversation: u32,
    turn: usize,
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
/// preselected from it, in a new conversation (or the empty one open).
pub(super) fn begin(ui: &mut Ui) {
    if ui.assistant.answering.is_some() {
        return;
    }
    let context = super::changes::context(ui);
    let mut asks = setup::questions(os(ui));
    setup::preselect(&mut asks, &context.settings, &context.state);
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
            now: setup::now::now(&context.settings),
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

/// An option's label as the person saw it, with the server it names.
pub(super) fn label(ask: &Ask, option: &Choice) -> String {
    let label = words(ask, &option.label);
    if option.extra.is_empty() {
        label.to_owned()
    } else {
        format!("{label} ({})", option.extra)
    }
}

/// The turn's paragraph of what is set now, then its round: answered, as
/// the chips chosen; open, while nothing came after it, as a card.
pub(super) fn view(ui: &Ui, p: Paint, conversation: u32, index: usize, turn: &Turn) -> Vec<El> {
    let pane = &ui.assistant;
    let mut shown = Vec::new();
    if !turn.now.trim().is_empty() {
        shown.push(super::recall::answer(p, &turn.now, None, false, false).key("now"));
    }
    if turn.asks.is_empty() {
        return shown;
    }
    let last = pane
        .store
        .get(conversation)
        .is_some_and(|open| open.turns.len() == index + 1)
        && pane.answering_in(Some(conversation)).is_none();
    let at = At {
        conversation,
        turn: index,
    };
    if turn.asks.iter().all(|ask| ask.done) {
        shown.extend(past(p, index, &turn.asks));
    } else if last {
        shown.push(open(p, at, &turn.asks));
    }
    shown
}

/// Whether this round is round one, with its fixed questions.
fn first(asks: &[Ask]) -> bool {
    asks.iter().any(|ask| ask.id == "use")
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
    if !ask.hint.is_empty() && !small {
        line = line.child(
            text(words(ask, &ask.hint).to_owned())
                .key("hint")
                .font(p.k.ty.small)
                .color(p.faint()),
        );
    }
    line
}

/// A round to answer: each question that shows with its chips, then
/// Continue, and "Skip to the plan" after round one.
fn open(p: Paint, at: At, asks: &[Ask]) -> El {
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
        if !setup::shows(asks, ask) {
            continue;
        }
        card = card.child(question(p, at, number, ask));
    }
    let go = rune_kit::Button::new(tr("Continue"))
        .variant(rune_kit::ButtonVariant::Neutral)
        .on_press(move |ui: &mut Ui| answered(ui, at, false));
    let mut foot = row().key("continue").gap(8.0).center_y();
    if !first(asks) {
        foot = foot.child(super::card::push(
            p,
            "skip",
            tr("Skip to the plan"),
            super::card::Look::Ghost,
            move |ui, _| answered(ui, at, true),
        ));
    }
    card.child(
        foot.child(el().key("space").grow(1.0))
            .child(super::super::controls::PushSize::build_small_push(go, p.k).key("go")),
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

/// A round answered: each question with an answer, and its chosen chips.
fn past(p: Paint, index: usize, asks: &[Ask]) -> Option<El> {
    let mut lines = col()
        .key(format!("asked{index}"))
        .role("group")
        .label(tr("Setup questions"))
        .gap(8.0)
        .px(14.0)
        .py(10.0)
        .radius(p.k.radius.md)
        .border(p.hair, p.k.c.border_weak);
    let mut any = false;
    for (number, ask) in asks.iter().enumerate() {
        if ask.chosen.is_empty() {
            continue;
        }
        any = true;
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
        lines = lines.child(
            col()
                .key(format!("q{number}"))
                .gap(6.0)
                .child(title(p, ask, true))
                .child(chips),
        );
    }
    any.then_some(lines)
}

/// One option: outlined, or on a soft accent tint with a check when
/// chosen, its caption under its label. `small` for an answered round's.
fn chip(p: Paint, key: &str, ask: &Ask, option: &Choice, on: bool, small: bool) -> El {
    let c = &p.k.c;
    let label = label(ask, option);
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
        .child(text(label).key("label").font(font).color(ink).max_lines(1));
    if captioned {
        words = words.child(
            text(note.to_owned())
                .key("note")
                .font(p.k.ty.small)
                .color(if on { p.muted() } else { p.faint() })
                .wrap(),
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

fn round(ui: &Ui, at: At) -> Option<Vec<Ask>> {
    Some(
        ui.assistant
            .store
            .get(at.conversation)?
            .turns
            .get(at.turn)?
            .asks
            .clone(),
    )
}

fn set_round(ui: &mut Ui, at: At, asks: Vec<Ask>) {
    for (number, ask) in asks.into_iter().enumerate() {
        ui.assistant
            .store
            .set_ask(at.conversation, at.turn, number, ask);
    }
}

/// A chip pressed: a single choice takes it, a multiple one toggles. What
/// the person picked is theirs; an untouched question follows it (code
/// with Russian is Russian with English tech terms).
fn pick(ui: &mut Ui, at: At, number: usize, id: &str) {
    let Some(mut asks) = round(ui, at) else {
        return;
    };
    let follow_up = !first(&asks);
    let Some(ask) = asks.get_mut(number) else {
        return;
    };
    ask.touched = true;
    if ask.multi {
        match ask.chosen.iter().position(|chosen| chosen == id) {
            Some(index) => {
                ask.chosen.remove(index);
            }
            None => ask.chosen.push(id.to_owned()),
        }
    } else if ask.chosen == [id] && follow_up {
        // A follow-up pressed again is unanswered again.
        ask.chosen.clear();
    } else {
        ask.chosen = vec![id.to_owned()];
    }
    if asks[number].id == "use" {
        let context = super::changes::context(ui);
        setup::preselect(&mut asks, &context.settings, &context.state);
    }
    set_round(ui, at, asks);
}

/// The round is answered (or skipped, `skip`): the next round, or the
/// plan.
fn answered(ui: &mut Ui, at: At, skip: bool) {
    let Some(mut asks) = round(ui, at) else {
        return;
    };
    if first(&asks)
        && asks
            .iter()
            .any(|ask| ask.id == "use" && ask.chosen.is_empty())
    {
        return;
    }
    let shown: Vec<bool> = asks.iter().map(|ask| setup::shows(&asks, ask)).collect();
    for (ask, shows) in asks.iter_mut().zip(shown) {
        ask.done = true;
        if !shows || (skip && !ask.touched) {
            ask.chosen.clear();
        }
    }
    set_round(ui, at, asks);
    if skip {
        plan(ui, at.conversation);
    } else {
        next(ui, at);
    }
}

/// The turns of the open setup, from its round one.
fn setup_turns(turns: &[Turn]) -> &[Turn] {
    let from = turns
        .iter()
        .rposition(|turn| first(&turn.asks))
        .unwrap_or(0);
    &turns[from..]
}

/// Every question the setup asked, in order.
fn asked(ui: &Ui, conversation: u32) -> Vec<Ask> {
    ui.assistant
        .store
        .get(conversation)
        .map(|open| {
            setup_turns(&open.turns)
                .iter()
                .flat_map(|turn| turn.asks.iter().cloned())
                .collect()
        })
        .unwrap_or_default()
}

/// Whether this conversation asks a built-in model, which never writes
/// setup's questions or plan: the app does.
fn on_device(ui: &Ui, conversation: u32) -> bool {
    if ui.is_fixture() || ui.assistant.fixture {
        return true;
    }
    let open = ui.assistant.store.get(conversation);
    super::engine(
        &ui.settings,
        open.and_then(|open| open.engine.as_deref()),
        open.and_then(|open| open.effort.as_deref()),
    )
    .is_local()
}

/// After round `at`: the next follow-up round, else the plan.
fn next(ui: &mut Ui, at: At) {
    let Some(open) = ui.assistant.store.get(at.conversation) else {
        return;
    };
    let Some(turn) = open.turns.get(at.turn) else {
        return;
    };
    let rounds = setup_turns(&open.turns)
        .iter()
        .filter(|turn| !turn.asks.is_empty() && !first(&turn.asks))
        .count();
    let more = turn.more.clone();
    let follow_up = !first(&turn.asks);
    if !more.is_empty() && rounds < follow::ROUNDS {
        push_round(ui, at.conversation, more, Vec::new());
        return;
    }
    if follow_up || !more.is_empty() {
        return plan(ui, at.conversation);
    }
    let answers = setup::answers(&asked(ui, at.conversation));
    let context = super::changes::context(ui);
    let candidates = follow::candidates(&answers, &context.settings, &context.state);
    if candidates.is_empty() {
        return plan(ui, at.conversation);
    }
    if on_device(ui, at.conversation) {
        push_round(ui, at.conversation, candidates, Vec::new());
        return;
    }
    if ui.assistant.answering.is_some() {
        return;
    }
    let line = setup::answers_line(&asked(ui, at.conversation), label);
    composer::ask_with(
        ui,
        at.conversation,
        line.clone(),
        Recall::Guide,
        Stage::Follow {
            answers,
            line,
            candidates,
        },
    );
}

/// A follow-up round of the app's: the first [`follow::ROUND`] of `asks`,
/// the rest for later.
fn push_round(ui: &mut Ui, conversation: u32, mut asks: Vec<Ask>, answers: Vec<Answer>) {
    let more = asks.split_off(asks.len().min(follow::ROUND));
    let now = ui.assistant.now_ms();
    ui.assistant.store.push(
        conversation,
        Turn {
            asks,
            more,
            answers,
            created_ms: now,
            ..Turn::default()
        },
    );
}

/// Every answer so far, to the plan: the app's own on a built-in model,
/// else the model's on top of the app's rules.
fn plan(ui: &mut Ui, conversation: u32) {
    let asked = asked(ui, conversation);
    let answers = setup::answers(&asked);
    let line = setup::answers_line(&asked, label);
    if on_device(ui, conversation) {
        let context = super::changes::context(ui);
        let card = setup::plan::plan(Vec::new(), &answers, &context.settings, &context.state);
        let now = ui.assistant.now_ms();
        let answer = if card.as_ref().is_some_and(|card| !card.items.is_empty()) {
            tr("Here is what fits how you work. Untick what you don't need.")
        } else {
            tr("Everything already fits how you work.")
        };
        ui.assistant.store.push(
            conversation,
            Turn {
                answer: answer.to_owned(),
                answers,
                cards: card.into_iter().collect(),
                created_ms: now,
                ..Turn::default()
            },
        );
        return;
    }
    if ui.assistant.answering.is_some() {
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

/// Asks an answers turn again: its follow-ups, or its plan.
pub(super) fn again(ui: &mut Ui, conversation: u32, turn: &Turn) {
    if ui.assistant.answering.is_some() || ui.assistant.fixture {
        return;
    }
    let stage = if turn.follow {
        let context = super::changes::context(ui);
        Stage::Follow {
            candidates: follow::candidates(&turn.answers, &context.settings, &context.state),
            answers: turn.answers.clone(),
            line: turn.question.clone(),
        }
    } else {
        Stage::Plan {
            answers: turn.answers.clone(),
            line: turn.question.clone(),
        }
    };
    composer::ask_with(
        ui,
        conversation,
        turn.question.clone(),
        Recall::Guide,
        stage,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// What the model reads of the answers: ids and the words the person
    /// saw, in the window's language for the app's own.
    #[test]
    fn the_answers_line_names_what_was_seen() {
        let mut shown = setup::questions("macos");
        shown.truncate(2);
        shown[1].chosen = vec!["mixed".into()];
        let line = setup::answers_line(&shown, label);
        assert_eq!(line, "language: mixed (Other or mixed)");
        let mut server = setup::questions("macos").remove(2);
        server.chosen = vec!["custom".into()];
        server.options[4].extra = "LM Studio".into();
        assert_eq!(
            setup::answers_line(&[server], label),
            "cleanup: custom (My own server (LM Studio))"
        );
    }
}
