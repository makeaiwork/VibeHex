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
use super::changes::At;
use super::composer::ask;
use super::face::{Mood, face};
use crate::assistant::changes::CardKind;
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
pub(super) fn feed(ui: &Ui, p: Paint, greet: bool, still: bool, layer: &mut Vec<El>) -> El {
    let pane = &ui.assistant;
    let mut feed = col().key("assistant-feed").gap(18.0);
    if let Some(error) = &pane.error {
        feed = feed.child(
            text(tr_fill("The question wasn't answered: {}", error))
                .key("error")
                .font(DESCRIPTION)
                .color(p.danger())
                .wrap_anywhere(),
        );
    }
    let Some(conversation) = pane.conversation() else {
        return if greet {
            feed.child(start(ui, p))
        } else {
            feed
        };
    };
    if conversation.turns.is_empty() && pane.consent.is_none() && pane.answering.is_none() {
        return if greet {
            feed.child(start(ui, p))
        } else {
            feed
        };
    }
    let known = super::places::Known::of(ui);
    for (index, turn) in conversation.turns.iter().enumerate() {
        // Setup's own questions and the answers to them have no bubble.
        if !turn.question.trim().is_empty() && turn.answers.is_empty() {
            feed = feed.child(session::bubble(p, &format!("asked{index}"), &turn.question));
        }
        feed = feed.child(reply(ui, p, &known, (conversation.id, index), turn, layer));
    }
    if let Some(answering) = pane.answering_in(pane.open) {
        let mood = pane.face.mood(true, still);
        feed = feed.children(coming(ui, p, answering, mood, pane.dots.phase()));
    } else if let Some(pending) = pane
        .consent
        .as_ref()
        .filter(|pending| Some(pending.conversation) == pane.open)
    {
        feed = feed.children(super::recall::consent(ui, p, pending));
    }
    feed
}

/// A greeting, a way into what changed after an update, setup, and the
/// example questions, two by two.
fn start(ui: &Ui, p: Paint) -> El {
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
                "Whether it can do something, how to set it up, which model to use. When a setting should change, you see the change and decide.",
            ))
            .key("lead")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap_anywhere(),
        )
        .children(super::whats_new::row_for(ui, p).map(|row| row.mt(12.0)))
        .child(setup(p).mt(if ui.assistant.updated.is_some() { 8.0 } else { 12.0 }))
        .children(
            crate::assistant::setup::preset::offered(&ui.settings).then(|| like_denis(p)),
        )
        .child(grid.mt(8.0))
        .child(knows(p))
}

/// "Set VibeHex up for me": setup's questions, at once.
fn setup(p: Paint) -> El {
    let c = &p.k.c;
    let title = tr("Set VibeHex up for me");
    row()
        .key("setup")
        .role("button")
        .label(title)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .gap(12.0)
        .px(14.0)
        .py(12.0)
        .center_y()
        .radius(p.k.radius.md)
        .fill(p.k.alpha(p.accent(), 0.08))
        .border(p.hair, p.k.alpha(p.accent(), 0.4))
        .hover(c.hover)
        .press(c.pressed)
        .child(
            rune::el()
                .key("tile")
                .size(30.0, 30.0)
                .center()
                .radius(p.k.radius.sm)
                .fill(p.k.alpha(p.accent(), 0.14))
                .child(icon(&SLIDERS, 15.0).key("glyph").color(p.accent())),
        )
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(title)
                        .key("title")
                        .font(p.k.ty.body_medium)
                        .color(p.text()),
                )
                .child(
                    text(tr(
                        "A few questions about how you work, then one list of changes to apply",
                    ))
                    .key("lead")
                    .font(p.k.ty.small)
                    .color(p.muted())
                    .wrap_anywhere(),
                ),
        )
        .child(icon(&CHEVRON, 14.0).key("chevron").color(p.accent()))
        .on_press(super::ask::begin)
}

/// A quiet second way in: the owner's own settings. Only shown with
/// Russian recognition (`setup::preset::offered`).
fn like_denis(p: Paint) -> El {
    let c = &p.k.c;
    let title = tr("Or set it up like Denis, who makes VibeHex");
    row()
        .key("like-denis")
        .role("button")
        .label(title)
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .gap(4.0)
        .px(2.0)
        .py(2.0)
        .center_y()
        .radius(p.k.radius.sm)
        .hover(c.hover)
        .child(text(title).key("title").font(p.k.ty.small).color(p.muted()))
        .child(icon(&CHEVRON, 12.0).key("chevron").color(p.faint()))
        .on_press(|ui: &mut Ui| super::preset::begin(ui, String::new()))
}

static SLIDERS: rune::Icon = rune::Icon {
    name: "hex-assistant-setup",
    view: 16.0,
    body: r#"<path d="M2.5 4.5h6M11.5 4.5h2M2.5 11.5h2M7.5 11.5h6" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/><circle cx="10" cy="4.5" r="1.5" fill="none" stroke="currentColor" stroke-width="1.4"/><circle cx="6" cy="11.5" r="1.5" fill="none" stroke="currentColor" stroke-width="1.4"/>"#,
};

static CHEVRON: rune::Icon = rune::Icon {
    name: "hex-assistant-chevron",
    view: 16.0,
    body: r#"<path d="m6 3.5 4.5 4.5L6 12.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

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
            .wrap_anywhere()
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
        .border(p.hair, c.border_base)
        .hover(c.hover)
        .press(c.pressed)
        .child(
            text(question)
                .key("question")
                .font(p.k.ty.ui)
                .color(p.text())
                .wrap_anywhere()
                .shrink(),
        )
        .on_press(move |ui: &mut Ui| {
            ask(ui, question.to_owned());
        })
}

/// A reply beside the face: its words, then what stopped or failed it, the
/// guide articles it names, and its row.
fn reply(
    ui: &Ui,
    p: Paint,
    known: &super::places::Known,
    (conversation, index): (u32, usize),
    turn: &Turn,
    layer: &mut Vec<El>,
) -> El {
    let now_ms = ui.assistant.now_ms();
    let mut words = col().key("words").grow(1.0).shrink().min_w(0.0).gap(8.0);
    words = words.children(super::knew::folded(ui, p, conversation, index, turn));
    if !turn.answer.trim().is_empty() {
        words = words.child(super::recall::answer(
            p,
            &turn.answer,
            Some((conversation, index)),
            Some(known),
            (false, false),
        ));
    }
    words = words.children(super::ask::view(ui, p, conversation, index, turn));
    words = words.children(super::preset::view(ui, p, conversation, index));
    if let Some(error) = &turn.error {
        let again = turn.clone();
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
                        .wrap_anywhere()
                        .shrink(),
                )
                .child(small_button(p, "retry", tr("Try again"), move |ui, _| {
                    retry(ui, conversation, index, &again);
                })),
        );
    }
    words = words.children(super::recall::chips(
        p,
        &turn.refs,
        now_ms,
        (conversation, index),
    ));
    words = words.children(sources(p, &turn.sources, &turn.web.sources));
    // The changes it suggested, then the lines the app refused.
    let newest = super::changes::newest(ui, conversation);
    for card in 0..turn.cards.len() {
        let at = At {
            conversation,
            turn: index,
            card,
        };
        let primary = newest == Some(at);
        words = words.children(match turn.cards[card].kind {
            CardKind::Plan => super::plan::view(ui, p, at, primary, layer),
            CardKind::Vocabulary => super::vocab::view(ui, p, at, primary),
            CardKind::VocabularyEdit => super::vocab_edit::view(ui, p, at, primary),
            _ => super::card::view(ui, p, at, primary, layer),
        });
    }
    for (number, refused) in turn.refused.iter().enumerate() {
        words = words.children(super::card::refused(ui, p, number, refused));
    }
    // The app's own questions have no model, and no row.
    if turn.error.is_none() && !turn.model.is_empty() {
        let again = turn.clone();
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
            move |ui: &mut Ui, _| retry(ui, conversation, index, &again),
        ));
    }
    beside_face(p, &format!("answer{index}"), Mood::default(), words)
}

/// Asks turn `index` again: setup's turns as the setup asks them (its
/// look, its follow-ups, its plan), else its question.
fn retry(ui: &mut Ui, conversation: u32, index: usize, turn: &Turn) {
    if turn.answers.is_empty() && !crate::assistant::setup::flow::is_look(turn) {
        ask(ui, turn.question.clone());
    } else {
        super::ask::again(ui, conversation, index);
    }
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

/// "From the guide" and the articles a reply names, by their titles, then
/// "From the web" and the sites it cites.
fn sources(p: Paint, ids: &[String], web: &[String]) -> Option<El> {
    let c = &p.k.c;
    let titles: Vec<&str> = ids
        .iter()
        .filter_map(|id| guide::article(id))
        .map(|article| article.title.as_str())
        .collect();
    if titles.is_empty() && web.is_empty() {
        return None;
    }
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    let mut line = row().key("sources").gap(6.0).center_y().flex_wrap();
    if !titles.is_empty() {
        line = line.child(
            text(tr("From the guide"))
                .key("label")
                .font(small)
                .color(p.faint()),
        );
    }
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
    Some(line.children(super::web::chips(p, web)))
}

/// The question being answered and what came so far, the face reading.
/// "Looking in the guide" is said only while that is all it does: before
/// its first words, with nothing read past the guide.
fn coming(ui: &Ui, p: Paint, answering: &Answering, mood: Mood, dots: Option<u8>) -> Vec<El> {
    let steps = super::recall::live(p, answering, dots);
    let web = super::web::live(p, answering, dots);
    // It asked to read the recordings and waits for the person.
    let asking = super::recall::asking(ui, p, answering);
    // The model said nothing in time and is asked again.
    let retry = (answering.retrying && answering.draft.trim().is_empty()).then(|| {
        super::recall::step_line(p, "retry", false, tr("No reply yet, asking again"), dots)
    });
    let words = if !answering.draft.trim().is_empty() {
        let mut draft = answering.draft.trim_end().to_owned();
        draft.push('▌');
        Some(super::recall::answer(p, &draft, None, None, (true, true)))
    } else if steps.is_none() && web.is_none() && retry.is_none() {
        // A step like the others: its three dots move while it goes on.
        Some(
            super::recall::step_line(p, "looking", false, tr("Looking in the guide"), dots).mt(2.0),
        )
    } else {
        None
    };
    // A change being read and checked: never its raw lines.
    let preparing = answering
        .cards_coming
        .then(|| super::recall::step_line(p, "preparing", false, tr("Preparing a change"), dots));
    let bubble = (!answering.question.trim().is_empty())
        .then(|| session::bubble(p, "asking", &answering.question));
    bubble
        .into_iter()
        .chain([beside_face(
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
                .children(web)
                .children(asking)
                .children(retry)
                .children(words)
                .children(preparing),
        )])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::language;
    use crate::desktop::i18n::speaking;
    use crate::transcription::models::{TranscriptionModelId, TranscriptionSelection};

    fn dictating(language: &str) -> Ui {
        crate::desktop::i18n::apply(Some("en"));
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.pane = super::super::super::sidebar::Pane::Assistant;
        ui.settings.transcription = TranscriptionSelection {
            model: TranscriptionModelId::ParakeetV3,
            language: language.into(),
        };
        ui
    }

    fn russian(english: &str) -> String {
        speaking(Some("ru"), || tr(english).to_owned())
    }

    /// An English window, Russian dictated: the start page's questions and
    /// setup's own questions, chips, and "Now:" are Russian.
    #[test]
    fn the_dictation_language_speaks_on_the_start_page_and_in_setup() {
        let mut ui = dictating("ru");
        let shown = language::within(&ui.settings, || EXAMPLES.map(tr));
        for (shown, english) in shown.iter().zip(EXAMPLES) {
            assert_eq!(*shown, russian(english));
            assert_ne!(*shown, english, "{english} has a Russian row");
        }
        super::super::ask::begin(&mut ui);
        let turn = &ui.assistant.conversation().unwrap().turns[0];
        assert!(
            turn.now.starts_with(&format!("**{}**", russian("Now:"))),
            "{}",
            turn.now
        );
        let ask = &turn.asks[0];
        let (q, label) = language::within(&ui.settings, || {
            (tr(&ask.q).to_owned(), ask.label(&ask.options[0]))
        });
        assert_eq!(q, russian(&ask.q));
        assert_ne!(q, ask.q);
        assert_eq!(label, russian(&ask.options[0].label));
        // The window stays English.
        assert_eq!(
            tr("Ask anything about VibeHex"),
            "Ask anything about VibeHex"
        );
    }

    /// A setup round's own controls (Continue, Skip to the plan, Change
    /// answers, a plan's buttons) and a returning setup's own lines speak
    /// every language Assistant speaks.
    #[test]
    fn setup_controls_speak_every_language() {
        const CONTROLS: &[&str] = &[
            "Continue",
            "Skip to the plan",
            "Change answers",
            "Setup questions",
            "Suggested changes",
            "Not now",
            "Apply 1 change",
            "Already set: 1 thing",
            "Also turns on",
            "Needs a download",
            "Undo",
            "Try again",
            "Left as it was",
            // A returning setup's opener, its step, its plan's term rows,
            // and the follow-ups kept from before.
            "What your use shows:",
            "Ask “What's new?” to hear more",
            "Read {}",
            "your settings and {}",
            "your settings, {dictations} and {meetings}",
            "Cleanup keeps fixing how these are heard",
            "You say these, and Vocabulary doesn't have them",
            "Answered before: {}",
            // A switch's companion in its row.
            "after {}",
            "remind me {}",
            "light {}",
            "voice {}",
            "At start",
        ];
        for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
            let english: Vec<&str> = CONTROLS
                .iter()
                .copied()
                .filter(|control| speaking(Some(code), || tr(control)) == *control)
                .collect();
            assert!(english.is_empty(), "{code} shows {english:?} in English");
        }
    }

    /// Before an answer's words come, its line is a step going on: the
    /// dots' clock counts it ("Looking in the guide", and "Preparing a
    /// change" beside a draft), and the words carry no typed ellipsis in
    /// any language, the dots say it.
    #[test]
    fn the_lines_before_an_answer_are_steps_going_on() {
        let mut ui = dictating("auto");
        super::super::fixture::answering(&mut ui);
        let live = |ui: &Ui| super::super::steps_live(ui);
        fn answering(ui: &mut Ui) -> &mut Answering {
            ui.assistant.answering.as_mut().expect("answering")
        }
        // Words alone: nothing goes on.
        assert!(!live(&ui));
        answering(&mut ui).cards_coming = true;
        assert!(live(&ui), "Preparing a change");
        answering(&mut ui).cards_coming = false;
        answering(&mut ui).draft.clear();
        assert!(live(&ui), "Looking in the guide");

        for words in ["Looking in the guide", "Preparing a change"] {
            for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
                let shown = speaking(Some(code), || tr(words).to_owned());
                assert_ne!(shown, words, "{code} has no row for {words}");
                assert!(
                    !shown.ends_with('…') && !shown.ends_with("..."),
                    "{code}: {shown}"
                );
            }
        }
    }

    /// What the window shows of `ui`, as its texts.
    fn shown(apply: impl FnOnce(&mut Ui)) -> Vec<String> {
        crate::desktop::i18n::apply(Some("en"));
        let rt = super::super::layout::window(apply, super::super::super::WINDOW_SIZE);
        rt.elements().into_iter().filter_map(|e| e.text).collect()
    }

    /// An agent's answer on its way says only what goes on: it looks in
    /// the guide while the articles laid out are all it has; past them it
    /// lists what it read and thinks; and while it asks to read the
    /// recordings the card stands there and nothing else moves.
    #[test]
    fn an_agents_answer_says_only_what_it_does() {
        fn answering(ui: &mut Ui) -> &mut Answering {
            ui.assistant.answering.as_mut().expect("answering")
        }
        let has = |texts: &[String], words: &str| texts.iter().any(|text| text.contains(words));

        // Only the articles the app laid out: it is in the guide.
        let first = shown(|ui| {
            super::super::fixture::steps(ui);
            answering(ui).trace.truncate(1);
        });
        assert!(has(&first, "Looking in the guide"), "{first:?}");
        assert!(!has(&first, "guide articles") && !has(&first, "Thinking it over"));

        // Past them: each thing read, then that it thinks.
        let read = shown(super::super::fixture::steps);
        for line in [
            "Read 3 guide articles: ",
            "Searched 12 meetings: 4 match",
            "Read the notes of “Бюджет Q1”",
            "Thinking it over",
        ] {
            assert!(has(&read, line), "{line}: {read:?}");
        }
        assert!(!has(&read, "Looking in the guide"));
        assert!(!has(&read, "Searching your recordings"));

        // Its words come: the steps stay, nothing goes on beside them.
        let drafted = shown(|ui| {
            super::super::fixture::steps(ui);
            answering(ui).draft = "In March".into();
        });
        assert!(has(&drafted, "Searched 12 meetings: 4 match"));
        assert!(!has(&drafted, "Thinking it over"));

        // It asks: the card, under what it read, and no step going on.
        let asking = shown(super::super::fixture::agent_consent);
        assert!(has(&asking, "Read your meetings, files, and dictations?"));
        assert!(has(&asking, "Answer without recordings"));
        assert!(has(&asking, "Read 3 guide articles: "), "{asking:?}");
        assert!(!has(&asking, "Thinking it over") && !has(&asking, "Looking in the guide"));
        let mut ui = dictating("auto");
        super::super::fixture::agent_consent(&mut ui);
        assert!(
            !super::super::steps_live(&ui),
            "nothing moves while it asks"
        );
    }

    /// A finished agent answer's fold counts what its tools read; the
    /// articles the app laid out are listed under it, never counted, and
    /// a call that was refused has no check.
    #[test]
    fn an_agents_fold_counts_what_its_tools_read() {
        let has = |texts: &[String], words: &str| texts.iter().any(|text| text.contains(words));
        let guide = shown(super::super::fixture::knew);
        // The scene dictates Russian: its answer's lines speak it.
        let russian = |english: &str| speaking(Some("ru"), || tr(english).to_owned());
        assert!(
            has(
                &guide,
                &russian("Read the guide in {} s").replace("{}", "7")
            ),
            "{guide:?}"
        );
        let read = shown(super::super::fixture::steps_open);
        // Web was on until the recordings were read: the fold says it
        // stayed off, as a guide answer's does, and the last step why.
        assert!(
            has(
                &read,
                "Read 4 meetings, 1 file, and your settings in 9 s, not the web"
            ),
            "{read:?}"
        );
        assert!(has(
            &read,
            "The web stays off once a conversation has read your recordings, Vocabulary, or cleanup instructions"
        ));
        assert!(has(&read, "Read 3 guide articles: "), "{read:?}");
        assert!(has(&read, "You allowed reading your recordings"));
        assert!(has(&read, "Didn't read your settings: sending them is off"));
    }

    /// A fold that read things and was kept off the web says so in every
    /// language, and one Web was never on for says nothing of it.
    #[test]
    fn a_fold_kept_off_the_web_says_so() {
        use crate::assistant::agent::trace::Did;
        let trace = [Did::Settings {
            keys: vec!["labs.meetings".into()],
        }];
        let mut turn = Turn {
            trace: trace.to_vec(),
            ..Turn::default()
        };
        let line = |turn: &Turn| super::super::recall::summary((&trace, false), Some(turn), 9.0);
        assert_eq!(line(&turn), "Read your settings in 9 s");
        turn.web.held_back = true;
        assert_eq!(line(&turn), "Read your settings in 9 s, not the web");
        assert!(!line(&turn).contains('·'));
        for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
            let shown = speaking(Some(code), || line(&turn));
            let guide = speaking(Some(code), || line(&Turn::default()));
            assert!(
                shown.starts_with(&guide) && shown != guide,
                "{code}: {shown}"
            );
        }
        assert_eq!(
            speaking(Some("ru"), || line(&turn)),
            format!(
                "{}, без интернета",
                speaking(Some("ru"), || line(&Turn::default()))
            )
        );
    }

    /// While an answer in another conversation waits on the consent card,
    /// a question here says what it waits for, with a way there, and the
    /// card stays in its own conversation; while one only answers there,
    /// that it answers. In its own conversation neither shows.
    #[test]
    fn a_question_here_says_what_it_waits_for_elsewhere() {
        use super::super::composer::PARKED;
        use crate::assistant::setup::flow::BUSY;
        let has = |texts: &[String], words: &str| texts.iter().any(|text| text.contains(words));
        let elsewhere = |ui: &mut Ui| {
            super::super::fixture::agent_consent(ui);
            ui.assistant.open = None;
        };
        let parked = shown(elsewhere);
        assert!(has(&parked, PARKED), "{parked:?}");
        assert!(has(&parked, "Open that conversation"));
        assert!(!has(&parked, BUSY));
        assert!(!has(&parked, "Read your meetings, files, and dictations?"));
        let busy = shown(|ui| {
            elsewhere(ui);
            ui.assistant.answering.as_mut().expect("answering").asking = false;
        });
        assert!(has(&busy, BUSY) && !has(&busy, PARKED), "{busy:?}");
        assert!(!has(&busy, "Open that conversation"));
        let here = shown(super::super::fixture::agent_consent);
        assert!(!has(&here, PARKED) && !has(&here, BUSY));

        let mut ui = dictating("auto");
        super::super::fixture::agent_consent(&mut ui);
        let id = ui.assistant.open;
        ui.assistant.open = None;
        let (_, open) = super::super::composer::elsewhere(&ui).expect("waits");
        assert_eq!(open, id, "the line opens the card's conversation");
        for words in [PARKED, "Open that conversation"] {
            for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
                let shown = speaking(Some(code), || tr(words).to_owned());
                assert_ne!(shown, words, "{code} has no row for {words}");
            }
        }
        assert_eq!(
            speaking(Some("ru"), || tr(PARKED).to_owned()),
            "Ответ в другом разговоре ждёт, разрешите ли вы ему читать ваши записи."
        );
    }

    /// Recognition on Auto follows the window; a language without a table
    /// (Italian) shows English.
    #[test]
    fn auto_follows_the_window_and_italian_reads_english() {
        for language in ["auto", "it"] {
            let ui = dictating(language);
            let shown = language::within(&ui.settings, || EXAMPLES.map(tr));
            assert_eq!(shown, EXAMPLES, "{language}");
        }
        let ui = dictating("auto");
        crate::desktop::i18n::apply(Some("de"));
        let shown = language::within(&ui.settings, || tr(EXAMPLES[0]).to_owned());
        assert_eq!(shown, speaking(Some("de"), || tr(EXAMPLES[0]).to_owned()));
        assert_ne!(shown, EXAMPLES[0]);
        crate::desktop::i18n::apply(Some("en"));
    }
}
