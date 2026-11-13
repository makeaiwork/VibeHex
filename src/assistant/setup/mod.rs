//! Guided setup, "Set VibeHex up for me": what is set now, the app's four
//! questions with every chip preselected from the settings, follow-up
//! rounds about the Labs features that fit the answers, then one list of
//! changes the person ticks and applies.
//!
//! Round one is the app's own (`questions`, fixed ids): what VibeHex is
//! for, the language, where dictation is cleaned up (the person waits for
//! it), and where meeting notes and files are worked on (they may take
//! longer). The Start screen's button draws it at once, in the window's
//! language, with no model call; a typed request ("помоги настроить") asks
//! the model to write it in the message's language (`hex-ask` lines), and
//! `asks` keeps only known ids, falling back to the app's own. Either way
//! `preselect` picks each chip from the settings.
//!
//! The follow-up rounds come from the Labs articles' `for:` and `ask:`
//! (`follow`): the app picks the ones that fit, a remote model may reword,
//! drop, and add to them, and each option carries its changes. The plan
//! (`plan`) is the app's rules first, which keep what already fits, never
//! move a choice the person made without saying so, and leave a line that
//! isn't ready yet unchecked; a remote model's lines join them under the
//! same rules.

pub mod about;
pub mod engines;
pub mod flow;
pub mod follow;
pub mod now;
pub mod past;
pub mod plan;
pub mod preset;
pub mod since;
pub mod spellings;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::app_settings::AppSettings;
use crate::desktop::i18n::tr;
use crate::transcription::models;

/// One question with option chips.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Ask {
    /// `use`, `language`, `cleanup`, `notes`, or a follow-up's id.
    pub id: String,
    /// The question as the model wrote it; for the app's own (`own`), the
    /// English the pane translates.
    pub q: String,
    /// A few words beside it: "pick any", a Labs feature's title.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
    /// Several options may be chosen.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub multi: bool,
    pub options: Vec<Choice>,
    /// The app's own words, shown in the window's language.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
    /// The option ids picked so far; round one's start from the settings.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chosen: Vec<String>,
    /// The person picked something here themselves.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub touched: bool,
    /// Answered: it shows as chosen chips.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
    /// Answered in an earlier setup and not asked again: it shows folded
    /// under "Answered before", its answer changeable there.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub before: bool,
}

/// One option of a question.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Choice {
    pub id: String,
    pub label: String,
    /// A line under the label: what picking it means.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// A name after the label, never translated: "LM Studio".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub extra: String,
    /// What a follow-up's option changes: `(key, value)`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<(String, Value)>,
}

/// What the person picked for one question.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Answer {
    pub id: String,
    pub chosen: Vec<String>,
    /// What the chosen options change (a follow-up's "Yes").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<(String, Value)>,
}

/// Where a setup conversation is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stage {
    /// An ordinary question.
    #[default]
    None,
    /// A typed setup request: the model writes round one's questions.
    Ask,
    /// Round one is answered: a remote model rewords, drops, or adds to
    /// the follow-up questions the app picked (`candidates`).
    /// `since` is what a returning setup knows came since the last one
    /// (`since::prompt`), empty for a first one.
    Follow {
        answers: Vec<Answer>,
        line: String,
        candidates: Vec<Ask>,
        since: String,
        /// The follow-ups answered in an earlier setup (`past`), joining
        /// the round folded, as answered then.
        before: Vec<Ask>,
        /// Round one's own words (`about`), empty when none were written.
        about: String,
    },
    /// Every round is answered: the model writes the plan. `line` is the
    /// answers as the model reads them, with the labels the person saw;
    /// `terms`, what a returning setup's use shows Vocabulary lacks.
    Plan {
        answers: Vec<Answer>,
        line: String,
        since: String,
        terms: Vec<since::Candidate>,
        /// Round one's own words (`about`).
        about: String,
    },
    /// A returning setup's opening, on a remote model: it reads what came
    /// since (`facts`, aggregates only) and what the app already showed
    /// (`shown`, in the window's language) and writes a few suggestions;
    /// round one (`asks`, folded) and the terms come after them, with the
    /// app's step of what it read.
    Look {
        facts: String,
        shown: String,
        asks: Vec<Ask>,
        terms: Vec<since::Candidate>,
        step: crate::assistant::recordings::Step,
        /// Round one's own words as last written, prefilled after the look.
        about: String,
    },
}

impl Ask {
    /// The question's words as the person sees them: the app's own in the
    /// window's language, the model's as written.
    pub fn words<'a>(&self, words: &'a str) -> &'a str {
        if self.own {
            crate::desktop::i18n::tr(words)
        } else {
            words
        }
    }

    /// An option's label as the person saw it, with the server it names.
    pub fn label(&self, option: &Choice) -> String {
        let label = self.words(&option.label);
        if option.extra.is_empty() {
            label.to_owned()
        } else {
            format!("{label} ({})", option.extra)
        }
    }
}

impl Stage {
    /// The answers it carries, if any.
    pub fn answers(&self) -> &[Answer] {
        match self {
            Self::Follow { answers, .. } | Self::Plan { answers, .. } => answers,
            _ => &[],
        }
    }

    /// Round one's own words it carries, if any.
    pub fn about(&self) -> &str {
        match self {
            Self::Follow { about, .. } | Self::Plan { about, .. } | Self::Look { about, .. } => {
                about
            }
            _ => "",
        }
    }
}

/// An option: id, English label, and English caption.
type Opt = (&'static str, &'static str, &'static str);

/// A fixed question: id, the English question, its hint, whether several
/// may be picked, and its options.
type Question = (
    &'static str,
    &'static str,
    &'static str,
    bool,
    &'static [Opt],
);

const QUESTIONS: [Question; 4] = [
    (
        "use",
        "What do you use VibeHex for?",
        "pick any",
        true,
        &[
            ("dictation_any", "Dictation into any app", ""),
            ("dictation_code", "Dictation into code and the terminal", ""),
            ("meetings", "Recording calls", ""),
            ("files", "Transcribing files", ""),
        ],
    ),
    (
        "language",
        "Which language do you usually speak?",
        "",
        false,
        &[
            ("ru", "Russian", ""),
            ("en", "English", ""),
            ("ru_tech", "Russian with English tech terms", ""),
            ("mixed", "Other or mixed", ""),
        ],
    ),
    (
        "cleanup",
        "Where should dictation be cleaned up?",
        "you wait for it",
        false,
        &[
            (
                "local",
                "On this Mac",
                "Fastest. A small model, little context",
            ),
            (
                "codex",
                "Codex",
                "ChatGPT plan. Slower, better on long ones",
            ),
            ("opencode", "OpenCode", "The OpenCode app, if it runs"),
            ("openrouter", "OpenRouter", "Cloud models, one API key"),
            ("custom", "My own server", "LM Studio, Ollama, or alike"),
            ("none", "No cleanup", "Pastes as you said it"),
        ],
    ),
    (
        "notes",
        "Where should meeting notes and files be worked on?",
        "may take longer",
        false,
        &[
            ("codex", "Codex", "ChatGPT plan. Reads a long call whole"),
            ("opencode", "OpenCode", "The OpenCode app, if it runs"),
            ("openrouter", "OpenRouter", "Cloud models, one API key"),
            ("custom", "My own server", "LM Studio, Ollama, or alike"),
            ("local", "On this Mac", "Private. Reads less at once"),
            (
                "apple",
                "Apple Intelligence",
                "This Mac, meeting notes only",
            ),
        ],
    ),
];

/// "On this Mac", off a Mac.
const OFF_MAC: &str = "On this computer";

/// The hint the language question gets when code was picked with
/// Russian, and Russian with English tech terms is preselected.
pub const TECH_HINT: &str = "for code, tech terms usually stay in English";

/// The app's own four questions on `os`, in English for the pane to
/// translate. Off a Mac, "this Mac" is "this computer" and Apple
/// Intelligence isn't offered.
pub fn questions(os: &str) -> Vec<Ask> {
    QUESTIONS
        .iter()
        .map(|&(id, q, hint, multi, options)| Ask {
            id: id.into(),
            q: q.into(),
            hint: hint.into(),
            multi,
            own: true,
            options: options
                .iter()
                .filter(|(option, ..)| *option != "apple" || os == "macos")
                .map(|&(id, label, note)| Choice {
                    id: id.into(),
                    label: if label == "On this Mac" && os != "macos" {
                        OFF_MAC.into()
                    } else {
                        label.into()
                    },
                    note: note.into(),
                    ..Choice::default()
                })
                .collect(),
            ..Ask::default()
        })
        .collect()
}

/// Every English word the app's own questions show, for the translation
/// tables' test.
#[cfg(test)]
pub fn own_words() -> Vec<&'static str> {
    let mut words = vec![OFF_MAC, TECH_HINT];
    words.extend(about::words());
    words.extend(follow::words());
    words.extend(engines::words());
    words.extend(plan::words());
    words.extend(since::words());
    words.extend(crate::assistant::changes::companion::words());
    for (_, q, hint, _, options) in QUESTIONS {
        words.extend([q, hint].into_iter().filter(|word| !word.is_empty()));
        for (_, label, note) in options {
            words.extend([*label, *note].into_iter().filter(|word| !word.is_empty()));
        }
    }
    words
}

/// Round one's chips as the settings have them: the uses they show, the
/// language the speech model is set to, where cleanup runs now, and who
/// writes notes. A question the person touched keeps their pick; Russian
/// with code picked is Russian with English tech terms, with a hint why.
pub fn preselect(asks: &mut [Ask], settings: &AppSettings, state: &super::facts::State) {
    if let Some(ask) = asks.iter_mut().find(|ask| ask.id == "use" && !ask.touched) {
        ask.chosen = uses_now(settings)
            .iter()
            .map(|id| (*id).to_owned())
            .collect();
    }
    let code = asks
        .iter()
        .find(|ask| ask.id == "use")
        .is_some_and(|ask| ask.chosen.iter().any(|id| id == "dictation_code"));
    let language = language_now(settings, code);
    for ask in asks.iter_mut().filter(|ask| !ask.touched) {
        let chosen = match ask.id.as_str() {
            "language" => language,
            "cleanup" => engines::dictation_now(settings),
            "notes" => engines::notes_now(settings, state.os),
            _ => continue,
        };
        if ask.options.iter().any(|option| option.id == chosen) {
            ask.chosen = vec![chosen.to_owned()];
        }
        if ask.id == "language" && ask.own {
            ask.hint = if chosen == "ru_tech" && code {
                TECH_HINT.into()
            } else {
                String::new()
            };
        }
    }
    let server = engines::own_server(&settings.dictation_processing.server.url, state.servers());
    for ask in asks.iter_mut() {
        for option in ask
            .options
            .iter_mut()
            .filter(|option| option.id == "custom")
        {
            option.extra = server.unwrap_or_default().to_owned();
        }
    }
}

/// The uses the settings show: dictation always (a dictation shortcut is
/// always set), code with Russian vibe coding or a terminal lab on,
/// meetings and files with theirs.
fn uses_now(settings: &AppSettings) -> Vec<&'static str> {
    let labs = &settings.labs;
    let mut uses = vec!["dictation_any"];
    if labs.russian_vibe_coding || labs.terminal_paste_shortcut || settings.paste_with_shift {
        uses.push("dictation_code");
    }
    if settings.meetings.enabled {
        uses.push("meetings");
    }
    if labs.files {
        uses.push("files");
    }
    uses
}

/// The language the speech model is set to, as round one names it.
fn language_now(settings: &AppSettings, code: bool) -> &'static str {
    let tech = code || settings.labs.russian_vibe_coding;
    let model = models::definition(settings.transcription.model);
    let language = settings.transcription.language.as_str();
    let language = match language {
        "" | "auto" if model.languages.len() == 1 => model.languages[0],
        language => language,
    };
    match language.split(['-', '_']).next().unwrap_or_default() {
        "ru" if tech => "ru_tech",
        "ru" => "ru",
        "en" => "en",
        _ => "mixed",
    }
}

/// Whether a round-one question shows: the one about notes and files only
/// for calls or files.
pub fn shows(asks: &[Ask], ask: &Ask) -> bool {
    if ask.id != "notes" {
        return true;
    }
    let uses = asks
        .iter()
        .find(|ask| ask.id == "use")
        .map(|ask| ask.chosen.as_slice())
        .unwrap_or_default();
    uses.iter().any(|id| id == "meetings" || id == "files")
}

/// A model's words, or the app's own in Assistant's language where it only
/// handed back the English it was shown: a small model asked to write the
/// questions in the person's language may copy the example as it was.
fn written_or_own(written: &str, english: &str) -> String {
    if written.eq_ignore_ascii_case(english.trim()) {
        tr(english).to_owned()
    } else {
        written.to_owned()
    }
}

/// The questions to show for the `hex-ask` lines a model wrote: its own
/// words when every fixed question came with known option ids, else the
/// app's own. A caption it left out is the app's, translated, and so is
/// any text it copied from the English example (`written_or_own`).
pub fn asks(written: &[Value], os: &str) -> Vec<Ask> {
    let own = questions(os);
    let read: Option<Vec<Ask>> = own
        .iter()
        .map(|fixed| {
            let line = written
                .iter()
                .find(|line| line.get("id").and_then(Value::as_str) == Some(&fixed.id))?;
            let q = line.get("q").and_then(Value::as_str)?.trim();
            let options: Vec<Choice> = line
                .get("options")?
                .as_array()?
                .iter()
                .filter_map(|option| {
                    let id = option.get("id").and_then(Value::as_str)?;
                    let label = option.get("label").and_then(Value::as_str)?.trim();
                    let known = fixed.options.iter().find(|known| known.id == id)?;
                    let note = option
                        .get("note")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|note| !note.is_empty())
                        .map_or_else(
                            || tr(&known.note).to_owned(),
                            |note| written_or_own(note, &known.note),
                        );
                    (!label.is_empty()).then(|| Choice {
                        id: id.into(),
                        label: written_or_own(label, &known.label),
                        note,
                        ..Choice::default()
                    })
                })
                .collect();
            let hint = line
                .get("hint")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();
            (!q.is_empty() && options.len() >= 2).then(|| Ask {
                id: fixed.id.clone(),
                q: written_or_own(q, &fixed.q),
                hint: written_or_own(hint, &fixed.hint),
                multi: fixed.multi,
                options,
                ..Ask::default()
            })
        })
        .collect();
    read.unwrap_or(own)
}

/// The `hex-ask` lines a model is shown to write, in English.
pub fn ask_lines(os: &str) -> String {
    questions(os)
        .iter()
        .map(ask_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// One question as a `hex-ask` line: its id, words, and options.
pub fn ask_line(ask: &Ask) -> String {
    let mut line = serde_json::json!({
        "id": ask.id,
        "q": ask.q,
        "options": ask.options.iter().map(|option| {
            let mut written = serde_json::json!({"id": option.id, "label": option.label});
            if !option.note.is_empty() {
                written["note"] = option.note.clone().into();
            }
            written
        }).collect::<Vec<_>>(),
    });
    if !ask.hint.is_empty() {
        line["hint"] = ask.hint.clone().into();
    }
    if ask.multi {
        line["multi"] = true.into();
    }
    line.to_string()
}

/// The answers as the model reads them: `use: dictation_code (Dictation
/// into code…), meetings (…); language: ru (…)`, with `label` naming each
/// option as the person saw it. A question left unanswered is left out.
pub fn answers_line(asks: &[Ask], label: impl Fn(&Ask, &Choice) -> String) -> String {
    asks.iter()
        .filter(|ask| !ask.chosen.is_empty())
        .map(|ask| {
            let chosen: Vec<String> = ask
                .options
                .iter()
                .filter(|option| ask.chosen.contains(&option.id))
                .map(|option| format!("{} ({})", option.id, label(ask, option)))
                .collect();
            format!("{}: {}", ask.id, chosen.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What each answered question's chosen options are, and change.
pub fn answers(asks: &[Ask]) -> Vec<Answer> {
    asks.iter()
        .filter(|ask| ask.done && !ask.chosen.is_empty())
        .map(|ask| Answer {
            id: ask.id.clone(),
            chosen: ask.chosen.clone(),
            changes: ask
                .options
                .iter()
                .filter(|option| ask.chosen.contains(&option.id))
                .flat_map(|option| option.changes.iter().cloned())
                .collect(),
        })
        .collect()
}

/// What the guide is searched for, for these answers.
pub fn query(answers: &[Answer]) -> String {
    let mut words = vec!["set up"];
    for answer in answers {
        for id in &answer.chosen {
            words.push(match (answer.id.as_str(), id.as_str()) {
                (_, "dictation_any") => "dictation smart spacing vocabulary",
                (_, "dictation_code") => "vibe coding terminal code",
                ("use", "meetings") => "meetings notes calls",
                ("use", "files") => "transcribe files",
                ("language", "ru") => "Russian speech model",
                ("language", "ru_tech") => "Russian vibe coding speech model",
                ("language", "en") => "English speech model",
                ("language", "mixed") => "speech model languages",
                (_, "codex") => "Codex ChatGPT AI cleanup",
                (_, "local") => "built-in model on this computer",
                (_, "custom") => "own server LM Studio Ollama",
                (_, "openrouter") => "OpenRouter",
                _ => continue,
            });
        }
    }
    words.join(" ")
}

/// The ids picked for question `id`.
pub fn picked<'a>(answers: &'a [Answer], id: &str) -> impl Iterator<Item = &'a str> {
    answers
        .iter()
        .filter(move |answer| answer.id == id)
        .flat_map(|answer| answer.chosen.iter().map(String::as_str))
}

/// The owner's own setup, which a scene draws and the tests hold the
/// plan to: Orukeet in Russian, dictation cleaned up on this Mac for
/// speed (Gemma 4 E4B), notes and Files on Codex, Russian vibe coding and
/// Vocabulary quick add on, 94 terms, LM Studio's address saved.
pub fn sample(settings: &mut AppSettings) {
    use crate::app_settings::CleanupRunsOn;
    use crate::dictation::vocabulary::VocabularyEntry;
    settings.transcription.model = models::TranscriptionModelId::Orukeet;
    settings.transcription.language = "ru".into();
    settings
        .dictation_processing
        .default_mode
        .post_processing
        .enabled = true;
    settings.labs.ai_cleanup_server = true;
    let server = &mut settings.dictation_processing.server;
    server.runs_on = CleanupRunsOn::Local;
    server.local_model = "gemma-4-e4b-it".into();
    server.url = "http://127.0.0.1:1234/v1".into();
    settings.meetings.enabled = true;
    settings.meetings.notes.writer = Some("codex:gpt-6.1-terra".into());
    settings.labs.files = true;
    settings.files.cleanup_engine = Some("codex".into());
    settings.labs.russian_vibe_coding = true;
    settings.labs.vocabulary_quick_add = true;
    settings.vocabulary = (1..=94)
        .map(|number| VocabularyEntry {
            term: format!("Term{number}"),
            sounds_like: Vec::new(),
        })
        .collect();
}

#[cfg(test)]
mod tests;
