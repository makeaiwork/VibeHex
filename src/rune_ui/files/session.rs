//! A request session in Requests, as a chat: the first request's answer,
//! then each later question as a bubble with its answer under it. Every
//! answer ends in the same compact row: Copy and Ask again as quiet icons
//! in the same grey as the model and the time, as the mockups draw them,
//! Stopped in amber when it was stopped.

use rune::{El, col, icon, row, text};

use super::super::Ui;
use super::super::controls::Paint;
use super::composer::Target;
use super::submit;
use crate::desktop::i18n::{tr, tr_fill};
use crate::files::store::{FileRecord, Version};
use crate::files::worker::{Job, Stage, Task};

static COPY_ICON: rune::Icon = rune::Icon {
    name: "hex-files-answer-copy",
    view: 16.0,
    body: r#"<rect x="5" y="5" width="8.5" height="8.5" rx="1.2" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M3 10.5V3.2c0-.4.3-.7.7-.7H11" fill="none" stroke="currentColor" stroke-width="1.4"/>"#,
};

static AGAIN_ICON: rune::Icon = rune::Icon {
    name: "hex-files-ask-again",
    view: 16.0,
    body: r#"<path d="M13 8a5 5 0 1 1-1.6-3.7" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/><path d="M13 2.8v2.9h-2.9" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

/// What one answer row says.
pub(in super::super) struct Answered<'a> {
    pub(in super::super) key: String,
    /// What Ask again asks.
    pub(in super::super) question: &'a str,
    pub(in super::super) answer: &'a str,
    pub(in super::super) model: &'a str,
    pub(in super::super) created_ms: u64,
    /// What `created_ms` is told against.
    pub(in super::super) now_ms: u64,
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
            now_ms: crate::files::store::now_ms(),
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
                now_ms: crate::files::store::now_ms(),
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

/// Room a question's bubble always leaves on its left, so in a narrow
/// window it still reads as the person's side of the chat.
const BUBBLE_INDENT: f32 = 72.0;

/// The widest a question's bubble gets, its padding included.
const BUBBLE_MAX: f32 = 460.0;
const BUBBLE_PAD_X: f32 = 12.0;

/// A question, right-aligned in a quiet bubble that hugs its longest line:
/// wrapped text alone would keep the bubble at its widest, leaving a gap on
/// the right of a question that breaks onto a second line.
pub(in super::super) fn bubble(p: Paint, key: &str, question: &str) -> El {
    let c = &p.k.c;
    let question = question.trim().to_owned();
    let (lines_w, _) = rune::text::measure(
        &question,
        &[],
        p.k.ty.ui,
        Some(BUBBLE_MAX - 2.0 * BUBBLE_PAD_X),
    );
    let width = (lines_w + 2.0 * BUBBLE_PAD_X + 1.0).min(BUBBLE_MAX);
    row()
        .key(key.to_owned())
        .child(rune::el().key("push").grow(1.0).min_w(BUBBLE_INDENT))
        .child(
            text(question)
                .key("question")
                .font(p.k.ty.ui)
                .color(c.text_base)
                .wrap()
                .shrink()
                .max_w(width)
                .px(BUBBLE_PAD_X)
                .py(8.0)
                .radius(12.0)
                .fill(c.layer_02),
        )
}

pub(in super::super) fn markdown(p: Paint, source: String, muted: bool, streaming: bool) -> El {
    let style = rune_kit::MarkdownStyle::from_kit(p.k)
        .body_font(p.k.ty.ui)
        .text(if muted { p.muted() } else { p.reading() });
    // Bold words stand out at full brightness; a draft stays muted whole.
    let bright = if muted {
        Vec::new()
    } else {
        super::super::assistant::strong::bright(&source)
    };
    let strong = rune_kit::markdown::Highlight::new()
        .bg(rune::Color::TRANSPARENT)
        .fg(p.k.c.text_base);
    rune_kit::Markdown::new(source)
        .style(style)
        .highlights(bright.into_iter().map(|at| (at, strong.clone())))
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
pub(in super::super) fn answer_row(
    p: Paint,
    answered: &Answered<'_>,
    again: impl Fn(&mut Ui, String) + 'static,
) -> El {
    let copied = answered.answer.to_owned();
    let question = answered.question.to_owned();
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    row()
        .key("row")
        .gap(2.0)
        // No pull to the left: the scroll would clip the hover fill.
        .center_y()
        .child(quiet_icon(
            p,
            "copy",
            &COPY_ICON,
            tr("Copy"),
            move |_, tx| tx.copy(copied.clone()),
        ))
        .child(quiet_icon(
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
            text(when(answered.created_ms, answered.now_ms))
                .key("time")
                .font(small)
                .color(p.faint()),
        )
}

/// A 14 pt icon in the row's grey on a 24 pt target that shows a quiet
/// background on hover.
fn quiet_icon(
    p: Paint,
    key: &str,
    glyph: &'static rune::Icon,
    label: &str,
    press: impl Fn(&mut Ui, &mut rune::Tx) + 'static,
) -> El {
    let c = &p.k.c;
    row()
        .key(key.to_owned())
        .role("button")
        .label(label)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .size(24.0, 24.0)
        .center()
        .radius(6.0)
        .hover(c.hover)
        .press(c.pressed)
        .child(icon(glyph, 14.0).key("glyph").color(p.faint()))
        .on_press_tx(press)
}

/// "Today, 11:21", "Yesterday, 18:05", the weekday within the last week
/// ("Tuesday, 9:30"), else the full date, as of `now`.
pub(in super::super) fn when(ms: u64, now: u64) -> String {
    const DAY: u64 = 86_400_000;
    let day = crate::meeting::day_label(ms, now);
    let day = match day.as_str() {
        "Today" | "Yesterday" => super::day_label(&day).to_owned(),
        // `Friday, March 14, 2025`: the weekday when it is one of the
        // last seven days, else the date.
        date => {
            let recent = (2..7).any(|days| {
                crate::meeting::day_label(ms, now.saturating_sub(days * DAY)) == "Today"
            });
            match date.split_once(", ") {
                Some((weekday, _)) if recent => weekday.to_owned(),
                Some((_, rest)) => rest.to_owned(),
                None => date.to_owned(),
            }
        }
    };
    format!("{day}, {}", crate::meeting::clock(ms))
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
    let status = if task.stage == Stage::CleaningUp {
        tr_fill("{} is cleaning up", model)
    } else {
        tr_fill("{} is answering", model)
    };
    let status = text(status)
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

#[cfg(test)]
mod tests {
    use super::when;

    /// Noon on Saturday 14 March 2026, local time near enough: days are
    /// told by the local calendar, so the test steps whole days from noon.
    const NOW: u64 = 1_773_489_600_000;
    const DAY: u64 = 86_400_000;

    #[test]
    fn dates_read_today_yesterday_a_weekday_then_the_date() {
        let clock = crate::meeting::clock(NOW);
        assert_eq!(when(NOW, NOW), format!("Today, {clock}"));
        assert_eq!(when(NOW - DAY, NOW), format!("Yesterday, {clock}"));
        let weekday = crate::meeting::day_label(NOW - 3 * DAY, NOW);
        let weekday = weekday.split(", ").next().unwrap();
        assert_eq!(when(NOW - 3 * DAY, NOW), format!("{weekday}, {clock}"));
        let date = crate::meeting::day_label(NOW - 30 * DAY, NOW);
        let date = date.split_once(", ").unwrap().1;
        assert_eq!(when(NOW - 30 * DAY, NOW), format!("{date}, {clock}"));
        assert!(date.ends_with("2026"), "{date}");
    }
}
