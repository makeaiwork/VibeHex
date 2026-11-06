//! A request session in Requests, as a chat: the first request's answer,
//! then each later question as a bubble with its answer under it. Every
//! answer ends in the same compact row: Copy and Ask again as icons, the
//! model in grey, Stopped in amber when it was stopped, and the time.

use rune::{El, col, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::super::list_detail as ld;
use super::composer::Target;
use super::submit;
use crate::desktop::i18n::{tr, tr_fill};
use crate::files::store::{FileRecord, Version};
use crate::files::worker::{Job, Task};

static AGAIN_ICON: rune::Icon = rune::Icon {
    name: "hex-files-ask-again",
    view: 16.0,
    body: r#"<path d="M12.8 6.2A5 5 0 1 0 13 9.5M13 3v3.4H9.6" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

/// What one answer row says.
pub(in super::super) struct Answered<'a> {
    pub(in super::super) key: String,
    /// What Ask again asks.
    pub(in super::super) question: &'a str,
    pub(in super::super) answer: &'a str,
    pub(in super::super) model: &'a str,
    pub(in super::super) created_ms: u64,
    pub(in super::super) stopped: bool,
}

/// The session's chat: `first` is the request as a chip named it (no
/// bubble) or as typed (a bubble).
pub(super) fn chat(ui: &Ui, p: Paint, record: &FileRecord, version: &Version, typed: bool) -> El {
    let mut chat = col().key("session").gap(14.0);
    let first = version.prompt.as_deref().unwrap_or(version.name.as_str());
    if typed {
        chat = chat.child(bubble(p, "asked0", first));
    }
    let session = version.id;
    chat = chat.child(answer(
        p,
        Answered {
            key: "answer0".into(),
            question: first,
            answer: &version.text,
            model: &version.model,
            created_ms: version.created_ms,
            stopped: version.stopped,
        },
        move |ui: &mut Ui, question| ask_again(ui, session, question),
    ));
    for (index, asked) in version.follow_ups.iter().enumerate() {
        chat = chat.child(bubble(p, &format!("asked{}", index + 1), &asked.question));
        chat = chat.child(answer(
            p,
            Answered {
                key: format!("answer{}", index + 1),
                question: &asked.question,
                answer: &asked.answer,
                model: &asked.model,
                created_ms: asked.created_ms,
                stopped: asked.stopped,
            },
            move |ui: &mut Ui, question| ask_again(ui, session, question),
        ));
    }
    if let Some(task) = ui
        .files
        .tasks_of(&record.id)
        .find(|task| task.session == Some(version.id))
    {
        chat = chat.children(answering(p, task));
    }
    chat
}

/// A question, right-aligned in a quiet bubble.
pub(in super::super) fn bubble(p: Paint, key: &str, question: &str) -> El {
    let c = &p.k.c;
    row()
        .key(key.to_owned())
        .child(rune::el().key("push").grow(1.0))
        .child(
            text(question.trim().to_owned())
                .key("question")
                .font(p.k.ty.ui)
                .color(c.text_base)
                .wrap()
                .shrink()
                .max_w(460.0)
                .px(12.0)
                .py(8.0)
                .radius(12.0)
                .fill(c.layer_02),
        )
}

fn markdown(p: Paint, source: String, muted: bool, streaming: bool) -> El {
    let style = rune_kit::MarkdownStyle::from_kit(p.k)
        .body_font(p.k.ty.ui)
        .text(if muted { p.muted() } else { p.reading() });
    rune_kit::Markdown::new(source)
        .style(style)
        .selectable(!streaming)
        .streaming(streaming)
        .tasks_read_only()
        .build(p.k)
        .key("markdown")
}

/// An answer and its row; Ask again calls `again` with the question.
pub(in super::super) fn answer(
    p: Paint,
    answered: Answered<'_>,
    again: impl Fn(&mut Ui, String) + 'static,
) -> El {
    let body = if answered.answer.trim().is_empty() {
        text(tr("This version is empty."))
            .key("empty")
            .font(p.k.ty.ui)
            .color(p.faint())
    } else {
        markdown(p, answered.answer.to_owned(), false, false)
    };
    col()
        .key(answered.key.clone())
        .gap(6.0)
        .child(body)
        .child(answer_row(p, &answered, again))
}

/// Copy and Ask again, the model, Stopped, and the time.
fn answer_row(p: Paint, answered: &Answered<'_>, again: impl Fn(&mut Ui, String) + 'static) -> El {
    let copied = answered.answer.to_owned();
    let question = answered.question.to_owned();
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    row()
        .key("row")
        .gap(2.0)
        .center_y()
        .child(ld::ghost_icon(
            p,
            "copy",
            ld::COPY_ICON,
            tr("Copy"),
            move |_, tx| tx.copy(copied.clone()),
        ))
        .child(ld::ghost_icon(
            p,
            "again",
            &AGAIN_ICON,
            tr("Ask again"),
            move |ui: &mut Ui, _| again(ui, question.clone()),
        ))
        .child(
            text(answered.model.to_owned())
                .key("model")
                .font(small)
                .color(p.faint())
                .ml(4.0)
                .max_lines(1),
        )
        .children(answered.stopped.then(|| {
            text(tr("Stopped"))
                .key("stopped")
                .font(small)
                .color(p.warning())
                .ml(8.0)
        }))
        .child(rune::el().key("push").grow(1.0))
        .child(
            text(when(answered.created_ms))
                .key("time")
                .font(small)
                .color(p.faint()),
        )
}

/// "Today, 11:21", as the list names days.
pub(in super::super) fn when(ms: u64) -> String {
    let now = crate::files::store::now_ms();
    let day = crate::meeting::day_label(ms, now);
    format!("{}, {}", super::day_label(&day), crate::meeting::clock(ms))
}

/// Asks a question of the session again, as its next question.
fn ask_again(ui: &mut Ui, session: u32, question: String) {
    if super::composer::running(ui, Target::Session(session)).is_some() {
        return;
    }
    let Some(record) = ui.files.selected.clone() else {
        return;
    };
    submit(ui, &record, Job::FollowUp { session, question });
    ui.files.stale = true;
}

/// The question being answered and what came so far.
pub(in super::super) fn answering(p: Paint, task: &Task) -> Vec<El> {
    let mut shown = Vec::new();
    if let Some(question) = &task.question {
        shown.push(bubble(p, "asking", question));
    }
    let model = task.model.as_deref().unwrap_or("Codex");
    let status = text(tr_fill("{} is answering", model))
        .key("answering")
        .font(rune::TextStyle {
            size: 12.0,
            ..p.k.ty.ui
        })
        .color(p.faint());
    match &task.draft {
        Some(draft) if !draft.text.trim().is_empty() => {
            let mut source = String::new();
            if draft.cut {
                source.push_str("…\n\n");
            }
            source.push_str(draft.text.trim_end());
            source.push('▌');
            shown.push(
                col()
                    .key("draft")
                    .gap(6.0)
                    .child(markdown(p, source, true, true))
                    .child(status),
            );
        }
        _ => shown.push(status),
    }
    shown
}
