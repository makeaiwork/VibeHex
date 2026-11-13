//! Questions about the person's own recordings, in the chat (the
//! Consent, Steps, and Recordings boards):
//!
//! - the first such question, before anything is read, gets a card naming
//!   the model the passages would go to: Allow, or Answer without
//!   recordings. Allow is remembered (`assistant.read_recordings`) and
//!   Assistant › Settings takes it back;
//! - while it works, the search's steps land one by one, each with a
//!   check, the current one with three dots, the face reading;
//! - a finished answer folds them into one grey line, "Read 4 meetings and
//!   1 file in 9 s ›", which opens on a click; a guide answer has none;
//! - its `[n]` show as small numbered marks, and "From your recordings"
//!   lists the places cited ("Meeting title, Tuesday, 10:12"). A mark or a
//!   chip opens its place (`jump`).
//!
//! What a question may read is decided here (`recall`): only the question's
//! words before the person has agreed (`assistant::intent`).

use std::rc::Rc;

use rune::{Color, El, Icon, LinkPaint, Span, Tx, col, el, icon, row, text};
use rune_kit::markdown::Highlight;

use super::super::Ui;
use super::super::controls::{ButtonKind, DESCRIPTION, Paint, button};
use super::super::files::session;
use super::super::sidebar::Pane;
use super::face::Mood;
use super::places::Known;
use super::strong;
use crate::assistant::agent::trace::Did;
use crate::assistant::agent::{live as agent_live, words};
use crate::assistant::intent;
use crate::assistant::recordings::{Allowed, Kind, Ref, Sources, Step, cite};
use crate::assistant::store::Turn;
use crate::assistant::worker::{Answering, Recall};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// A question waiting for the person to say whether recordings may be read.
#[derive(Clone, Debug)]
pub(in super::super) struct Pending {
    pub(super) conversation: u32,
    pub(super) question: String,
}

static LOCK: Icon = Icon {
    name: "hex-assistant-lock",
    view: 24.0,
    body: r#"<rect x="5" y="11" width="14" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M8 11V8a4 4 0 0 1 8 0v3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>"#,
};

/// What `question` may read. `None`: it is about the person's recordings
/// and they haven't allowed reading them, so the card asks first. A
/// follow-up that only refers back ("ответь на последний вопрос", "tell me
/// more") reads what the conversation's last question about recordings did.
pub(super) fn recall(ui: &mut Ui, conversation: u32, question: &str) -> Option<Recall> {
    let now = ui.assistant.now_ms();
    let earlier = super::composer::asked_before(ui, conversation);
    let earlier: Vec<&str> = earlier.iter().map(String::as_str).collect();
    match intent::carried(question, &earlier, now) {
        Some(intent) => allowed(ui, conversation, question, intent),
        None => Some(Recall::Guide),
    }
}

/// What a question with `intent` may read (`intent::reads`); `None` once
/// the consent card is up for it.
pub(super) fn allowed(
    ui: &mut Ui,
    conversation: u32,
    question: &str,
    intent: intent::Intent,
) -> Option<Recall> {
    let now = ui.assistant.now_ms();
    match intent::reads(intent, &ui.settings, Sources::here(), now) {
        intent::Reads::Recall(recall) => Some(recall),
        intent::Reads::Consent => {
            ui.assistant.consent = Some(Pending {
                conversation,
                question: question.to_owned(),
            });
            None
        }
    }
}

/// The model the open conversation `id` runs on, which the card names.
fn engine_of(ui: &Ui, id: u32) -> crate::files::rewrite::Engine {
    match ui.assistant.store.get(id) {
        Some(conversation) => super::engine(
            &ui.settings,
            conversation.engine.as_deref(),
            conversation.effort.as_deref(),
        ),
        None => super::engine(&ui.settings, None, None),
    }
}

/// Why the card is there, over it.
fn why(p: Paint) -> El {
    text(tr(
        "This is a question about your recordings. To answer, I need to read them.",
    ))
    .key("why")
    .font(p.k.ty.ui)
    .color(p.reading())
    .wrap_anywhere()
}

/// The question waiting, the face saying why, and the card.
pub(super) fn consent(ui: &Ui, p: Paint, pending: &Pending) -> Vec<El> {
    let card = consent_card(
        p,
        &engine_of(ui, pending.conversation),
        tr("Answer without recordings"),
        |ui: &mut Ui, allow| decide(ui, allow),
    );
    let words = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(10.0)
        .child(why(p))
        .child(card);
    vec![
        session::bubble(p, "waiting", &pending.question),
        super::feed::beside_face(p, "consent-face", Mood::default(), words),
    ]
}

/// An answer on its way asked to read the recordings and waits for the
/// person: the same card, under the steps so far. Allow is remembered and
/// the answer goes on with them; without, it goes on from the guide, and
/// this conversation isn't asked again.
pub(super) fn asking(ui: &Ui, p: Paint, answering: &Answering) -> Option<El> {
    answering.asking.then(|| {
        col()
            .key("asking")
            .gap(10.0)
            .child(why(p))
            .child(consent_card(
                p,
                &engine_of(ui, answering.conversation),
                tr("Answer without recordings"),
                |ui: &mut Ui, allow| {
                    if ui.is_fixture() || ui.assistant.fixture {
                        return;
                    }
                    ui.update(|settings| settings.assistant.read_recordings = Some(allow));
                    agent_live::decide(Allowed::from_settings(&ui.settings.assistant));
                },
            ))
    })
}

/// The card asking to read the person's recordings: what goes to the
/// model `engine` names, Allow, and `without` (answering without them);
/// `decide` takes the choice. Requests show it too.
pub(in super::super) fn consent_card(
    p: Paint,
    engine: &crate::files::rewrite::Engine,
    without: &str,
    decide: impl Fn(&mut Ui, bool) + Clone + 'static,
) -> El {
    let c = &p.k.c;
    let goes = if engine.is_local() {
        tr("VibeHex searches them on this computer. The model that reads the passages runs here too.")
            .to_owned()
    } else {
        tr_fill(
            "VibeHex searches them on this computer and sends only the passages it finds to {}. Recordings themselves never leave.",
            &engine.model_name(),
        )
    };
    let refuse = decide.clone();
    row()
        .key("consent")
        .gap(12.0)
        .items_start()
        .px(14.0)
        .py(14.0)
        .radius(p.k.radius.md)
        .border(p.hair, c.border_focus)
        .fill(c.layer_01)
        .child(
            el().key("lock")
                .size(28.0, 28.0)
                .center()
                .radius(p.k.radius.sm)
                .fill(c.layer_02)
                .child(icon(&LOCK, 15.0).key("glyph").color(p.accent())),
        )
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(4.0)
                .child(
                    text(tr("Assistant"))
                        .key("eyebrow")
                        .font(p.k.ty.small)
                        .color(p.faint()),
                )
                .child(
                    text(tr("Read your meetings, files, and dictations?"))
                        .key("title")
                        .font(p.k.ty.body_medium)
                        .color(p.text())
                        .wrap_anywhere(),
                )
                .child(
                    text(goes)
                        .key("body")
                        .font(DESCRIPTION)
                        .color(p.muted())
                        .wrap_anywhere(),
                )
                .child(
                    row()
                        .key("choices")
                        .mt(8.0)
                        .gap(8.0)
                        .child(button(
                            p,
                            "allow",
                            tr("Allow"),
                            ButtonKind::Primary,
                            move |ui, _| decide(ui, true),
                        ))
                        .child(button(
                            p,
                            "without",
                            without,
                            ButtonKind::Secondary,
                            move |ui, _| refuse(ui, false),
                        )),
                ),
        )
}

/// Allow (remembered) or answer from the guide alone; then asks.
fn decide(ui: &mut Ui, allow: bool) {
    let Some(pending) = ui.assistant.consent.take() else {
        return;
    };
    if ui.is_fixture() || ui.assistant.fixture {
        return;
    }
    ui.update(|settings| settings.assistant.read_recordings = Some(allow));
    let recall = if allow {
        recall(ui, pending.conversation, &pending.question).unwrap_or(Recall::Guide)
    } else {
        Recall::Declined
    };
    super::composer::ask_with(
        ui,
        pending.conversation,
        pending.question,
        recall,
        crate::assistant::setup::Stage::None,
    );
}

/// While it works: each step with a check, then the current one. An
/// agent turn lists what it read so far (the articles the app laid out
/// only once it read past them), then what it is doing now.
pub(super) fn live(p: Paint, answering: &Answering, dots: Option<u8>) -> Option<El> {
    let drafted = !answering.draft.trim().is_empty();
    if !agent(answering) {
        return live_steps(p, &answering.steps, answering.searching, drafted, dots);
    }
    if !reads(answering) {
        return None;
    }
    let mut list = col()
        .key("steps")
        .gap(6.0)
        .children(trace_lines(p, &answering.trace));
    // A search under way: its steps as they land.
    for (index, step) in answering.steps.iter().enumerate() {
        list = list.child(step_line(
            p,
            &format!("found{index}"),
            true,
            &words::step(step),
            None,
        ));
    }
    // The web's own line says it while it searches, the card while it asks.
    let doing = answering
        .doing
        .filter(|_| !drafted && !answering.asking && !answering.web.went_online())
        .map(|doing| words::doing(doing, true));
    Some(match doing {
        Some(doing) => list.child(step_line(p, "doing", false, doing, dots)),
        None => list,
    })
}

/// The answer is an agent's: it keeps a trace.
fn agent(answering: &Answering) -> bool {
    !answering.trace.is_empty() || answering.doing.is_some() || answering.asking
}

/// An agent's answer has steps of its own to show: it read past the
/// articles the app laid out, is reading, or asks.
pub(super) fn reads(answering: &Answering) -> bool {
    agent(answering)
        && (words::read_more(&answering.trace)
            || matches!(
                answering.doing,
                Some(crate::assistant::agent::trace::Doing::Reading(_))
            )
            || answering.asking)
}

/// `lines`, each with a check.
fn done_lines(p: Paint, lines: &[String]) -> El {
    let mut list = col().key("steps").gap(6.0);
    for (index, line) in lines.iter().enumerate() {
        list = list.child(step_line(p, &format!("step{index}"), true, line, None));
    }
    list
}

/// The search's `steps` so far, each with a check, then what goes on: the
/// search while `searching`, else the answer until it is `drafted`; its
/// dots at `dots` (`Dots::phase`).
pub(in super::super) fn live_steps(
    p: Paint,
    steps: &[Step],
    searching: bool,
    drafted: bool,
    dots: Option<u8>,
) -> Option<El> {
    if steps.is_empty() && !searching {
        return None;
    }
    let lines: Vec<String> = steps.iter().map(words::step).collect();
    let list = done_lines(p, &lines);
    let doing = if searching {
        tr("Searching your recordings")
    } else if !drafted {
        tr("Writing the answer")
    } else {
        return Some(list);
    };
    Some(list.child(step_line(p, "doing", false, doing, dots)))
}

/// One step: a check when done, three dots while it goes on, their wave
/// at `dots` (`None`: still).
pub(super) fn step_line(p: Paint, key: &str, done: bool, words: &str, dots: Option<u8>) -> El {
    let c = &p.k.c;
    let mark = if done {
        icon(&rune_theme_opencode::icons::CHECK_SMALL, 14.0)
            .key("mark")
            .color(p.success())
    } else {
        let dot = |key: &str, alpha: f32| {
            el().key(key.to_owned())
                .size(3.0, 3.0)
                .radius(1.5)
                .fill(p.k.alpha(c.icon_base, alpha))
        };
        let [a, b, c] = super::dots::alphas(dots);
        row()
            .key("mark")
            .size(14.0, 14.0)
            .gap(2.0)
            .center()
            .child(dot("a", a))
            .child(dot("b", b))
            .child(dot("c", c))
    };
    let ink = if done { p.faint() } else { p.muted() };
    marked_line(p, key, mark, (words, ink))
}

/// A step's `mark` and its words in `ink`.
fn marked_line(p: Paint, key: &str, mark: El, (words, ink): (&str, Color)) -> El {
    row()
        .key(key.to_owned())
        .gap(8.0)
        .center_y()
        .child(mark)
        .child(
            text(words.to_owned())
                .key("words")
                .font(p.k.ty.small)
                .color(ink)
                .wrap_anywhere()
                .shrink(),
        )
}

/// `steps` folded into one line, "Read 4 meetings and 1 file in 9 s ›"
/// ("2 other meetings" when they are `other` than the item asked about),
/// listed under it while `open`; a press calls `toggle`. `web`: the turn
/// whose web search shows too ("Read the guide and 2 web pages in 9 s").
pub(in super::super) fn steps_folded(
    p: Paint,
    (steps, other): (&[Step], bool),
    web: Option<&Turn>,
    seconds: f32,
    open: bool,
    toggle: impl Fn(&mut Ui) + 'static,
) -> Option<El> {
    if steps.is_empty() && web.is_none_or(|turn| turn.web.is_empty()) {
        return None;
    }
    let trace = words::of(steps, &[]);
    let line = summary((&trace, other), web, seconds);
    let mut list = trace_lines(p, &trace);
    list.extend(web.into_iter().flat_map(|turn| super::web::steps(p, turn)));
    Some(fold(p, line, open, toggle, list))
}

/// Each line of `trace`: a check beside what was read, a small cross
/// beside what was not (a no on the card, a call refused).
pub(super) fn trace_lines(p: Paint, trace: &[Did]) -> Vec<El> {
    words::lines(trace)
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let key = format!("step{index}");
            match line.not {
                true => marked_line(
                    p,
                    &key,
                    icon(&rune_theme_opencode::icons::CLOSE, 14.0)
                        .key("mark")
                        .color(p.k.c.icon_muted),
                    (&line.words, p.faint()),
                ),
                false => step_line(p, &key, true, &line.words, None),
            }
        })
        .collect()
}

/// One folded `line` with a chevron, and `list` under it while `open`; a
/// press calls `toggle`.
pub(super) fn fold(
    p: Paint,
    line: String,
    open: bool,
    toggle: impl Fn(&mut Ui) + 'static,
    list: Vec<El>,
) -> El {
    let c = &p.k.c;
    let toggle = row()
        .key("folded")
        .role("button")
        .label(line.as_str())
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .self_start()
        .gap(4.0)
        .center_y()
        .radius(p.k.radius.sm)
        .child(
            text(line.clone())
                .key("words")
                .font(p.k.ty.small)
                .color(p.faint()),
        )
        .child(
            icon(
                if open {
                    &rune_theme_opencode::icons::CHEVRON_DOWN
                } else {
                    &rune_theme_opencode::icons::CHEVRON_RIGHT
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_muted),
        )
        .on_press(toggle);
    let mut shown = col().key("searched").gap(6.0).child(toggle);
    if open {
        shown = shown.child(col().key("steps").gap(6.0).pl(2.0).children(list));
    }
    shown
}

/// The answer, its `[n]` as small numbered marks that open their place,
/// the places in the window it names in bold as links that open them
/// (with `known`, once it is in), and its other bold words at full
/// brightness. `from`: the conversation and turn it answers, once it is in.
pub(super) fn answer(
    p: Paint,
    answer: &str,
    from: Option<(u32, usize)>,
    known: Option<&Known>,
    (muted, streaming): (bool, bool),
) -> El {
    let open = from.map(|from| -> Open {
        Rc::new(move |ui: &mut Ui, n, mark, tx: &mut Tx| {
            super::jump::open(ui, from, n, mark, tx);
        })
    });
    answer_with(p, answer, open, known, (muted, streaming))
}

/// Opens place `n` of one answer: the lines its `mark`-th `[n]` cites, or
/// its first place for a chip.
pub(in super::super) type Open = Rc<dyn Fn(&mut Ui, u32, Option<u32>, &mut Tx)>;

/// An answer, its `[n]` as small numbered marks that `open` their place,
/// and the places in the window it names in bold (with `known`) as links.
pub(in super::super) fn answer_with(
    p: Paint,
    answer: &str,
    open: Option<Open>,
    known: Option<&Known>,
    (muted, streaming): (bool, bool),
) -> El {
    let style = rune_kit::MarkdownStyle::from_kit(p.k)
        .body_font(p.k.ty.ui)
        .text(if muted { p.muted() } else { p.reading() });
    let accent = p.accent();
    let k = p.k;
    let mark = Span {
        size: Some(11.0),
        weight: Some(500.0),
        color: Some(accent),
        bg: Some(k.alpha(accent, 0.12)),
        bg_pad: 3.0,
        ..Span::default()
    };
    let hover = LinkPaint::new().bg(k.alpha(accent, 0.22));
    let ring = k.alpha(accent, 0.5);
    let (source, names) = match known {
        Some(known) => strong::linked(answer, |name| known.shown(name)),
        None => (answer.to_owned(), Vec::new()),
    };
    let source = marked(&source);
    let bright = if muted {
        Vec::new()
    } else {
        strong::bright(&source)
    };
    // A place reads in the link colour, underlined under the pointer.
    let place = Span {
        color: Some(accent),
        ..Span::default()
    };
    let place_hover = LinkPaint::new().underline(accent);
    let names: Rc<[String]> = names.into();
    let named = names.clone();
    rune_kit::Markdown::new(source)
        .style(style)
        .highlights(bright.into_iter().map(|at| {
            (
                at,
                Highlight::new().bg(Color::TRANSPARENT).fg(k.c.text_base),
            )
        }))
        .selectable(!streaming)
        .streaming(streaming)
        .tasks_read_only()
        .link_style(move |url| {
            if url.starts_with(REF) {
                Some((mark.clone(), hover))
            } else {
                url.starts_with(strong::PLACE)
                    .then(|| (place.clone(), place_hover))
            }
        })
        .link_ring(move |url| url.starts_with(REF).then_some((ring, 1.0)))
        .link_help(move |url| {
            let name = named.get(url.strip_prefix(strong::PLACE)?.parse::<usize>().ok()?)?;
            Some(tr_fill("Open {}", name))
        })
        .on_link_tx(move |ui: &mut Ui, url: &str, tx: &mut Tx| {
            if let Some(n) = url.strip_prefix(strong::PLACE) {
                if let Some(name) = n.parse::<usize>().ok().and_then(|n| names.get(n)) {
                    super::places::open_named(ui, name, tx);
                }
                return;
            }
            // A web page it cites opens in the browser.
            if url.starts_with("https://") || url.starts_with("http://") {
                let _ = tx.open_url(url);
                return;
            }
            // `hex-ref:2.1`: the second of the answer's [2].
            let Some((n, mark)) = url.strip_prefix(REF).and_then(|id| id.split_once('.')) else {
                return;
            };
            if let (Some(open), Ok(n), Ok(mark)) = (&open, n.parse(), mark.parse()) {
                open(ui, n, Some(mark), tx);
            }
        })
        .build(k)
        .key("markdown")
}

const REF: &str = "hex-ref:";

/// `[2]` and `[1, 3]` as links the marks are drawn for, each excerpt once
/// where they stand together; the k-th mark of `[2]` links `hex-ref:2.k`.
fn marked(answer: &str) -> String {
    let mut out = String::with_capacity(answer.len());
    let mut marks: Vec<(u32, u32)> = Vec::new();
    let mut from = 0;
    for run in cite::runs(answer) {
        out.push_str(&answer[from..run.at.start]);
        let rest = &answer[run.at.end..];
        // The sentence's own stop comes first, so a mark never sits in it:
        // "маркетингу.[1]".
        let stop = rest
            .char_indices()
            .take_while(|(_, c)| PUNCTUATION.contains(*c))
            .last()
            .map_or(0, |(at, c)| at + c.len_utf8());
        while out.ends_with(' ') {
            out.pop();
        }
        out.push_str(&rest[..stop]);
        for n in run.numbers() {
            let mark = match marks.iter_mut().find(|(known, _)| *known == n) {
                Some((_, count)) => {
                    *count += 1;
                    *count - 1
                }
                None => {
                    marks.push((n, 1));
                    0
                }
            };
            // Kept on the line of what it follows, a little apart.
            if !out.is_empty() && !out.ends_with(char::is_whitespace) {
                out.push_str(GAP);
            }
            out.push_str(&format!("[{n}]({REF}{n}.{mark})"));
        }
        from = run.at.end + stop;
        // The badge's own padding takes up a plain space after it.
        if answer[from..].starts_with(' ') {
            out.push('\u{202f}');
        }
    }
    out.push_str(&answer[from..]);
    out
}

/// Before a mark: two narrow no-break spaces, about 2 pt clear of its badge.
const GAP: &str = "\u{202f}\u{202f}";
/// What a mark moves behind.
const PUNCTUATION: &str = ".,;:!?…";

/// "From your recordings" and the places cited, each opening its place.
pub(super) fn chips(p: Paint, refs: &[Ref], now: u64, from: (u32, usize)) -> Option<El> {
    let open: Open = Rc::new(move |ui: &mut Ui, n, mark, tx: &mut Tx| {
        super::jump::open(ui, from, n, mark, tx);
    });
    chips_with(p, refs, now, tr("From your recordings"), open)
}

/// `label` and the places `refs` cite, each chip calling `open`.
pub(in super::super) fn chips_with(
    p: Paint,
    refs: &[Ref],
    now: u64,
    label: &str,
    open: Open,
) -> Option<El> {
    if refs.is_empty() {
        return None;
    }
    let c = &p.k.c;
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    let mut line = row()
        .key("recordings")
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .child(
            text(label.to_owned())
                .key("label")
                .font(small)
                .color(p.faint()),
        );
    let mut sorted = refs.to_vec();
    sorted.sort_by_key(|cited| cited.n);
    for cited in sorted {
        let glyph = match cited.kind {
            Kind::Meeting => Pane::Meetings.icon(),
            Kind::File => Pane::Files.icon(),
            Kind::Dictation => Pane::History.icon(),
        };
        let title = match (cited.kind, cited.title.trim()) {
            (Kind::Dictation, "") => tr("Dictation").to_owned(),
            (_, title) => title.to_owned(),
        };
        let at = session::when(cited.at_ms, now);
        let label = format!("{} {title}, {at}", cited.n);
        let open = open.clone();
        line = line.child(
            row()
                .key(format!("ref{}", cited.n))
                .role("link")
                .label(label.as_str())
                .focusable(true)
                .focus_ring(2.0, 0.0, c.border_focus)
                .h(22.0)
                .px(7.0)
                .gap(5.0)
                .center_y()
                .radius(p.k.radius.sm)
                .border(p.hair, c.border_weak)
                .hover(c.hover)
                .press(c.pressed)
                .child(icon(glyph, 12.0).key("glyph").color(c.icon_muted))
                .child(
                    text(cited.n.to_string())
                        .key("n")
                        .font(rune::TextStyle {
                            weight: 600.0,
                            ..small
                        })
                        .color(p.accent()),
                )
                .child(
                    text(title)
                        .key("title")
                        .font(small)
                        .color(p.muted())
                        .max_lines(1),
                )
                .child(text(at).key("at").font(small).color(p.faint()).max_lines(1))
                .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| open(ui, cited.n, None, tx)),
        );
    }
    Some(line)
}

/// The folded line: what the steps read, and in how long, and that the
/// web was held back when it was. Nothing a tool read: that a search of
/// the recordings found nothing, else the guide, with the web when the
/// answer went there.
pub(super) fn summary((trace, other): (&[Did], bool), web: Option<&Turn>, seconds: f32) -> String {
    let seconds = format!("{:.0}", seconds.max(1.0));
    let web = web.filter(|turn| !turn.web.is_empty());
    let mut read = words::read(trace, other);
    if !read.is_empty()
        && let Some(turn) = web
        && turn.web.read() > 0
    {
        read.push(words::counted(
            turn.web.read(),
            "1 web page",
            "{} web pages",
        ));
    }
    match (words::folded(&read, &seconds), web) {
        // Kept off the web, as a guide answer's line says it.
        (Some(line), Some(turn)) if turn.web.held_back && !turn.web.went_online() => {
            tr_named("{line}, not the web", &[("line", &line)])
        }
        (Some(line), _) => line,
        (None, _) if words::searched(trace) => {
            tr_fill("Found nothing in your recordings in {} s", &seconds)
        }
        (None, Some(turn)) => super::web::folded(turn, &seconds),
        (None, None) => tr_fill("Read the guide in {} s", &seconds),
    }
}

#[cfg(test)]
mod tests {
    use super::{GAP, marked};

    #[test]
    fn citations_become_links_and_the_rest_stays() {
        let gap = GAP;
        assert_eq!(
            marked("Cut by 15 % [2][3]. See [1, 4]."),
            format!(
                "Cut by 15 %.{gap}[2](hex-ref:2.0){gap}[3](hex-ref:3.0)\u{202f} See.{gap}[1](hex-ref:1.0){gap}[4](hex-ref:4.0)"
            )
        );
        assert_eq!(
            marked("маркетингу [1], и бюджет [2]"),
            format!("маркетингу,{gap}[1](hex-ref:1.0)\u{202f} и бюджет{gap}[2](hex-ref:2.0)")
        );
        // Each of an excerpt's marks opens its own place; one per excerpt
        // where they stand together.
        assert_eq!(
            marked("A [2][2] b [2]."),
            format!("A{gap}[2](hex-ref:2.0)\u{202f} b.{gap}[2](hex-ref:2.1)")
        );
        assert_eq!(
            marked("A [link](https://x) and [b]"),
            "A [link](https://x) and [b]"
        );
        assert_eq!(marked("[3](x) stays"), "[3](x) stays");
    }
}
