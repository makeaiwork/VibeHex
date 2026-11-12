//! What a model reads for one question: Assistant's instructions, the
//! guide's table of contents, the articles search found, the settings a
//! card may change (with their values while the person lets it read them,
//! `changes::catalog`), the conversation so far, and the question. Nothing
//! else: no transcripts, history, meetings, or keys. The model ends its
//! reply with a `Sources:` line, which the pane turns into "From the guide"
//! chips and never shows, and may write `hex-change` lines before it, which
//! the pane turns into cards.

use super::changes::{self, Refused, catalog};
use super::facts;
use super::guide::{self, Article};
use super::intent;
use super::recordings::{Excerpt, Found, Kind, cite};
use super::releases;
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
    /// It carries Vocabulary entries: personal, so no web search.
    pub vocabulary: bool,
    /// What it tells the model of this install, for the turn to keep
    /// (`facts::text::knew`); empty when it tells nothing.
    pub knew: String,
}

const INSTRUCTIONS: &str = "You are Assistant, the help built into VibeHex, a dictation app \
for macOS, Windows, and Linux. Answer the person's question about VibeHex from the guide \
excerpts you are given.

- Reply in the language of the person's latest question, whatever the language of the guide \
or of the app. If that question has no clear language (a word or two, a name), keep the \
conversation's; with nothing of theirs to go by, their default language (below).
- Name panes, buttons, and settings exactly as the guide writes them, in English, with › \
between levels (Labs › Meetings), so they can be found on screen. Put a place in the window \
in bold, its whole path at once (**General › Vocabulary**): it becomes a link that opens it.
- Answer first, in a sentence or two, then the steps as a short numbered list when there \
are steps. No preamble.
- Say which systems a feature works on when it matters, and say plainly when VibeHex cannot \
do something.
- Use only the excerpts. When they don't answer the question, say the guide doesn't cover \
it. Never invent settings, keys, buttons, or menus.
- When the best choice depends on how the person uses VibeHex (Russian full of English \
terms or plain Russian, say), ask one short question or name both cases instead of \
recommending one.
- Never suggest replacing a setting with one that is worse for what their settings show: \
with Russian vibe coding on they dictate English terms in Russian, so not GigaAM v3.
- You never change settings yourself; say where the person changes them.
- For questions that are not about VibeHex, say in one sentence that you help with VibeHex.
- End with one last line: `Sources:` and the ids of the articles you used, comma-separated \
(for example `Sources: labs/meetings, topics/codex`), or `Sources: none`.";

/// The last line a question's prompt ends on: a model that has just read
/// pages in English and earlier turns in another language still answers
/// in the question's own.
const LATEST: &str = "Reply in the language the question above is written in; if it is too \
short to tell (a word or two, a name), in the conversation's.";

/// The language Assistant writes in when the person's own words don't say
/// (`assistant::language`): their speech recognition language, else the
/// window's.
fn default_language(settings: Option<&crate::app_settings::AppSettings>) -> String {
    let name = super::language::name(settings);
    format!(
        "\n\nTheir default language is {name}: the language they dictate in (or, when that \
         isn't set, the app's). The app shows its own words in it. Write setup's questions, \
         sentences, and plans in it when they typed no request of their own. It is not the \
         language of an answer: a question clearly in another language (English after turns in \
         {name}, say) is answered in the question's language, whatever the earlier turns and \
         this default are."
    )
}

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
- Never a card as a way around what you don't know, or only so the person can look something \
up (a list, a filter, a view): a card only when the change itself does what they asked or \
clearly improves how they use VibeHex. When <state> says unknown, or you can't tell, say \
where in the window they can see it, its place in bold, and write no card.

To add names or terms dictation gets wrong to the person's Vocabulary, write one JSON object \
per term in one block, after your answer, before the Sources line:

```hex-vocab
{\"term\":\"Kubernetes\",\"sounds_like\":[\"кубернетис\",\"кубер\"],\"why\":\"a short reason\"}
```

- `term` spelled as it should be written; `sounds_like`: up to 5 ways a transcript gets it \
wrong, in the language they speak. At most 8 terms, only ones the question is about.
- You see only the Vocabulary entries this question names, in <vocabulary> when it is there \
(with how many there are); the app leaves out a term it has.
- To change an entry <vocabulary> lists, write a line in the same block with its `term` as \
listed and what changes: `remove_sounds_like` (spellings it has), `add_sounds_like`, \
`rename` (the term spelled anew): \
{\"term\":\"AI cleanup\",\"remove_sounds_like\":[\"ай\"],\"why\":\"a short reason\"}. \
Only entries listed there; the card shows each before and after.";

/// A question about what cleanup or recognition did to their words, with
/// their instructions and Vocabulary entries to go by.
const DIAGNOSE: &str = "

The question is about what dictation did to their words. <cleanup_instructions> holds their \
cleanup instructions and replacements when the catalog carries it, and <vocabulary> the \
Vocabulary entries the conversation names. Find the cause from them: an entry whose term or \
sounds-like spellings catch the word (\"AI\" listed as a spelling of \"AI cleanup\" writes \
AI cleanup whenever they say AI), a replacement, a line of the instructions, or else the \
speech model. Name it and how to fix it; when it is the instructions, offer the fix as a card \
for the instructions key; when it is a Vocabulary entry, as a line changing that entry.";

const HEARD_WROTE: &str = "
- A dictation that cleanup changed shows two lines: Heard, what the speech model wrote down (after Vocabulary's own corrections), and Wrote, what was typed into the app. When the question asks why a word came out as it did, compare them, quoting both: a word already wrong in Heard is the speech model hearing it so, or a Vocabulary entry whose sounds-like spellings catch it; one right in Heard and changed in Wrote is cleanup: a replacement, a line of their instructions, or a Vocabulary entry the cleanup model was told to write. Name the cause. When it is a setting you can see, offer the fix as a card; for a Vocabulary entry, a line changing that entry.";

/// With the person's settings sent: answers for them, not for anyone.
const PERSONAL: &str = "

The now column of <changeable_settings> shows the person's own settings. Answer for them, in \
this order:
- Start from what they already have. Before recommending anything, look up the now column \
for every feature the excerpts offer for this question. One that is on and helps opens the \
answer: its first sentence says they have it on and how to use it, with their own value as \
the now column shows it (their shortcut, not the default).
- Then what would help and is off: say what it does and offer it with a card.
- What doesn't fit their setup (another system, a model they don't use) goes last or not at all.";

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
fn setup_follow(candidates: &str, others: &str, before: &str) -> String {
    format!(
        "

<answers> holds the person's answers to the first setup questions. The app picked the \
follow-up questions below, each about one Labs feature or setting, best first; each option's \
id is what the app changes on it (yes turns the feature on, no leaves it).

```hex-ask
{candidates}
```

- Write one short sentence, in the language of the setup request or, without one, in their \
default language, that a few questions about features that fit how they work follow.
- Then one ```hex-ask block, one JSON object per line, the most useful first, at most 12: \
the questions above that fit these answers, with `q` and every `label` in that language \
and every `id` exactly as written. Leave out one that doesn't fit them.
- You may add a question about a Labs feature from <labs_catalog> (`key | title | \
question`) that clearly fits, as {{\"id\":\"<key>\",\"q\":\"...\",\"options\":[{{\"id\":\"yes\",\"label\":\"...\"}},{{\"id\":\"no\",\"label\":\"...\"}}]}}, or about a \
setting from <changeable_settings> with each option's `value` as the list writes it (an \
option without one changes nothing).
- Never ask about a feature or setting in <answered_before> (`key | title`): the person \
answered it in an earlier setup, and the app keeps that answer.
- Questions only: no changes, no plan. End with `Sources: none`.

<labs_catalog>
{others}
</labs_catalog>

<answered_before>
{before}
</answered_before>"
    )
}

const SETUP_PLAN: &str = "

<answers> holds the person's answers to the setup questions: each question's id, then the \
ids of the options they picked, with the words they saw. A follow-up's id is the setting it \
asks about; `yes` means they want it. The app already builds the list from these answers: \
the speech model, where dictation and notes run, Meetings, Files, Russian vibe coding, and \
every follow-up they said yes to. Your part is a sentence and what it may have missed.

- First one or two sentences, in the language of the setup request or, without one, in their \
default language: what the list is for, and that they can untick what they don't need.
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

const SETUP_LOOK: &str = "

The person pressed Set VibeHex up for me again: they set it up before, or have used it a \
while. <since_last_setup> holds what the app knows: their last setup's answers and what it \
applied and declined, what is new in VibeHex since, and what their own use shows, worked \
out by the app as counts, shares, and single words (never their text). \
<changeable_settings> is how VibeHex is set now. <shown> is what the app has just shown \
them, in their default language.

- Write 2 to 4 suggestions as a short Markdown list, in their default language, no more \
than two lines each: one concrete change to how they work with VibeHex, and the fact from \
<since_last_setup> it comes from, in a few words. For example: most dictations go to code \
editors and cleanup keeps fixing кодекс → Codex, so Codex belongs in Vocabulary.
- Prefer what is new since and fits how they work. Never suggest what they declined then, \
or what the settings already have. Name a setting by where it is in the window.
- No greeting, no heading, no facts repeated from <shown>, no closing question: the app asks \
next whether anything changed in how they use VibeHex, and the list of changes comes after \
their answers. Write no change blocks. End with `Sources: none`.";

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
    /// A question about what changed: the release notes it reads.
    Releases(&'a releases::Selection),
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
- When the latest question refers back (answer the earlier question, more, the others), the \
excerpts were found again for the earlier question about recordings: answer that from them. \
While this block is here you can read their recordings; if an earlier answer said you \
couldn't, it was wrong.
- Times are the person's local time; the block says when now is.";

const DECLINED: &str = "

This question seems to be about the person's own meetings, files, or dictations, which you \
cannot read: they haven't allowed it. Say so in one sentence, and that Assistant › Settings › \
Your meetings, files, and dictations allows it; then answer what the guide covers, if anything.";

const LINKED: &str = "The person's question also reaches past this text, to their OTHER \
recordings. The <other_recordings> block holds numbered excerpts from them, found on their \
computer for this question: other meetings (notes and passages, Me is the person, Them the \
others), Files texts, and dictations. A transcript excerpt has one numbered line per phrase: \
[2.3] is line 3 of excerpt 2.
- Answer from the text first. Where the question reaches past it, bring the excerpts in: say \
which recording each fact is from and when, and how they connect with this text (the same \
topic coming back, a decision that changed, one thing leading to another).
- Cite every fact from an excerpt right after it, in square brackets: the lines that say it, \
like [2.3], or a run of lines, [2.3-2.5]; an excerpt without numbered lines by its number, \
[1]. Only ids in the block. Facts from this text keep their times as before, never a bracket.
- When neither this text nor the excerpts hold the answer, say so plainly. Never guess what \
was said.
- Times are the person's local time; the block says when now is.";

/// What a question in Requests is told when it reaches past its text and
/// the person said not now to reading their other recordings.
pub const LINKED_DECLINED: &str = "The question may be about the person's other meetings, \
files, or dictations, which you cannot read: they said not now. Say so in one short sentence, \
then answer from this text alone.";

/// What a question in a meeting's or a file's Requests adds when it reaches
/// past its own text: how to use and cite the other recordings `found`,
/// and the block of them, numbered as the Assistant numbers its excerpts.
pub fn linked(found: &Found, now_ms: u64) -> String {
    format!("{LINKED}\n{}", tagged("other_recordings", found, now_ms))
}

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
        // What came since names the features worth reading about.
        (Stage::Look { facts, .. }, _) => format!("{} {facts}", setup::query(&[])),
        (_, Some(last)) => format!("{} {}", last.question, question),
        (_, None) => question.to_owned(),
    };
    let found = retrieve::search(&query, room.articles());
    let mut input = contents();
    for article in &found {
        input.push_str(&excerpt(article));
    }
    // A question about what cleanup or recognition does to their words
    // (or the turn before it) reads their instructions and the Vocabulary
    // entries it names; so does one the cleanup or Vocabulary articles
    // answer.
    let wants = std::iter::once(question)
        .chain(earlier.iter().rev().take(2).map(|turn| turn.question))
        .map(intent::cleanup::wants)
        .fold(
            intent::cleanup::Wants::default(),
            intent::cleanup::Wants::or,
        )
        .or(intent::cleanup::Wants {
            instructions: found
                .iter()
                .any(|article| article.id == "topics/output-and-instructions"),
            vocabulary: found.iter().any(|article| {
                article.id == "topics/vocabulary" || article.id.starts_with("labs/vocabulary")
            }),
        });
    let mut vocabulary = false;
    let mut knew = String::new();
    let mut instructions = INSTRUCTIONS.to_owned();
    instructions.push_str(&default_language(changes.map(|context| &context.settings)));
    if *stage == Stage::Ask {
        let os = changes.map_or(std::env::consts::OS, |context| context.state.os);
        instructions.push_str(&setup_ask(os));
    } else if let Some(context) = changes {
        let plan = matches!(
            stage,
            Stage::Plan { .. } | Stage::Follow { .. } | Stage::Look { .. }
        );
        match stage {
            Stage::Follow { candidates, .. } => {
                let (asks, others, before) =
                    setup::follow::prompt_lines(candidates, &context.settings, &context.state);
                instructions.push_str(&setup_follow(&asks, &others, &before));
            }
            Stage::Plan { .. } => instructions.push_str(SETUP_PLAN),
            Stage::Look { .. } => instructions.push_str(SETUP_LOOK),
            _ => instructions.push_str(CHANGES),
        }
        if let Stage::Plan { since, .. } | Stage::Follow { since, .. } = stage
            && !since.trim().is_empty()
        {
            instructions.push_str(SETUP_SINCE);
        }
        if context.send() && !plan {
            instructions.push_str(PERSONAL);
        }
        // What the releases asked about changed may be offered too.
        let released = match reach {
            Reach::Releases(selection) => selection.settings_keys(),
            _ => Vec::new(),
        };
        let reading = catalog::Reading {
            // A plan reads every key, whatever the room.
            every: plan || room == Room::Large,
            keys: found
                .iter()
                .flat_map(|article| article.keys.iter().map(String::as_str))
                .chain(released)
                .collect(),
            instructions: wants.instructions,
        };
        let facts = context.facts();
        let mut sent = facts::text::Sent {
            articles: found.len(),
            instructions: context.send() && wants.instructions,
            ..facts::text::Sent::default()
        };
        if let Reach::Recordings(found, _) = &reach {
            sent.recordings = found.excerpts.len();
        }
        input.push('\n');
        input.push_str(&catalog::section(&context.settings, &reading, &facts));
        if context.send() && wants.vocabulary && !plan {
            let mut texts = vec![question];
            for turn in earlier.iter().rev().take(2) {
                texts.extend([turn.question, turn.answer]);
            }
            // The entries the dictations found name, too.
            if let Reach::Recordings(found, _) = &reach {
                texts.extend(found.excerpts.iter().map(|excerpt| excerpt.text.as_str()));
            }
            let (block, named) = super::personal::vocabulary(&context.settings, &texts);
            input.push_str(&block);
            sent.vocabulary = Some((named, context.settings.vocabulary.len()));
            // Only entries are the person's own: a block naming none of
            // them keeps nothing off the web.
            vocabulary = named > 0;
            instructions.push_str(DIAGNOSE);
        }
        knew = facts::text::knew(&facts, &sent);
    }
    match reach {
        Reach::Guide => {}
        Reach::Declined => instructions.push_str(DECLINED),
        Reach::Recordings(recordings, now_ms) => {
            instructions.push_str(RECORDINGS);
            if recordings
                .excerpts
                .iter()
                .any(|excerpt| excerpt.text.starts_with(super::recordings::HEARD))
            {
                instructions.push_str(HEARD_WROTE);
            }
            input.push_str(&recordings_block(recordings, now_ms));
        }
        Reach::Releases(selection) => {
            instructions.push_str(releases::prompt::INSTRUCTIONS);
            input.push_str(&releases::prompt::block(selection));
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
        Stage::Look { facts, shown, .. } => input.push_str(&format!(
            "\n<since_last_setup>\n{}\n</since_last_setup>\n<shown>\n{}\n</shown>\n",
            facts.trim(),
            shown.trim()
        )),
        _ => input.push_str(&format!(
            "\n<question>\n{}\n</question>\n{LATEST}\n",
            question.trim()
        )),
    }
    Prompt {
        instructions,
        input,
        articles: found.iter().map(|article| article.id).collect(),
        vocabulary,
        knew,
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
        vocabulary: prompt.vocabulary,
        knew: prompt.knew.clone(),
    }
}

const REWRITE: &str = "

Your reply above suggested changes the app shows no card for: each setting in <already_set> \
has that value now, and each in <not_here> can't be used on this computer. Write your whole \
reply again, the same answer in the same language, except: where it says such a setting is \
set otherwise or offers to change it, say in a few words that they already have it, or that \
it isn't available on this computer, and write no change line for it. \
Keep the other change lines (fixed, if <refused> names them) and end with the Sources line.";

/// One more ask, for a remote model that suggested a value already set or
/// a setting this computer can't have: its reply, which settings those
/// are, and to write the whole reply again so its words claim no card.
pub fn rewrite(
    prompt: &Prompt,
    reply: &str,
    already: &[String],
    not_here: &[String],
    refused: &[Refused],
) -> Prompt {
    let mut input = prompt.input.clone();
    input.push_str(&format!(
        "\n<your_reply>\n{}\n</your_reply>\n",
        reply.trim()
    ));
    for (tag, lines) in [("already_set", already), ("not_here", not_here)] {
        if !lines.is_empty() {
            input.push_str(&format!("<{tag}>\n{}\n</{tag}>\n", lines.join("\n")));
        }
    }
    if !refused.is_empty() {
        input.push_str("<refused>\n");
        for refused in refused {
            input.push_str(&format!("{} : {}\n", refused.line, refused.reason()));
        }
        input.push_str("</refused>\n");
    }
    Prompt {
        instructions: format!("{}{REWRITE}", prompt.instructions),
        input,
        articles: prompt.articles.clone(),
        vocabulary: prompt.vocabulary,
        knew: prompt.knew.clone(),
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
    tagged("recordings", found, now_ms)
}

/// The excerpts in a `<tag>` block.
fn tagged(tag: &str, found: &Found, now_ms: u64) -> String {
    let mut block = format!("\n<{tag} now=\"{}\">\n", stamp(now_ms));
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
    block.push_str(&format!("</{tag}>\n"));
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

    /// "Which speech model is best for Russian?" depends on how they speak:
    /// the article names both cases, and the model is told to ask or name
    /// both, and never to trade a model for a worse one for their settings.
    #[test]
    fn a_choice_that_depends_on_how_they_work_is_asked_or_named_both_ways() {
        for room in [Room::Small, Room::Large] {
            let prompt = build(
                "Which speech model is best for Russian?",
                &[],
                room,
                Reach::Guide,
                None,
                &Stage::None,
            );
            assert_eq!(prompt.articles.first(), Some(&"topics/speech-models"));
            assert!(prompt.input.contains("Russian with English tech terms"));
            assert!(prompt.input.contains("GigaAM v3 only for"));
            assert!(
                prompt
                    .instructions
                    .contains("ask one short question or name both cases")
            );
            assert!(prompt.instructions.contains("so not GigaAM v3"));
        }
    }

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
        assert!(prompt.input.ends_with(&format!(
            "<question>\nCan VibeHex record my Zoom calls?\n</question>\n{LATEST}\n"
        )));
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
        // And the app's own last line, on which language to answer in.
        for text in ["earlier question", "earlier answer", "new question", LATEST] {
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
        // Requests read the same excerpts as other recordings.
        let linked = linked(&found, 0);
        assert!(linked.contains("OTHER recordings"));
        assert!(linked.contains("<other_recordings now="));
        assert!(linked.contains("[2.2] The limit holds.\n"));
        assert!(linked.trim_end().ends_with("</other_recordings>"));
    }

    fn context(send: bool) -> changes::Context {
        let mut settings = crate::app_settings::AppSettings::default();
        settings.assistant.send_settings = send;
        changes::Context {
            settings,
            state: facts::State::default(),
        }
    }

    /// With the person's settings sent, the model is told to answer for
    /// them: what is on first, with their own shortcut; what is off as a
    /// card. "Some words come out wrong" reads the quick-add article.
    #[test]
    fn with_settings_sent_an_answer_is_for_the_person() {
        let question = "Some words come out wrong. How do I fix that?";
        let sent = build(
            question,
            &[],
            Room::Large,
            Reach::Guide,
            Some(&context(true)),
            &Stage::None,
        );
        assert!(sent.instructions.contains("Answer for them"));
        assert!(
            sent.instructions
                .contains("their shortcut, not the default")
        );
        assert!(
            sent.articles.contains(&"labs/vocabulary-quick-add"),
            "{:?}",
            sent.articles
        );
        let unsent = build(
            question,
            &[],
            Room::Large,
            Reach::Guide,
            Some(&context(false)),
            &Stage::None,
        );
        assert!(!unsent.instructions.contains("Answer for them"));
    }

    /// The owner's question, in Russian, about cleanup writing "AI cleanup"
    /// for "AI": the prompt carries their instructions, every mode's, and
    /// the Vocabulary entries the words name, and says how to find the
    /// cause. With settings not sent, none of it, and where to allow it.
    /// "Why does it write X instead of Y" with the dictations that wrote
    /// X: the model compares what was heard with what was written, and
    /// sees the Vocabulary entries the dictations name.
    #[test]
    fn a_word_written_wrong_is_compared_heard_against_wrote() {
        use crate::assistant::recordings::Excerpt;
        let mut sent = context(true);
        sent.settings.vocabulary = vec![crate::dictation::vocabulary::VocabularyEntry {
            term: "AI cleanup".into(),
            sounds_like: vec!["AI".into()],
        }];
        let found = Found {
            excerpts: vec![Excerpt {
                n: 1,
                kind: Kind::Dictation,
                id: "7".into(),
                title: "Notes".into(),
                at_ms: 0,
                offset_ms: None,
                label: String::new(),
                text: "Heard: turn on AI for notes\nWrote: turn on AI cleanup for notes".into(),
                lines: Vec::new(),
            }],
            steps: Vec::new(),
        };
        let prompt = build(
            "Почему пишет AI cleanup вместо AI?",
            &[],
            Room::Large,
            Reach::Recordings(&found, 0),
            Some(&sent),
            &Stage::None,
        );
        assert!(prompt.instructions.contains("compare them, quoting both"));
        assert!(
            prompt
                .input
                .contains("Heard: turn on AI for notes\nWrote: turn on AI cleanup")
        );
        assert!(prompt.input.contains("AI cleanup (sounds like: AI)"));
        // A dictation cleanup left alone needs no comparing.
        let plain = Found {
            excerpts: vec![Excerpt {
                text: "turn on AI cleanup".into(),
                ..found.excerpts[0].clone()
            }],
            steps: Vec::new(),
        };
        let prompt = build(
            "Почему пишет AI cleanup вместо AI?",
            &[],
            Room::Large,
            Reach::Recordings(&plain, 0),
            Some(&sent),
            &Stage::None,
        );
        assert!(!prompt.instructions.contains("compare them, quoting both"));
    }

    #[test]
    fn a_question_about_cleanup_reads_the_instructions_and_named_terms() {
        let mut sent = context(true);
        sent.settings
            .dictation_processing
            .default_mode
            .post_processing
            .prompt = "Fix punctuation. Keep tech terms in English.".into();
        sent.settings.vocabulary = vec![
            crate::dictation::vocabulary::VocabularyEntry {
                term: "AI cleanup".into(),
                sounds_like: vec!["эй ай клинап".into(), "AI".into()],
            },
            crate::dictation::vocabulary::VocabularyEntry {
                term: "Kubernetes".into(),
                sounds_like: vec!["кубер".into()],
            },
        ];
        let question = "можешь проверить, в промпте есть ли какие-то штуки, которые могли привести к этому, для AI cleanup";
        let earlier = [Turn {
            question: "у меня бывает, продиктовал слово, и оно распозналось как часть, например AI cleanup. Если я просто скажу AI, он может AI cleanup вставить",
            answer: "Без записи в словаре точную причину определить нельзя.",
            changes: "",
        }];
        let prompt = build(
            question,
            &earlier,
            Room::Large,
            Reach::Guide,
            Some(&sent),
            &Stage::None,
        );
        assert!(prompt.input.contains("<cleanup_instructions>"));
        assert!(
            prompt
                .input
                .contains("Fix punctuation. Keep tech terms in English.")
        );
        assert!(prompt.input.contains("<vocabulary total=\"2\">"));
        assert!(
            prompt
                .input
                .contains("AI cleanup (sounds like: эй ай клинап, AI)")
        );
        assert!(!prompt.input.contains("Kubernetes (sounds like"));
        assert!(prompt.vocabulary);
        assert!(prompt.instructions.contains("Find the cause from them"));
        assert!(
            !prompt
                .instructions
                .contains("You never see their vocabulary")
        );

        let unsent = build(
            question,
            &earlier,
            Room::Large,
            Reach::Guide,
            Some(&context(false)),
            &Stage::None,
        );
        assert!(!unsent.input.contains("<cleanup_instructions>"));
        assert!(!unsent.input.contains("<vocabulary"));
        assert!(!unsent.vocabulary);
        assert!(
            unsent
                .input
                .contains("**Assistant › Settings › Your current settings**")
        );
    }

    /// Dictating Russian in an English window: the model is told Russian is
    /// the default; with recognition on Auto, the window's language.
    #[test]
    fn the_prompt_carries_the_default_language() {
        crate::desktop::i18n::apply(Some("en"));
        let mut context = context(true);
        context.settings.transcription.model =
            crate::transcription::models::TranscriptionModelId::ParakeetV3;
        context.settings.transcription.language = "ru".into();
        let ask = |context: &changes::Context| {
            build(
                "ok",
                &[],
                Room::Large,
                Reach::Guide,
                Some(context),
                &Stage::None,
            )
            .instructions
        };
        assert!(ask(&context).contains("Their default language is Russian:"));
        context.settings.transcription.language = "auto".into();
        assert!(ask(&context).contains("Their default language is English:"));
        let setup = build("", &[], Room::Large, Reach::Guide, None, &Stage::None);
        assert!(
            setup
                .instructions
                .contains("Their default language is English:")
        );
    }

    /// "Which models do I have downloaded?": the snapshot names them, and
    /// the model is told never to offer a card in place of what it
    /// doesn't know, such as a filter to see the list.
    #[test]
    fn no_card_stands_in_for_what_the_model_doesnt_know() {
        let mut context = context(true);
        context.state.installed = facts::Fact::Known(vec!["parakeet_v3".into()]);
        let prompt = build(
            "Tell me which models I have downloaded",
            &[],
            Room::Large,
            Reach::Guide,
            Some(&context),
            &Stage::None,
        );
        assert!(
            prompt
                .instructions
                .contains("Never a card as a way around what you don't know")
        );
        assert!(prompt.instructions.contains("When <state> says unknown"));
        assert!(
            prompt
                .input
                .contains("Speech models downloaded: Parakeet v3 (parakeet_v3")
        );
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
                terms: Vec::new(),
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
                terms: Vec::new(),
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

    /// A returning setup's opening asks for a few suggestions from what
    /// came since and what was shown, never for changes, and carries no
    /// dictation's text.
    #[test]
    fn a_returning_setups_look_asks_for_suggestions_only() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assistant/recordings/history.json");
        let entries = crate::history::read_snapshot(&root).unwrap();
        let usage = crate::assistant::recordings::usage::from_entries(&entries, &[], u64::MAX / 2);
        let since = setup::since::Since {
            usage: Some(usage),
            ..Default::default()
        };
        let look = build(
            "",
            &[],
            Room::Large,
            Reach::Guide,
            Some(&context(true)),
            &Stage::Look {
                facts: setup::since::prompt(&since),
                shown: setup::since::paragraph(&since),
                asks: Vec::new(),
                terms: Vec::new(),
                step: crate::assistant::recordings::Step::Looked {
                    dictations: 5,
                    meetings: 0,
                },
            },
        );
        assert!(look.instructions.contains("2 to 4 suggestions"));
        // Written in their default language, as every setup prompt is.
        assert!(look.instructions.contains("Their default language is"));
        assert!(
            look.instructions
                .contains("in their default language, no more")
        );
        assert!(!look.instructions.contains("```hex-change"));
        assert!(
            look.input.contains("<since_last_setup>\ndictations kept:")
                || look.input.contains("<since_last_setup>\nno setup before")
        );
        assert!(look.input.contains("<shown>\n"));
        assert!(!look.input.contains("<question>"));
        for entry in &entries {
            assert!(!look.input.contains(entry.final_text.as_str()));
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
