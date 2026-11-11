//! What a model reads for one question: Assistant's instructions, the
//! guide's table of contents, the articles search found, the settings a
//! card may change (with their values while the person lets it read them,
//! `changes::catalog`), the conversation so far, and the question. Nothing
//! else: no transcripts, history, meetings, or keys. The model ends its
//! reply with a `Sources:` line, which the pane turns into "From the guide"
//! chips and never shows, and may write `hex-change` lines before it, which
//! the pane turns into cards.

use super::changes::{self, Refused, catalog};
use super::guide::{self, Article};
use super::recordings::{Excerpt, Found, Kind, cite};
use super::retrieve;
use super::setup::{self, Stage};

/// How many articles a model reads: a small local one has a short window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Room {
    Small,
    Large,
}

impl Room {
    fn articles(self) -> usize {
        match self {
            Self::Small => 3,
            Self::Large => 6,
        }
    }

    /// How much of each earlier answer goes back in.
    fn earlier_answer_chars(self) -> usize {
        match self {
            Self::Small => 600,
            Self::Large => 4_000,
        }
    }

    /// How many earlier turns go back in, newest kept.
    fn earlier_turns(self) -> usize {
        match self {
            Self::Small => 2,
            Self::Large => 8,
        }
    }
}

/// One earlier question and its answer.
#[derive(Clone, Copy, Debug)]
pub struct Turn<'a> {
    pub question: &'a str,
    pub answer: &'a str,
    /// What became of the changes it suggested, one line each.
    pub changes: &'a str,
}

/// A question, ready for an engine.
#[derive(Debug)]
pub struct Prompt {
    pub instructions: String,
    pub input: String,
    /// The articles it carries, best first. Read by tests for now; a later
    /// stage shows them as the steps an answer took.
    #[cfg_attr(not(test), allow(dead_code))]
    pub articles: Vec<&'static str>,
}

const INSTRUCTIONS: &str = "You are Assistant, the help built into VibeHex, a dictation app \
for macOS, Windows, and Linux. Answer the person's question about VibeHex from the guide \
excerpts you are given.

- Reply in the language of the person's latest question, whatever the language of the guide \
or of the app. If that question has no clear language, keep the conversation's.
- Name panes, buttons, and settings exactly as the guide writes them, in English, with › \
between levels (Labs › Meetings), so they can be found on screen.
- Answer first, in a sentence or two, then the steps as a short numbered list when there \
are steps. No preamble.
- Say which systems a feature works on when it matters, and say plainly when VibeHex cannot \
do something.
- Use only the excerpts. When they don't answer the question, say the guide doesn't cover \
it. Never invent settings, keys, buttons, or menus.
- You never change settings yourself; say where the person changes them.
- For questions that are not about VibeHex, say in one sentence that you help with VibeHex.
- End with one last line: `Sources:` and the ids of the articles you used, comma-separated \
(for example `Sources: labs/meetings, topics/codex`), or `Sources: none`.";

const CHANGES: &str = "

You may also suggest setting changes. The app turns each one into a card the person presses; \
nothing changes until they do. When a setting in <changeable_settings> would do what the \
person wants and isn't set that way now, add after your answer, before the Sources line:

```hex-change
{\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"a short reason\"}
```

- One JSON object per line, one card each, at most 3. Keys exactly as listed, values as the \
list writes them (true/false for on/off).
- `why`: one short phrase, in the language of the question.
- To let the person pick, write `options` (2 to 4 values) and `suggest` (the one you \
recommend) instead of `value`.
- Only what this question is about. None when nothing should change, or when the now column \
already has the value.
- Name only the setting asked about: the app also turns on what it needs (a Labs feature, \
Meetings).
- Never a key that isn't listed. Never say you changed anything: the person decides on the card.

To add names or terms dictation gets wrong to the person's Vocabulary, write one JSON object \
per term in one block, after your answer, before the Sources line:

```hex-vocab
{\"term\":\"Kubernetes\",\"sounds_like\":[\"кубернетис\",\"кубер\"],\"why\":\"a short reason\"}
```

- `term` spelled as it should be written; `sounds_like`: up to 5 ways a transcript gets it \
wrong, in the language they speak. At most 8 terms, only ones the question is about.
- You never see their vocabulary: the app leaves out a term it has.";

/// Guided setup, a typed request: round one's four questions, in the
/// message's language, as option chips the app preselects.
fn setup_ask(os: &str) -> String {
    format!(
        "

The person asks you to set VibeHex up for them. Don't answer from the guide yet, and \
suggest no changes. In one sentence, in the language of their message, say you'll ask a \
few short questions, with their current settings already picked, and then offer one list \
of changes they decide on. Then write this block, with `q`, `hint`, every `label`, and \
every `note` (one short line under an option) in the language of their message, and every \
`id` exactly as written:

```hex-ask
{}
```

End with `Sources: none`.",
        setup::ask_lines(os)
    )
}

/// Guided setup after round one: the follow-up questions the app picked,
/// for a remote model to reword, drop, or add to.
fn setup_follow(candidates: &str, others: &str) -> String {
    format!(
        "

<answers> holds the person's answers to the first setup questions. The app picked the \
follow-up questions below, each about one Labs feature or setting, best first; each option's \
id is what the app changes on it (yes turns the feature on, no leaves it).

```hex-ask
{candidates}
```

- Write one short sentence, in the language of the setup request or, without one, of the \
words in <answers>, that a few questions about features that fit how they work follow.
- Then one ```hex-ask block, one JSON object per line, the most useful first, at most 12: \
the questions above that fit these answers, with `q` and every `label` in that language \
and every `id` exactly as written. Leave out one that doesn't fit them.
- You may add a question about a Labs feature from <labs_catalog> (`key | title | \
question`) that clearly fits, as {{\"id\":\"<key>\",\"q\":\"...\",\"options\":[{{\"id\":\"yes\",\"label\":\"...\"}},{{\"id\":\"no\",\"label\":\"...\"}}]}}, or about a \
setting from <changeable_settings> with each option's `value` as the list writes it (an \
option without one changes nothing).
- Questions only: no changes, no plan. End with `Sources: none`.

<labs_catalog>
{others}
</labs_catalog>"
    )
}

const SETUP_PLAN: &str = "

<answers> holds the person's answers to the setup questions: each question's id, then the \
ids of the options they picked, with the words they saw. A follow-up's id is the setting it \
asks about; `yes` means they want it. The app already builds the list from these answers: \
the speech model, where dictation and notes run, Meetings, Files, Russian vibe coding, and \
every follow-up they said yes to. Your part is a sentence and what it may have missed.

- First one or two sentences, in the language of the setup request or, without one, of the \
words in <answers>: what the list is for, and that they can untick what they don't need.
- Then, only if something else clearly fits how they work, one block, one JSON object per \
line, at most 6:

```hex-plan
{\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"a short reason\"}
```

- Keys exactly as listed, values as the list writes them (true/false for on/off). Skip what \
the now column already has, and anything they answered no to.
- `\"optional\":true` on a line that is only nice to have: it starts unchecked.
- Never change the speech model, where cleanup or notes run, or a value they set \
themselves: the app decides those from their answers. Dictation stays fast: never move it \
to a slower engine than they picked.
- `why`: one short phrase each, in that language. You may also write a line the app's list \
has, with the same value, to give its reason in that language. Never say you changed \
anything.";

const SETUP_SINCE: &str = "

They set VibeHex up before, or have used it a while: <since_last_setup> holds what the app \
knows of it. Their answers and what that setup applied and declined; what is new in VibeHex \
since; and what their own use shows, worked out by the app as counts, shares, and single \
words (never their text), or that it wasn't read. Let it decide what fits: never offer again \
what they declined then, prefer what is new and fits, and name a fact from it in a few words \
at most.";

/// What a question may read besides the guide.
#[derive(Clone, Copy, Debug)]
pub enum Reach<'a> {
    /// The guide alone.
    Guide,
    /// The question is about the person's recordings, which Assistant may
    /// not read: the answer says so and where to allow it.
    Declined,
    /// What a search of the person's recordings found, with their `now`.
    Recordings(&'a Found, u64),
}

const RECORDINGS: &str = "

The person also asked about their own recordings. The <recordings> block holds numbered \
excerpts from their meetings (notes and passages, Me is the person, Them the others), their \
Files texts, and their dictations, found on their computer for this question. A transcript \
excerpt has one numbered line per phrase: [2.3] is line 3 of excerpt 2.
- Answer from the excerpts. Bring several together: sum up, say how they connect (the same \
topic coming back, a decision that changed, one thing leading to another), and name when.
- Cite every fact from them right after it, in square brackets: the lines that say it, like \
[2.3], or a run of lines, [2.3-2.5]; an excerpt without numbered lines by its number, [1]. \
Cite the fewest lines that say it, only ids in the block. No Sources line entry for them.
- When the excerpts don't hold the answer, say so plainly. Never guess what was said.
- Times are the person's local time; the block says when now is.";

const DECLINED: &str = "

This question seems to be about the person's own meetings, files, or dictations, which you \
cannot read: they haven't allowed it. Say so in one sentence, and that Assistant › Settings › \
Your meetings, files, and dictations allows it; then answer what the guide covers, if anything.";

/// The prompt for `question` after `earlier` turns, oldest first. With
/// `changes`, the model may suggest setting changes from the catalog; a
/// setup `stage` asks for its questions or its plan instead.
pub fn build(
    question: &str,
    earlier: &[Turn],
    room: Room,
    reach: Reach,
    changes: Option<&changes::Context>,
    stage: &Stage,
) -> Prompt {
    // A follow-up ("and on Windows?") is about what the last question was;
    // setup's answers, about what they picked.
    let query = match (stage, earlier.last()) {
        (Stage::Plan { answers, .. } | Stage::Follow { answers, .. }, _) => setup::query(answers),
        (_, Some(last)) => format!("{} {}", last.question, question),
        (_, None) => question.to_owned(),
    };
    let found = retrieve::search(&query, room.articles());
    let mut input = contents();
    for article in &found {
        input.push_str(&excerpt(article));
    }
    let mut instructions = INSTRUCTIONS.to_owned();
    if *stage == Stage::Ask {
        let os = changes.map_or(std::env::consts::OS, |context| context.state.os);
        instructions.push_str(&setup_ask(os));
    } else if let Some(context) = changes {
        let plan = matches!(stage, Stage::Plan { .. } | Stage::Follow { .. });
        match stage {
            Stage::Follow { candidates, .. } => {
                let (asks, others) = setup::follow::prompt_lines(candidates, &context.settings);
                instructions.push_str(&setup_follow(&asks, &others));
            }
            Stage::Plan { .. } => instructions.push_str(SETUP_PLAN),
            _ => instructions.push_str(CHANGES),
        }
        if let Stage::Plan { since, .. } | Stage::Follow { since, .. } = stage
            && !since.trim().is_empty()
        {
            instructions.push_str(SETUP_SINCE);
        }
        let reading = catalog::Reading {
            // A plan reads every key, whatever the room.
            every: plan || room == Room::Large,
            keys: found
                .iter()
                .flat_map(|article| article.keys.iter().map(String::as_str))
                .collect(),
            send: context.send,
            instructions: found
                .iter()
                .any(|article| article.id == "topics/output-and-instructions"),
        };
        input.push('\n');
        input.push_str(&catalog::section(
            &context.settings,
            &reading,
            &context.state,
        ));
    }
    match reach {
        Reach::Guide => {}
        Reach::Declined => instructions.push_str(DECLINED),
        Reach::Recordings(recordings, now_ms) => {
            instructions.push_str(RECORDINGS);
            input.push_str(&recordings_block(recordings, now_ms));
        }
    }
    let kept = earlier.len().saturating_sub(room.earlier_turns());
    for turn in &earlier[kept..] {
        let changes = match turn.changes.trim() {
            "" => String::new(),
            changes => format!("\n<changes>\n{changes}\n</changes>"),
        };
        input.push_str(&format!(
            "\n<earlier_question>\n{}\n</earlier_question>\n<earlier_answer>\n{}{changes}\n</earlier_answer>\n",
            turn.question.trim(),
            shortened(turn.answer.trim(), room.earlier_answer_chars()),
        ));
    }
    match stage {
        Stage::Plan { line, since, .. } | Stage::Follow { line, since, .. } => {
            if !since.trim().is_empty() {
                input.push_str(&format!(
                    "\n<since_last_setup>\n{}\n</since_last_setup>\n",
                    since.trim()
                ));
            }
            input.push_str(&format!("\n<answers>\n{}\n</answers>\n", line.trim()));
        }
        _ => input.push_str(&format!("\n<question>\n{}\n</question>\n", question.trim())),
    }
    Prompt {
        instructions,
        input,
        articles: found.iter().map(|article| article.id).collect(),
    }
}

const REPAIR: &str = "

Your reply above had change lines the app refused (in <refused>, each with why). Write them \
again, fixed: only keys from <changeable_settings>, values as it writes them. Reply with \
only one ```hex-change block (vocabulary terms in one ```hex-vocab block), and leave out a \
line you can't fix. An empty block if none fit.";

/// One more ask, for a remote model whose change lines were refused: its
/// reply, what was refused and why, and to write those lines again.
pub fn repair(prompt: &Prompt, reply: &str, refused: &[Refused]) -> Prompt {
    let mut input = prompt.input.clone();
    input.push_str(&format!(
        "\n<your_reply>\n{}\n</your_reply>\n<refused>\n",
        reply.trim()
    ));
    for refused in refused {
        input.push_str(&format!("{} : {}\n", refused.line, refused.reason()));
    }
    input.push_str("</refused>\n");
    Prompt {
        instructions: format!("{}{REPAIR}", prompt.instructions),
        input,
        articles: prompt.articles.clone(),
    }
}

/// What became of an earlier turn's changes, for the next prompt: each
/// card's key and value with what the person did, and each refusal.
pub fn changes_line(cards: &[changes::Card], refused: &[Refused]) -> String {
    let mut lines = Vec::new();
    for card in cards {
        let done = match &card.decision {
            changes::Decision::Proposed => "not decided yet",
            changes::Decision::Declined => "declined",
            changes::Decision::Undone => "applied, then undone",
            changes::Decision::Failed { .. } => "failed to apply",
            changes::Decision::Applied { .. } => "applied",
        };
        for item in &card.items {
            let done = match (&card.decision, item.included) {
                (changes::Decision::Applied { .. }, false) => "left out",
                _ => done,
            };
            lines.push(format!("{} = {}: {done}", item.key, item.to));
        }
    }
    for refused in refused {
        lines.push(format!("refused: {} ({})", refused.line, refused.reason()));
    }
    lines.join("\n")
}

/// The guide's table of contents as every prompt carries it: one line per
/// article, its id, title, and systems.
pub(super) fn contents() -> String {
    let mut contents = String::from("<guide_contents>\n");
    for article in guide::articles() {
        contents.push_str(&format!(
            "{}: {} ({})\n",
            article.id, article.title, article.os
        ));
    }
    contents.push_str("</guide_contents>\n");
    contents
}

fn excerpt(article: &Article) -> String {
    format!(
        "\n<guide_article id=\"{}\">\n{}\n</guide_article>\n",
        article.id, article.body
    )
}

/// The excerpts as the model reads them: `[n]`, what and when, the text.
fn recordings_block(found: &Found, now_ms: u64) -> String {
    let mut block = format!("\n<recordings now=\"{}\">\n", stamp(now_ms));
    if found.excerpts.is_empty() {
        block.push_str("Nothing in the recordings matches this question.\n");
    }
    for excerpt in &found.excerpts {
        block.push_str(&format!("[{}] {}\n", excerpt.n, heading(excerpt)));
        if excerpt.lines.is_empty() {
            block.push_str(&excerpt.text);
            block.push('\n');
        }
        for (index, line) in excerpt.lines.iter().enumerate() {
            block.push_str(&format!("[{}.{}] {}\n", excerpt.n, index + 1, line.text));
        }
        block.push('\n');
    }
    block.push_str("</recordings>\n");
    block
}

fn heading(excerpt: &Excerpt) -> String {
    let what = match excerpt.kind {
        Kind::Meeting => format!("Meeting \"{}\"", excerpt.title),
        Kind::File => format!("File \"{}\"", excerpt.title),
        Kind::Dictation if excerpt.title.is_empty() => "Dictation".to_owned(),
        Kind::Dictation => format!("Dictation in {}", excerpt.title),
    };
    let label = match excerpt.label.as_str() {
        "" => String::new(),
        label => format!(", {label}"),
    };
    format!("{what}, {}{label}", stamp(excerpt.at_ms))
}

/// `2026-11-04 10:12`, local.
fn stamp(ms: u64) -> String {
    let iso = crate::meeting::query::iso_local(ms);
    iso.get(..16).unwrap_or(&iso).replace('T', " ")
}

fn shortened(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

/// A reply split into what the pane shows and the guide articles it names
/// on its `Sources:` line, only ids the guide has, in order.
pub fn split_sources(reply: &str) -> (&str, Vec<&'static str>) {
    let trimmed = reply.trim_end();
    let (body, last) = trimmed.rsplit_once('\n').unwrap_or(("", trimmed));
    let line = last.trim().trim_matches(['*', '`', '_']).trim();
    let Some(list) = line
        .get(..8)
        .filter(|head| head.eq_ignore_ascii_case("sources:"))
        .map(|_| &line[8..])
    else {
        return (trimmed, Vec::new());
    };
    let mut ids = Vec::new();
    for id in list
        .split([',', ';', ' '])
        .map(|id| id.trim_matches(['`', '.', '*']))
    {
        if let Some(article) = guide::article(id)
            && !ids.contains(&article.id)
        {
            ids.push(article.id);
        }
    }
    (body.trim_end(), ids)
}

/// While a reply streams, the part to show: never a half-written
/// `Sources:` line, citation, or change block, and lines cited as their
/// excerpt's `[n]`.
pub fn draft(reply: &str) -> String {
    let (visible, _) = changes::parse::visible(reply);
    cite::plain(streamed(visible))
}

/// Whether a change block has started in a streaming reply.
pub fn cards_coming(reply: &str) -> bool {
    changes::parse::visible(reply).1
}

fn streamed(reply: &str) -> &str {
    let reply = without_open_citation(reply);
    let (body, last) = reply.rsplit_once('\n').unwrap_or(("", reply));
    let head: String = last
        .trim_start_matches(['*', '`', '_', ' '])
        .chars()
        .take(8)
        .collect::<String>()
        .to_lowercase();
    if !head.is_empty() && ("sources:".starts_with(&head) || head.starts_with("sources:")) {
        body.trim_end()
    } else {
        reply
    }
}

/// `reply` without a `[1` or `[2.3-` still being written at its end.
fn without_open_citation(reply: &str) -> &str {
    match reply.rfind('[') {
        Some(open) if cite::unfinished(&reply[open + 1..]) => &reply[..open],
        _ => reply,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_carries_the_contents_and_the_articles_search_found() {
        let prompt = build(
            "Can VibeHex record my Zoom calls?",
            &[],
            Room::Large,
            Reach::Guide,
            None,
            &Stage::None,
        );
        assert!(prompt.input.contains("<guide_contents>"));
        assert!(prompt.input.contains("topics/privacy: Privacy"));
        assert!(prompt.articles.len() <= 6);
        assert!(prompt.articles.iter().any(|id| id.contains("meetings")));
        for id in &prompt.articles {
            assert!(
                prompt
                    .input
                    .contains(&format!("<guide_article id=\"{id}\">"))
            );
        }
        assert!(
            prompt
                .input
                .ends_with("<question>\nCan VibeHex record my Zoom calls?\n</question>\n")
        );
        assert!(
            prompt
                .instructions
                .contains("language of the person's latest question")
        );
    }

    #[test]
    fn a_small_model_reads_three_articles_and_short_earlier_answers() {
        let long = "x".repeat(5_000);
        let earlier = [Turn {
            question: "first",
            answer: &long,
            changes: "",
        }];
        let prompt = build(
            "и на Windows?",
            &earlier,
            Room::Small,
            Reach::Guide,
            None,
            &Stage::None,
        );
        assert!(prompt.articles.len() <= 3);
        assert!(prompt.input.contains(&format!("{}…", "x".repeat(600))));
        assert!(!prompt.input.contains(&"x".repeat(601)));
    }

    #[test]
    fn only_the_newest_earlier_turns_go_back_in() {
        let earlier: Vec<Turn> = (0..10)
            .map(|_| Turn {
                question: "q",
                answer: "a",
                changes: "",
            })
            .collect();
        let prompt = build(
            "next",
            &earlier,
            Room::Large,
            Reach::Guide,
            None,
            &Stage::None,
        );
        assert_eq!(prompt.input.matches("<earlier_question>").count(), 8);
    }

    /// The prompt is the guide, the session, and the question: nothing a
    /// person said elsewhere in the app.
    #[test]
    fn the_prompt_holds_only_the_guide_the_session_and_the_question() {
        let earlier = [Turn {
            question: "earlier question",
            answer: "earlier answer",
            changes: "",
        }];
        let prompt = build(
            "new question",
            &earlier,
            Room::Large,
            Reach::Guide,
            None,
            &Stage::None,
        );
        let mut rest = prompt.input.clone();
        let contents_end = rest.find("</guide_contents>\n").unwrap() + "</guide_contents>\n".len();
        rest.replace_range(..contents_end, "");
        for id in &prompt.articles {
            rest = rest.replace(&excerpt(guide::article(id).unwrap()), "");
        }
        for text in ["earlier question", "earlier answer", "new question"] {
            rest = rest.replacen(text, "", 1);
        }
        for tag in ["earlier_question", "earlier_answer", "question"] {
            rest = rest
                .replace(&format!("<{tag}>"), "")
                .replace(&format!("</{tag}>"), "");
        }
        assert_eq!(rest.trim(), "");
    }

    /// Only `assistant::recordings` reads what a person said or keeps:
    /// nothing else in `assistant` or in its pane names it.
    #[test]
    fn assistant_code_names_no_personal_data() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let forbidden = [
            "crate::history",
            "meeting::requests",
            "meeting::transcript",
            "files::store",
            "keys::get",
        ];
        let mut folders = vec![
            root.join("src/assistant"),
            root.join("src/rune_ui/assistant"),
        ];
        let mut checked = 0;
        while let Some(folder) = folders.pop() {
            for entry in std::fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if !path.ends_with("src/assistant/recordings") {
                        folders.push(path);
                    }
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                let code = text.split("#[cfg(test)]").next().unwrap();
                for name in forbidden {
                    assert!(!code.contains(name), "{} names {name}", path.display());
                }
                checked += 1;
            }
        }
        assert!(checked > 10, "only {checked} files checked");
    }

    #[test]
    fn the_sources_line_becomes_known_ids_and_disappears() {
        let reply =
            "Turn on Labs › Meetings.\n\nSources: labs/meetings, topics/nope, `topics/meetings`";
        let (body, ids) = split_sources(reply);
        assert_eq!(body, "Turn on Labs › Meetings.");
        assert_eq!(ids, ["labs/meetings", "topics/meetings"]);
        assert_eq!(split_sources("No line here").1, Vec::<&str>::new());
        assert_eq!(split_sources("A.\n**Sources:** none"), ("A.", vec![]));
    }

    #[test]
    fn a_half_written_citation_never_shows() {
        assert_eq!(draft("Cut by 15 % [2"), "Cut by 15 % ");
        assert_eq!(draft("Cut [2][3"), "Cut [2]");
        assert_eq!(draft("Cut [2]"), "Cut [2]");
        assert_eq!(draft("A [link"), "A [link");
        // Lines never show as ids, whole or half written.
        for partial in ["[2", "[2.", "[2.3", "[2.3-", "[2.3-2.", "[2.3, 4"] {
            assert_eq!(draft(&format!("Cut {partial}")), "Cut ", "{partial}");
        }
        assert_eq!(draft("Cut [2.3-2.5][4.1"), "Cut [2]");
        assert_eq!(draft("Cut [2.3-2.5][4.1]."), "Cut [2][4].");
    }

    #[test]
    fn a_transcript_excerpt_numbers_its_lines() {
        use crate::assistant::recordings::{Excerpt, Line};
        let line = |text: &str| Line {
            text: text.into(),
            start_ms: 0,
            end_ms: 0,
            at: 0..0,
        };
        let found = Found {
            excerpts: vec![
                Excerpt {
                    n: 1,
                    kind: Kind::Meeting,
                    id: "m".into(),
                    title: "Budget".into(),
                    at_ms: 0,
                    offset_ms: None,
                    label: "Notes".into(),
                    text: "Approved.".into(),
                    lines: Vec::new(),
                },
                Excerpt {
                    n: 2,
                    kind: Kind::Meeting,
                    id: "m".into(),
                    title: "Budget".into(),
                    at_ms: 0,
                    offset_ms: Some(0),
                    label: "Them".into(),
                    text: "So. The limit holds.".into(),
                    lines: vec![line("So."), line("The limit holds.")],
                },
            ],
            steps: Vec::new(),
        };
        let block = recordings_block(&found, 0);
        assert!(block.contains(", Notes\nApproved.\n"));
        assert!(block.contains(", Them\n[2.1] So.\n[2.2] The limit holds.\n"));
        assert!(!block.contains("So. The limit holds."));
    }

    fn context(send: bool) -> changes::Context {
        changes::Context {
            settings: crate::app_settings::AppSettings::default(),
            send,
            state: catalog::State::default(),
        }
    }

    /// With changes on, the model reads how to write them and the
    /// settings it may name; a small model only its articles' and the core
    /// ones.
    #[test]
    fn a_prompt_with_changes_carries_the_protocol_and_the_catalog() {
        let question = "Когда диктую после напечатанного слова, текст слипается";
        let large = build(
            question,
            &[],
            Room::Large,
            Reach::Guide,
            Some(&context(true)),
            &Stage::None,
        );
        assert!(large.instructions.contains("```hex-change"));
        assert!(large.instructions.contains("```hex-vocab"));
        assert!(large.input.contains("<changeable_settings>"));
        assert!(
            large
                .input
                .contains("labs.spoken_formatting | on/off | Labs › Spoken formatting | ")
        );
        let small = build(
            question,
            &[],
            Room::Small,
            Reach::Guide,
            Some(&context(false)),
            &Stage::None,
        );
        let lines = |prompt: &Prompt| {
            prompt
                .input
                .lines()
                .filter(|line| line.contains(" | "))
                .count()
        };
        assert!(
            lines(&small) < lines(&large) / 2,
            "{} of {}",
            lines(&small),
            lines(&large)
        );
        assert!(small.input.contains("transcription.model | "));
        assert!(
            small
                .input
                .contains("don't know the person's current values")
        );
        let none = build(question, &[], Room::Large, Reach::Guide, None, &Stage::None);
        assert!(!none.input.contains("<changeable_settings>"));
        assert!(!none.instructions.contains("hex-change"));
    }

    #[test]
    fn an_earlier_turns_changes_and_refusals_go_back_in() {
        let card = changes::Card {
            items: vec![changes::Item {
                key: "labs.smart_spacing".into(),
                to: serde_json::json!(true),
                ..changes::Item::default()
            }],
            decision: changes::Decision::Declined,
            ..changes::Card::default()
        };
        let refused = Refused {
            line: "{\"key\":\"labs.teleport\"}".into(),
            key: Some("labs.teleport".into()),
            why: changes::Why::Unknown,
        };
        let line = changes_line(&[card], std::slice::from_ref(&refused));
        assert_eq!(
            line,
            "labs.smart_spacing = true: declined\nrefused: {\"key\":\"labs.teleport\"} (no such setting)"
        );
        let earlier = [Turn {
            question: "q",
            answer: "a",
            changes: &line,
        }];
        let prompt = build(
            "next",
            &earlier,
            Room::Large,
            Reach::Guide,
            None,
            &Stage::None,
        );
        assert!(
            prompt
                .input
                .contains("a\n<changes>\nlabs.smart_spacing = true: declined")
        );
        let again = repair(&prompt, "reply", &[refused]);
        assert!(again.instructions.contains("refused"));
        assert!(
            again
                .input
                .contains("<refused>\n{\"key\":\"labs.teleport\"} : no such setting\n</refused>")
        );
    }

    #[test]
    fn a_change_block_never_shows_while_it_streams() {
        let reply = "Turn on Smart spacing.\n\n```hex-change\n{\"key\":\"labs.sm";
        assert_eq!(draft(reply), "Turn on Smart spacing.");
        assert!(cards_coming(reply));
        assert_eq!(
            draft("Turn on Smart spacing.\n``"),
            "Turn on Smart spacing."
        );
        assert!(!cards_coming("Turn on Smart spacing."));
    }

    #[test]
    fn setup_asks_its_questions_then_its_plan() {
        let ask = build(
            "Помоги мне настроить VibeHex под меня",
            &[],
            Room::Small,
            Reach::Guide,
            Some(&context(true)),
            &Stage::Ask,
        );
        assert!(ask.instructions.contains("```hex-ask"));
        assert!(ask.instructions.contains("\"id\":\"dictation_code\""));
        assert!(!ask.instructions.contains("```hex-change"));
        assert!(!ask.input.contains("<changeable_settings>"));
        let plan = build(
            "",
            &[],
            Room::Small,
            Reach::Guide,
            Some(&context(true)),
            &Stage::Plan {
                answers: vec![setup::Answer {
                    id: "language".into(),
                    chosen: vec!["ru".into()],
                    ..Default::default()
                }],
                line: "language: ru (Русский)".into(),
                since: String::new(),
            },
        );
        assert!(plan.instructions.contains("```hex-plan"));
        assert!(
            plan.input
                .ends_with("<answers>\nlanguage: ru (Русский)\n</answers>\n")
        );
        assert!(!plan.input.contains("<question>"));
        // Every key, though the room is small.
        assert!(plan.input.contains("labs.spoken_formatting | on/off"));
        assert!(
            plan.articles.contains(&"topics/speech-models"),
            "{:?}",
            plan.articles
        );
    }

    /// A returning setup's prompt carries what came since: the use's
    /// aggregates and the last setup, never a dictation's text.
    #[test]
    fn a_returning_setups_prompt_has_no_recording_text() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assistant/recordings/history.json");
        let entries = crate::history::read_snapshot(&root).unwrap();
        let usage = crate::assistant::recordings::usage::from_entries(&entries, &[], u64::MAX / 2);
        let since = setup::since::Since {
            usage: Some(usage),
            ..Default::default()
        };
        let facts = setup::since::prompt(&since);
        let plan = build(
            "",
            &[],
            Room::Small,
            Reach::Guide,
            Some(&context(true)),
            &Stage::Plan {
                answers: Vec::new(),
                line: "use: dictation_any (Dictation into any app)".into(),
                since: facts.clone(),
            },
        );
        assert!(plan.instructions.contains("never offer again"));
        let block = plan
            .input
            .split("<since_last_setup>")
            .nth(1)
            .and_then(|rest| rest.split("</since_last_setup>").next())
            .unwrap();
        assert!(block.contains("dictations kept:"), "{block}");
        for entry in &entries {
            for text in [&entry.raw_text, &entry.final_text] {
                assert!(!plan.input.contains(text.as_str()), "{text}");
                let words: Vec<&str> = text.split_whitespace().collect();
                for three in words.windows(3) {
                    let three = three.join(" ");
                    assert!(!block.contains(&three), "{three} in {block}");
                }
            }
        }
    }

    #[test]
    fn a_streaming_sources_line_never_shows() {
        assert_eq!(draft("Answer.\nSour"), "Answer.");
        assert_eq!(draft("Answer.\nSources: labs/"), "Answer.");
        assert_eq!(draft("Answer.\nSome more"), "Answer.\nSome more");
        assert_eq!(draft("Answer"), "Answer");
    }
}
