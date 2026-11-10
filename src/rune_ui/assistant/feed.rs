//! The open conversation as a chat: each question in a bubble on the
//! right, each reply open beside a small face, with the guide articles it
//! drew on and the row Files' answers end in (Copy, Ask again, the model,
//! the time). A new conversation shows a greeting and a few questions to
//! ask instead.

use rune::{El, col, icon, row, text};

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint};
use super::super::files::session::{self, Answered};
use super::super::icons;
use super::super::lists::small_button;
use super::composer::ask;
use super::face::{Mood, face};
use crate::assistant::guide;
use crate::assistant::store::Turn;
use crate::assistant::worker::Answering;
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// The questions a new conversation offers.
const EXAMPLES: [&str; 4] = [
    "Which Labs features are worth turning on for me?",
    "Some words come out wrong. How do I fix that?",
    "Which speech model is best for Russian?",
    "Can I use my ChatGPT plan to clean up dictation?",
];

/// The face beside a reply.
const FACE: f32 = 20.0;

/// The open conversation, or the start of a new one (`greet`).
pub(super) fn feed(ui: &Ui, p: Paint, greet: bool, still: bool) -> El {
    let pane = &ui.assistant;
    let mut feed = col().key("assistant-feed").gap(18.0);
    if let Some(error) = &pane.error {
        feed = feed.child(
            text(tr_fill("The question wasn't answered: {}", error))
                .key("error")
                .font(DESCRIPTION)
                .color(p.danger())
                .wrap(),
        );
    }
    let Some(conversation) = pane.conversation() else {
        return if greet { feed.child(start(p)) } else { feed };
    };
    if conversation.turns.is_empty() && pane.consent.is_none() && pane.answering.is_none() {
        return if greet { feed.child(start(p)) } else { feed };
    }
    for (index, turn) in conversation.turns.iter().enumerate() {
        feed = feed
            .child(session::bubble(p, &format!("asked{index}"), &turn.question))
            .child(reply(ui, p, conversation.id, index, turn));
    }
    if let Some(answering) = pane.answering_in(pane.open) {
        let mood = pane.face.mood(true, still);
        feed = feed.children(coming(p, answering, mood));
    } else if let Some(pending) = pane
        .consent
        .as_ref()
        .filter(|pending| Some(pending.conversation) == pane.open)
    {
        feed = feed.children(super::recall::consent(ui, p, pending));
    }
    feed
}

/// A greeting and the example questions, two by two.
fn start(p: Paint) -> El {
    let mut grid = col().key("examples").gap(8.0);
    for (index, pair) in EXAMPLES.chunks(2).enumerate() {
        let mut line = row().key(format!("line{index}")).gap(8.0);
        for (offset, question) in pair.iter().enumerate() {
            line = line.child(example(p, index * 2 + offset, tr(question)));
        }
        grid = grid.child(line);
    }
    col()
        .key("assistant-start")
        .pt(28.0)
        .gap(6.0)
        .child(
            text(tr("Ask anything about VibeHex"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(
            text(tr(
                "Whether it can do something, how to set it up, which model to use",
            ))
            .key("lead")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap(),
        )
        .child(grid.mt(12.0))
        .child(knows(p))
}

/// What the guide covers, under the examples.
fn knows(p: Paint) -> El {
    let articles = guide::articles();
    let labs = articles.iter().filter(|article| article.is_lab()).count();
    let topics = articles.len() - labs;
    row()
        .key("knows")
        .mt(10.0)
        .gap(8.0)
        .center_y()
        .child(
            icon(&icons::BOOK, 13.0)
                .key("glyph")
                .color(p.k.c.icon_muted),
        )
        .child(
            text(tr_named(
                "Answers from the guide: {labs} Labs features and {topics} topics",
                &[("labs", &labs.to_string()), ("topics", &topics.to_string())],
            ))
            .key("words")
            .font(p.k.ty.small)
            .color(p.faint())
            .wrap()
            .shrink(),
        )
}

/// An example question: pressing it asks it.
fn example(p: Paint, index: usize, question: &'static str) -> El {
    let c = &p.k.c;
    row()
        .key(format!("example{index}"))
        .role("button")
        .label(question)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .w(0.0)
        .grow(1.0)
        .min_h(52.0)
        .px(12.0)
        .py(9.0)
        .center_y()
        .radius(p.k.radius.md)
        .border(p.hair, c.border_weak)
        .hover(c.hover)
        .press(c.pressed)
        .child(
            text(question)
                .key("question")
                .font(p.k.ty.ui)
                .color(p.text())
                .wrap()
                .shrink(),
        )
        .on_press(move |ui: &mut Ui| {
            ask(ui, question.to_owned());
        })
}

/// A reply beside the face: its words, then what stopped or failed it, the
/// guide articles it names, and its row.
fn reply(ui: &Ui, p: Paint, conversation: u32, index: usize, turn: &Turn) -> El {
    let now_ms = ui.assistant.now_ms();
    let mut words = col().key("words").grow(1.0).shrink().min_w(0.0).gap(8.0);
    words = words.children(super::recall::folded(ui, p, conversation, index, turn));
    if !turn.answer.trim().is_empty() {
        words = words.child(super::recall::answer(
            p,
            &turn.answer,
            Some((conversation, index)),
            false,
            false,
        ));
    }
    if let Some(error) = &turn.error {
        let question = turn.question.clone();
        words = words.child(
            row()
                .key("failed")
                .gap(10.0)
                .center_y()
                .child(
                    text(tr_fill("The question wasn't answered: {}", error))
                        .key("error")
                        .font(DESCRIPTION)
                        .color(p.danger())
                        .wrap()
                        .shrink(),
                )
                .child(small_button(p, "retry", tr("Try again"), move |ui, _| {
                    ask(ui, question.clone());
                })),
        );
    }
    words = words.children(super::recall::chips(
        p,
        &turn.refs,
        now_ms,
        (conversation, index),
    ));
    words = words.children(sources(p, &turn.sources));
    if turn.error.is_none() {
        words = words.child(session::answer_row(
            p,
            &Answered {
                key: "row".into(),
                question: &turn.question,
                answer: &turn.answer,
                model: &turn.model,
                created_ms: turn.created_ms,
                now_ms,
                stopped: turn.stopped,
            },
            |ui: &mut Ui, question| {
                ask(ui, question);
            },
        ));
    }
    beside_face(p, &format!("answer{index}"), Mood::default(), words)
}

/// `words` with the face at their left, which says who wrote them.
pub(super) fn beside_face(p: Paint, key: &str, mood: Mood, words: El) -> El {
    let c = &p.k.c;
    row()
        .key(key.to_owned())
        .gap(10.0)
        .items_start()
        .child(face("face", FACE, c.icon_muted, c.icon_base, mood).mt(1.0))
        .child(words)
}

/// "From the guide" and the articles a reply names, by their titles.
fn sources(p: Paint, ids: &[String]) -> Option<El> {
    let c = &p.k.c;
    let titles: Vec<&str> = ids
        .iter()
        .filter_map(|id| guide::article(id))
        .map(|article| article.title.as_str())
        .collect();
    if titles.is_empty() {
        return None;
    }
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    let mut line = row().key("sources").gap(6.0).center_y().flex_wrap().child(
        text(tr("From the guide"))
            .key("label")
            .font(small)
            .color(p.faint()),
    );
    for (index, title) in titles.into_iter().enumerate() {
        line = line.child(
            row()
                .key(format!("source{index}"))
                .role("note")
                .label(title)
                .h(22.0)
                .px(7.0)
                .gap(5.0)
                .center_y()
                .radius(p.k.radius.sm)
                .border(p.hair, c.border_weak)
                .child(icon(&icons::BOOK, 12.0).key("glyph").color(c.icon_muted))
                .child(
                    text(title.to_owned())
                        .key("title")
                        .font(small)
                        .color(p.muted())
                        .max_lines(1),
                ),
        );
    }
    Some(line)
}

/// The question being answered and what came so far, the face reading.
fn coming(p: Paint, answering: &Answering, mood: Mood) -> Vec<El> {
    let steps = super::recall::live(p, answering);
    let words = if !answering.draft.trim().is_empty() {
        let mut draft = answering.draft.trim_end().to_owned();
        draft.push('▌');
        Some(super::recall::answer(p, &draft, None, true, true))
    } else if steps.is_none() {
        Some(
            text(tr("Looking in the guide…"))
                .key("looking")
                .font(DESCRIPTION)
                .color(p.faint())
                .mt(2.0),
        )
    } else {
        None
    };
    vec![
        session::bubble(p, "asking", &answering.question),
        beside_face(
            p,
            "answering",
            mood,
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(10.0)
                .children(steps)
                .children(words),
        ),
    ]
}
