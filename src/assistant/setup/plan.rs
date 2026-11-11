//! Setup's one list of changes. The app's rules come first (`rules`):
//!
//! - What already fits is kept and named under "Already set" (a speech
//!   model that handles the language, the engine the person left
//!   preselected, Labs features on).
//! - A speech model is never replaced while it handles the language well:
//!   Russian with English tech terms, or code, keeps Orukeet or Parakeet
//!   v3 and never gets GigaAM, which is Russian-only and skips Vocabulary.
//! - Round one's engines map to their keys (`engines`); a follow-up's
//!   "Yes" brings its changes. These are the person's own answers, so they
//!   start checked, unless the engine isn't ready yet.
//! - A line the rules infer over a value the person chose themselves
//!   (anything but the default) starts unchecked, with "You chose".
//!
//! A remote model's plan lines (`plan`) then join under the same rules: a
//! key the rules decide stays theirs (only a reason in the person's
//! language is taken), and a shorter retention or a Codex line signed out
//! starts unchecked. The list is grouped by area.

use serde_json::Value;

use super::{Answer, engines, follow, picked};
use crate::app_settings::AppSettings;
use crate::assistant::changes::catalog::{self, State};
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::vocab;
use crate::assistant::changes::{self, Card, CardKind, Item, parts};
use crate::control::settings;
use crate::desktop::i18n::tr;
use crate::transcription::models::{self, TranscriptionModelId};

/// The most lines a plan keeps.
pub const PLAN_LINES: usize = 24;

/// One line the rules make.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub key: String,
    pub value: Value,
    /// Why, in English, for `tr`.
    pub why: &'static str,
    /// The person asked for it in a question.
    pub answer: bool,
    /// Where to finish first, in English, when it isn't ready ("" for the
    /// Codex sign-in, which has its own row).
    pub note: Option<&'static str>,
}

impl Line {
    /// A line the person's answer asks for.
    pub fn answer(key: &str, value: Value, why: &'static str) -> Self {
        Self {
            key: key.into(),
            value,
            why,
            answer: true,
            note: None,
        }
    }

    /// A line the rules infer.
    fn rule(key: &str, value: Value, why: &'static str) -> Self {
        Self {
            answer: false,
            ..Self::answer(key, value, why)
        }
    }

    /// Not ready yet: unchecked, with where to finish.
    pub fn not_ready(&mut self, note: Option<&'static str>) {
        if note.is_some() {
            self.note = note;
        }
    }
}

/// What the rules make of the answers: lines, the keys kept as they are,
/// and every key they decide.
#[derive(Clone, Debug, Default)]
pub struct Rules {
    pub lines: Vec<Line>,
    pub kept: Vec<String>,
    pub owned: Vec<String>,
}

/// The keys round one decides, whatever a model writes.
const OWNED: [&str; 11] = [
    "transcription.model",
    "dictation_processing.server.runs_on",
    parts::CLEANUP,
    "meetings.notes.writer",
    "meetings.summaries",
    "files.cleanup_engine",
    "labs.ai_cleanup_server",
    "meetings.enabled",
    "labs.files",
    "labs.russian_vibe_coding",
    "dictation_processing.server.local_model",
];

/// Whether Russian is spoken with English tech terms: picked so, or
/// Russian while dictating code.
pub fn tech(answers: &[Answer]) -> bool {
    let language = picked(answers, "language").next().unwrap_or_default();
    let code = picked(answers, "use").any(|id| id == "dictation_code");
    language == "ru_tech" || (language == "ru" && code)
}

/// The rules' lines for `answers` against `settings`.
pub fn rules(answers: &[Answer], settings: &AppSettings, state: &State) -> Rules {
    let uses: Vec<&str> = picked(answers, "use").collect();
    let language = picked(answers, "language").next().unwrap_or_default();
    let cleanup = picked(answers, "cleanup").next();
    let notes = picked(answers, "notes").next();
    let dictates = uses.contains(&"dictation_any") || uses.contains(&"dictation_code");
    let code = uses.contains(&"dictation_code");
    let (meetings, files) = (uses.contains(&"meetings"), uses.contains(&"files"));
    let mut rules = Rules {
        owned: OWNED.map(str::to_owned).to_vec(),
        ..Rules::default()
    };
    match speech_model(language, code, settings) {
        Model::Keep => rules.kept.push("transcription.model".into()),
        Model::Suggest(model, why) => rules.lines.push(Line::rule(
            "transcription.model",
            Value::from(model.as_str()),
            why,
        )),
        Model::Nothing => {}
    }
    let switch = |key: &str, why: &'static str, rules: &mut Rules| {
        if changes::current(settings, key) == Some(Value::Bool(true)) {
            rules.kept.push(key.into());
        } else {
            rules.lines.push(Line::answer(key, Value::Bool(true), why));
        }
    };
    if dictates && let Some(id) = cleanup {
        let (lines, kept) = engines::dictation(id, settings, state);
        rules.lines.extend(lines);
        rules.kept.extend(kept);
    }
    // Russian vibe coding needs a cleanup engine; it turns cleanup on.
    if tech(answers) && cleanup.is_some_and(|id| id != "none") {
        switch(
            "labs.russian_vibe_coding",
            "Russian speech with English tech terms written as they should be",
            &mut rules,
        );
        rules.lines.retain(|line| line.key != parts::CLEANUP);
    }
    if meetings {
        switch(
            "meetings.enabled",
            "Records calls, with a transcript and notes",
            &mut rules,
        );
    }
    if files {
        switch(
            "labs.files",
            "Turns recordings you already have into text",
            &mut rules,
        );
    }
    if (meetings || files)
        && let Some(id) = notes
    {
        let (lines, kept) = engines::notes(id, meetings, files, settings, state);
        rules.lines.extend(lines);
        rules.kept.extend(kept);
    }
    // A follow-up's "Yes": the person's own answer.
    for answer in answers {
        for (key, value) in &answer.changes {
            if key != vocab::KEY && changes::current(settings, key).as_ref() == Some(value) {
                rules.kept.push(key.clone());
                continue;
            }
            rules.owned.push(key.clone());
            rules
                .lines
                .push(Line::answer(key, value.clone(), "You said yes to this"));
        }
    }
    // The Labs features that fit and are on already.
    for key in follow::relevant(answers, settings) {
        if changes::current(settings, &key) == Some(Value::Bool(true)) && !rules.kept.contains(&key)
        {
            rules.kept.push(key);
        }
    }
    rules
        .lines
        .retain(|line| settings::unavailable(&line_key(&line.key, settings)).is_none());
    rules
        .kept
        .retain(|key| settings::unavailable(&line_key(key, settings)).is_none());
    rules.kept.dedup();
    rules
}

pub(super) fn line_key(key: &str, settings: &AppSettings) -> String {
    match parts::part(key, settings) {
        Some(part) => part.parent().to_owned(),
        None => key.to_owned(),
    }
}

/// The rules' lines as proposals, each why in the window's language.
pub fn proposals(rules: &Rules) -> Vec<Proposal> {
    rules
        .lines
        .iter()
        .map(|line| Proposal {
            key: line.key.clone(),
            value: Some(line.value.clone()),
            why: tr(line.why).to_owned(),
            plan: true,
            line: line.key.clone(),
            ..Proposal::default()
        })
        .collect()
}

/// What happens to the speech model.
enum Model {
    Keep,
    Suggest(TranscriptionModelId, &'static str),
    Nothing,
}

/// The speech model for `language`: the one in use while it handles the
/// language well, else one that does. Code or tech terms in Russian keep
/// Orukeet or Parakeet v3 (Parakeet v3 if neither), never GigaAM; plain
/// Russian keeps GigaAM or Orukeet (GigaAM v3 if neither); English, any
/// English model; other or mixed, any with more than one language.
fn speech_model(language: &str, code: bool, settings: &AppSettings) -> Model {
    use TranscriptionModelId::{GigaAmV3, Orukeet, ParakeetV3};
    let now = settings.transcription.model;
    let definition = models::definition(now);
    let (good, model, why) = match language {
        "ru_tech" => (
            matches!(now, Orukeet | ParakeetV3),
            ParakeetV3,
            "Hears Russian with English tech terms and reads your Vocabulary",
        ),
        "ru" if code => (
            matches!(now, Orukeet | ParakeetV3),
            ParakeetV3,
            "Hears Russian with English tech terms and reads your Vocabulary",
        ),
        "ru" => (
            matches!(now, GigaAmV3 | Orukeet),
            GigaAmV3,
            "Best accuracy for Russian",
        ),
        "en" => {
            let Some(choice) = models::choices_for_runtime("en").into_iter().next() else {
                return Model::Nothing;
            };
            (
                definition.supports_language("en"),
                choice.model.id,
                "Made for English",
            )
        }
        "mixed" => (
            definition.languages.len() > 1,
            ParakeetV3,
            "Hears many languages",
        ),
        _ => return Model::Nothing,
    };
    if good {
        return Model::Keep;
    }
    let usable = models::MODELS
        .iter()
        .any(|known| known.id == model && known.available());
    if usable && model != now {
        Model::Suggest(model, why)
    } else {
        Model::Nothing
    }
}

/// Where a plan's line goes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Area {
    Dictation,
    Code,
    Meetings,
    Files,
    Models,
}

impl Area {
    /// Its heading, in English for `tr`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dictation => "Dictation",
            Self::Code => "Code",
            Self::Meetings => "Meetings",
            Self::Files => "Files",
            Self::Models => "Models",
        }
    }
}

/// The area `key`'s line goes in: a Labs feature by its article's first
/// use.
pub fn area(key: &str) -> Area {
    if key.starts_with("transcription.") {
        return Area::Models;
    }
    if key == vocab::KEY || key == "labs.russian_vibe_coding" {
        return Area::Code;
    }
    if key.starts_with("meetings.") {
        return Area::Meetings;
    }
    if key.starts_with("files.") || key == "labs.files" {
        return Area::Files;
    }
    let first = crate::assistant::guide::articles()
        .iter()
        .find(|article| article.is_lab() && article.keys.first().is_some_and(|known| known == key))
        .and_then(|article| article.uses.first());
    match first.map(String::as_str) {
        Some("code") => Area::Code,
        Some("meetings") => Area::Meetings,
        Some("files") => Area::Files,
        _ => Area::Dictation,
    }
}

/// Whether `key` is set to something but its default: the person chose
/// it.
fn yours(key: &str, settings: &AppSettings) -> bool {
    let default = AppSettings::default();
    changes::current(settings, key) != changes::current(&default, key)
}

/// One checklist from the rules and a model's plan cards (none for the
/// app's own): every item once, none held here, at most [`PLAN_LINES`],
/// grouped by area, with "Already set". `None` when nothing is left to
/// show.
pub fn plan(
    cards: Vec<Card>,
    answers: &[Answer],
    settings: &AppSettings,
    state: &State,
) -> Option<Card> {
    let rules = rules(answers, settings, state);
    let shortcuts = crate::rune_ui::shortcut_rules();
    let (own, _) = changes::resolve(&proposals(&rules), settings, &shortcuts);
    let mut items: Vec<Item> = Vec::new();
    let mut terms = Vec::new();
    for mut item in own.into_iter().flat_map(|card| card.items) {
        if item.key == vocab::KEY {
            terms.push(item);
            continue;
        }
        let Some(line) = rules.lines.iter().find(|line| line.key == item.key) else {
            continue;
        };
        item.yours = !line.answer && yours(&item.key, settings);
        item.note = line.note.unwrap_or_default().to_owned();
        item.included = !item.yours && line.note.is_none();
        if !items.iter().any(|known| known.key == item.key) {
            items.push(item);
        }
    }
    let answered_terms = !terms.is_empty();
    for item in cards.into_iter().flat_map(|card| card.items) {
        if item.key == vocab::KEY {
            terms.push(item);
            continue;
        }
        if rules.owned.contains(&item.key) || rules.kept.contains(&item.key) {
            // The person's language for a line the rules make too.
            if let Some(known) = items
                .iter_mut()
                .find(|known| known.key == item.key && known.to == item.to)
                && !item.why.trim().is_empty()
            {
                known.why = item.why;
            }
            continue;
        }
        let held = settings::unavailable(&line_key(&item.key, settings)).is_some();
        let known = items.iter().any(|known| known.key == item.key);
        if held || known || item.from == item.to {
            continue;
        }
        let mut item = item;
        item.yours = yours(&item.key, settings);
        item.included &= !item.yours;
        items.push(item);
    }
    for item in &mut items {
        let shorter = catalog::shorter(&item.key, &item.from, &item.to);
        let signed_out =
            catalog::needs_codex(&item.key, &item.to) && state.codex_signed_in == Some(false);
        if shorter.is_some() || signed_out {
            item.included = false;
        }
    }
    items.sort_by_key(|item| area(&item.key));
    items.truncate(PLAN_LINES);
    // Every term in one row: checked when a follow-up asked for them.
    if let Some(mut row) = vocab::row(terms) {
        row.included = answered_terms;
        let at = items
            .iter()
            .position(|item| area(&item.key) > Area::Code)
            .unwrap_or(items.len());
        items.insert(at, row);
    }
    let mut kept: Vec<Item> = Vec::new();
    for key in &rules.kept {
        let Some(now) = changes::current(settings, key) else {
            continue;
        };
        if kept.iter().any(|known| known.key == *key) || items.iter().any(|item| item.key == *key) {
            continue;
        }
        kept.push(Item {
            key: key.clone(),
            from: now.clone(),
            to: now,
            ..Item::default()
        });
    }
    kept.sort_by_key(|item| area(&item.key));
    (!items.is_empty() || !kept.is_empty()).then(|| Card {
        kind: CardKind::Plan,
        items,
        kept,
        ..Card::default()
    })
}

/// Every English reason the rules give, for the translation tables' test.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    vec![
        "Russian speech with English tech terms written as they should be",
        "Records calls, with a transcript and notes",
        "Turns recordings you already have into text",
        "You said yes to this",
        "Hears Russian with English tech terms and reads your Vocabulary",
        "Best accuracy for Russian",
        "Made for English",
        "Hears many languages",
        "Dictation",
        "Code",
        "Meetings",
        "Files",
        "Models",
    ]
}
