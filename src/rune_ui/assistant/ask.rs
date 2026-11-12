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
use crate::assistant::changes::{Card, CardKind};
use crate::assistant::recordings::Step;
use crate::assistant::setup::since::{self, Since};
use crate::assistant::setup::{self, Answer, Ask, Choice, Stage, follow, past};
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
    let _speaking = crate::assistant::language::speak(&ui.settings);
    if ui.assistant.answering.is_some() {
        return;
    }
    let since = super::returning::since(ui);
    start(ui, since);
}

/// Opens setup: a first one with round one's questions, a returning one
/// (`since`) with what came since and round one folded. On a remote model
/// with the use read, the model first looks at what came since and
/// suggests a few things (`Stage::Look`), and round one comes after.
pub(super) fn start(ui: &mut Ui, since: Option<Since>) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let context = super::changes::context(ui);
    let mut asks = setup::questions(context.state.os);
    setup::preselect(&mut asks, &context.settings, &context.state);
    if let Some(since) = &since {
        since::preselect(&mut asks, since);
        setup::preselect(&mut asks, &context.settings, &context.state);
    }
    let now = ui.assistant.now_ms();
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
    let shown = since.as_ref().map(since::paragraph).unwrap_or_default();
    let facts = since.as_ref().map(since::prompt).unwrap_or_default();
    let terms = since
        .as_ref()
        .and_then(|since| since.usage.as_ref())
        .map(since::candidates)
        .unwrap_or_default();
    let looked = since
        .as_ref()
        .and_then(|since| since.usage.as_ref())
        .map(|usage| Step::Looked {
            dictations: usage.dictations,
            meetings: usage.meetings,
        })
        .filter(|_| !on_device(ui, id));
    let opening = Turn {
        now: setup::now::now(&context.facts()),
        since: shown.clone(),
        created_ms: now,
        ..Turn::default()
    };
    let round = Turn {
        asks,
        facts,
        folded: since.is_some(),
        terms,
        created_ms: now,
        ..Turn::default()
    };
    if let Some(step) = looked {
        ui.assistant.store.push(id, opening);
        let look = Stage::Look {
            facts: round.facts.clone(),
            shown,
            asks: round.asks.clone(),
            terms: round.terms.clone(),
            step,
        };
        if composer::ask_with(ui, id, String::new(), Recall::Guide, look) {
            return;
        }
        // Not asked after all: round one at once, as on a built-in model.
        ui.assistant.store.push(id, round);
        return;
    }
    ui.assistant.store.push(
        id,
        Turn {
            now: opening.now,
            since: opening.since,
            ..round
        },
    );
}

/// Looks again at what came since: a returning setup's opening asked
/// again, from the turn that showed it.
pub(super) fn look_again(ui: &mut Ui, conversation: u32, turn: &Turn) {
    if ui.assistant.answering.is_some() || ui.assistant.fixture {
        return;
    }
    let Some(step) = turn.steps.first().cloned() else {
        return;
    };
    let shown = ui
        .assistant
        .store
        .get(conversation)
        .and_then(|open| {
            open.turns
                .iter()
                .rev()
                .find(|turn| !turn.since.is_empty())
                .map(|turn| turn.since.clone())
        })
        .unwrap_or_default();
    let mut asks = turn.asks.clone();
    for ask in &mut asks {
        ask.done = false;
    }
    let look = Stage::Look {
        facts: turn.facts.clone(),
        shown,
        asks,
        terms: turn.terms.clone(),
        step,
    };
    composer::ask_with(ui, conversation, String::new(), Recall::Guide, look);
}

/// Whether `turn` is a returning setup's look: what the app read, a model's
/// suggestions, and round one after them.
pub(super) fn is_look(turn: &Turn) -> bool {
    turn.folded
        && first(&turn.asks)
        && turn
            .steps
            .iter()
            .any(|step| matches!(step, Step::Looked { .. }))
}

/// The terms the open setup's use showed, for its plan.
fn terms(ui: &Ui, conversation: u32) -> Vec<since::Candidate> {
    ui.assistant
        .store
        .get(conversation)
        .and_then(|open| {
            setup_turns(&open.turns)
                .first()
                .map(|turn| turn.terms.clone())
        })
        .unwrap_or_default()
}

/// A question's words as the person sees them: the app's own in the
/// window's language, the model's as written.
fn words<'a>(ask: &Ask, words: &'a str) -> &'a str {
    if ask.own { tr(words) } else { words }
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
        shown.push(super::recall::answer(p, &turn.now, None, None, (false, false)).key("now"));
    }
    if !turn.since.trim().is_empty() {
        shown.push(super::recall::answer(p, &turn.since, None, None, (false, false)).key("since"));
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
    let unfolded = pane.before_open.contains(&at);
    if turn.asks.iter().all(|ask| ask.done) {
        shown.extend(past(p, at, &turn.asks, unfolded));
    } else if last && turn.folded {
        shown.push(super::returning::folded(p, at, &turn.asks));
    } else if last {
        shown.push(open(p, at, &turn.asks, unfolded));
    }
    shown
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
/// Continue, and "Skip to the plan" after round one.
fn open(p: Paint, at: At, asks: &[Ask], unfolded: bool) -> El {
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
    card = card.children(before(p, at, asks, unfolded, true));
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
pub(super) fn pick(ui: &mut Ui, at: At, number: usize, id: &str) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
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
pub(super) fn answered(ui: &mut Ui, at: At, skip: bool) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
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
        // An answer from before stands, skipped or not.
        if !shows || (skip && !ask.touched && !ask.before) {
            ask.chosen.clear();
        }
    }
    set_round(ui, at, asks);
    // Kept at once: left half-way, or its plan put off, the answers stay.
    keep(ui, at.conversation, None);
    if skip {
        plan(ui, at.conversation);
    } else {
        next(ui, at);
    }
}

/// The turns of the open setup, from its round one.
pub(super) fn setup_turns(turns: &[Turn]) -> &[Turn] {
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
        push_round(ui, at.conversation, more, Vec::new(), Vec::new());
        return;
    }
    if follow_up || !more.is_empty() {
        return plan(ui, at.conversation);
    }
    let answers = setup::answers(&asked(ui, at.conversation));
    let context = super::changes::context(ui);
    let candidates = follow::candidates(&answers, &context.settings, &context.state);
    // What was answered in an earlier setup isn't asked again: it joins
    // the round folded, as answered then, or, with nothing new to ask,
    // round one's answers.
    let before = follow::before(&answers, &context.settings, &context.state);
    if candidates.is_empty() {
        let done = before
            .into_iter()
            .map(|ask| Ask { done: true, ..ask })
            .collect();
        ui.assistant.store.push_asks(at.conversation, at.turn, done);
        return plan(ui, at.conversation);
    }
    if on_device(ui, at.conversation) {
        push_round(ui, at.conversation, candidates, Vec::new(), before);
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
            since: super::returning::facts(ui, at.conversation),
            before,
        },
    );
}

/// A follow-up round of the app's: the first [`follow::ROUND`] of `asks`,
/// the rest for later, and what was answered `before`, folded.
fn push_round(
    ui: &mut Ui,
    conversation: u32,
    mut asks: Vec<Ask>,
    answers: Vec<Answer>,
    before: Vec<Ask>,
) {
    let more = asks.split_off(asks.len().min(follow::ROUND));
    asks.extend(before);
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
        let terms = terms(ui, conversation);
        let card = setup::plan::with_terms(
            Vec::new(),
            &answers,
            &terms,
            &context.settings,
            &context.state,
        );
        let now = ui.assistant.now_ms();
        let answer = if card.as_ref().is_some_and(|card| !card.items.is_empty()) {
            tr("Here is what fits how you work. Untick what you don't need.")
        } else {
            tr("Everything already fits how you work.")
        };
        let settled = card.is_none();
        ui.assistant.store.push(
            conversation,
            Turn {
                answer: answer.to_owned(),
                answers: answers.clone(),
                cards: card.into_iter().collect(),
                created_ms: now,
                ..Turn::default()
            },
        );
        if settled {
            // Nothing to decide: the setup is finished as it is.
            keep(ui, conversation, None);
        }
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
        Stage::Plan {
            answers,
            line,
            since: super::returning::facts(ui, conversation),
            terms: terms(ui, conversation),
        },
    );
}

/// A setup's plan was applied or put off: the setup is finished, and kept
/// with what it applied and what the person declined (`setup::past`).
pub(super) fn decided(ui: &mut Ui, at: super::changes::At) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let Some(open) = ui.assistant.store.get(at.conversation) else {
        return;
    };
    let Some(turn) = open.turns.get(at.turn) else {
        return;
    };
    let Some(card) = turn.cards.get(at.card) else {
        return;
    };
    if turn.answers.is_empty() || turn.follow || card.kind != CardKind::Plan {
        return;
    }
    let plan_ms = turn.created_ms;
    let done = ui.assistant.store.setups().iter().any(|setup| {
        // An older record, kept only once decided, has no conversation.
        plan_ms != 0
            && (setup.plan_ms == plan_ms || (setup.conversation == 0 && setup.at_ms == plan_ms))
    });
    if done {
        return;
    }
    let card = card.clone();
    keep(ui, at.conversation, Some((&card, plan_ms)));
}

/// Keeps the setup running in `conversation` with every answer so far
/// (`setup::past`), and with how its plan went once decided (`plan`: the
/// card and its turn's time); a later call updates the same record.
fn keep(ui: &mut Ui, conversation: u32, plan: Option<(&Card, u64)>) {
    let asked = asked(ui, conversation);
    let answers = setup::answers(&asked);
    let started_ms = ui
        .assistant
        .store
        .get(conversation)
        .and_then(|open| setup_turns(&open.turns).first().map(|turn| turn.created_ms))
        .unwrap_or_default();
    let at_ms = plan.map_or_else(|| ui.assistant.now_ms(), |(_, at)| at);
    let mut setup = past::finished(
        &asked,
        &answers,
        plan.map(|(card, _)| card),
        &ui.settings,
        at_ms,
    );
    setup.conversation = conversation;
    setup.started_ms = started_ms;
    setup.plan_ms = plan.map_or(0, |(_, at)| at);
    ui.assistant.store.finish(setup);
}

/// Asks an answers turn again: its follow-ups, or its plan.
pub(super) fn again(ui: &mut Ui, conversation: u32, turn: &Turn) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    if ui.assistant.answering.is_some() || ui.assistant.fixture {
        return;
    }
    let stage = if turn.follow {
        let context = super::changes::context(ui);
        Stage::Follow {
            candidates: follow::candidates(&turn.answers, &context.settings, &context.state),
            before: follow::before(&turn.answers, &context.settings, &context.state),
            answers: turn.answers.clone(),
            line: turn.question.clone(),
            since: super::returning::facts(ui, conversation),
        }
    } else {
        Stage::Plan {
            answers: turn.answers.clone(),
            line: turn.question.clone(),
            since: super::returning::facts(ui, conversation),
            terms: terms(ui, conversation),
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
}
