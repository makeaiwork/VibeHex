//! Setup's follow-up rounds: yes/no questions about the Labs features that
//! fit round one's answers, as many as fit, at most [`ROUND`] a round and
//! [`ROUNDS`] rounds.
//!
//! Each Labs article says whom it is for (`for: [code, ru]`) and asks one
//! question (`ask:`). A feature fits when one of its uses was picked
//! (`dictation` for any dictation), its qualifiers hold (`ru` for Russian,
//! `cleanup` while some engine cleans up), this OS and build have it, and
//! it isn't on already, counting what round one's answers turn on (Russian
//! vibe coding turns on several). The more specific its uses, the sooner
//! it is asked. A remote model may reword them, drop some, and add ones
//! from the catalog (`follow_ups`), which the app checks key by key.
//!
//! A follow-up answered in an earlier setup isn't asked again while it
//! holds (`past::answered_before`): the app leaves it out, tells a model
//! never to ask it, drops it if a model asks it anyway, and shows it
//! folded under "Answered before" with its answer (`before`).

use serde_json::Value;

use super::plan::{self, tech};
use super::{Answer, Ask, Choice, past, picked};
use crate::app_settings::AppSettings;
use crate::assistant::changes::vocab::{self, Term};
use crate::assistant::changes::{self, names, parse::Proposal};
use crate::assistant::facts::State;
use crate::assistant::guide::{self, Article};
use crate::assistant::offer::{Dropped, Gate, Offer, Origin, Verdict, Why};
use crate::control::settings;
use crate::desktop::hardware::Hardware;
use crate::desktop::i18n::{tr, tr_window};

/// The most questions a round asks.
pub const ROUND: usize = 4;
/// The most follow-up rounds.
pub const ROUNDS: usize = 3;

pub const YES: &str = "Yes, turn it on";
pub const NO: &str = "No";
const ADD: &str = "Yes, add them";

/// A follow-up of the app's own, not about a Labs feature.
pub struct Own {
    pub id: &'static str,
    pub ask: &'static str,
}

/// Tech terms for someone dictating code in Russian without Russian vibe
/// coding, which brings its own.
pub const OWN: [Own; 1] = [Own {
    id: "vocabulary.code_terms",
    ask: "Add tech terms you likely say to your Vocabulary?",
}];

/// Terms someone dictating code in Russian likely says.
const TERMS: [(&str, &[&str]); 5] = [
    ("GitHub", &["гитхаб", "гит хаб"]),
    ("Claude Code", &["клод код"]),
    ("pull request", &["пул реквест", "пулл реквест"]),
    ("TypeScript", &["тайпскрипт", "тайп скрипт"]),
    ("Docker", &["докер"]),
];

/// How much a use or qualifier makes a feature fit: the more specific,
/// the more.
fn weight(tag: &str) -> usize {
    match tag {
        "code" => 3,
        "meetings" | "files" | "ru" => 2,
        "dictation" => 1,
        _ => 0,
    }
}

/// How well `article` fits `answers`, or `None` when it doesn't.
fn fits(article: &Article, answers: &[Answer]) -> Option<usize> {
    let uses: Vec<&str> = picked(answers, "use").collect();
    let language = picked(answers, "language").next().unwrap_or_default();
    let cleans = picked(answers, "cleanup").any(|id| id != "none")
        || (uses.iter().any(|id| *id == "meetings" || *id == "files")
            && picked(answers, "notes").next().is_some());
    let mut used = false;
    let mut score = 0;
    for tag in &article.uses {
        let holds = match tag.as_str() {
            "dictation" => uses.iter().any(|id| id.starts_with("dictation")),
            "code" => uses.contains(&"dictation_code"),
            "meetings" => uses.contains(&"meetings"),
            "files" => uses.contains(&"files"),
            "ru" => {
                if !matches!(language, "ru" | "ru_tech") {
                    return None;
                }
                true
            }
            "cleanup" => {
                if !cleans {
                    return None;
                }
                true
            }
            _ => false,
        };
        if holds {
            used |= !matches!(tag.as_str(), "ru" | "cleanup");
            score += weight(tag);
        }
    }
    used.then_some(score)
}

/// The Labs articles setup may ask about here: they say whom they are for
/// and ask a question, and the gate has their switch here (this OS and
/// build have it, and this computer may have what it needs: a notch, a
/// mouse's extra buttons).
fn askable<'a>(gate: &'a Gate) -> impl Iterator<Item = (&'static Article, &'static str)> + 'a {
    guide::articles().iter().filter_map(move |article| {
        let key = article.keys.first()?.as_str();
        (article.is_lab()
            && !article.uses.is_empty()
            && !article.ask.is_empty()
            && gate.here(key).is_ok())
        .then_some((article, key))
    })
}

/// Whether `key` is a Labs switch whose feature this computer can't use.
pub fn unfit(key: &str, hardware: &Hardware) -> bool {
    guide::articles().iter().any(|article| {
        article.is_lab() && article.keys.iter().any(|known| known == key) && !article.fits(hardware)
    })
}

/// The switches of the Labs features that fit `answers`, best first.
pub fn relevant(answers: &[Answer], gate: &Gate) -> Vec<String> {
    let mut found: Vec<(usize, &str)> = askable(gate)
        .filter_map(|(article, key)| Some((fits(article, answers)?, key)))
        .collect();
    found.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    found.into_iter().map(|(_, key)| key.to_owned()).collect()
}

/// A yes/no question about switch `key`, in English for the pane.
pub(super) fn yes_no(key: &str, q: &str, hint: &str) -> Ask {
    Ask {
        id: key.into(),
        q: q.into(),
        hint: hint.into(),
        own: true,
        options: vec![
            Choice {
                id: "yes".into(),
                label: YES.into(),
                changes: vec![(key.into(), Value::Bool(true))],
                ..Choice::default()
            },
            Choice {
                id: "no".into(),
                label: NO.into(),
                ..Choice::default()
            },
        ],
        ..Ask::default()
    }
}

/// The tech terms question, its "Yes" adding them.
fn terms_ask() -> Ask {
    let terms = TERMS
        .iter()
        .map(|(term, sounds_like)| {
            (
                vocab::KEY.to_owned(),
                Term {
                    term: (*term).into(),
                    sounds_like: sounds_like.iter().map(|&text| text.into()).collect(),
                }
                .value(),
            )
        })
        .collect();
    let mut ask = yes_no(OWN[0].id, OWN[0].ask, "");
    ask.hint = TERMS.map(|(term, _)| term).join(", ");
    ask.options[0].label = ADD.into();
    ask.options[0].changes = terms;
    ask
}

/// The settings after round one's checked lines: what is on once they are
/// saved, Russian vibe coding's Labs features included.
fn after(answers: &[Answer], settings: &AppSettings, state: &State) -> AppSettings {
    let rules = plan::rules(answers, settings, state);
    let shortcuts = crate::rune_ui::shortcut_rules();
    let lines: Vec<Proposal> = plan::proposals(&rules)
        .into_iter()
        .filter(|line| line.key != vocab::KEY)
        .collect();
    let gate = Gate::new(settings, state);
    let cards = changes::resolve(&lines, &gate, Origin::Plan).cards;
    let mut now = settings.clone();
    for card in cards {
        let Ok(values) = changes::values(&card, &now) else {
            continue;
        };
        if let Ok(change) = settings::check(&now, &values, &shortcuts) {
            now = change.settings;
        }
    }
    now
}

/// The follow-up questions for round one's `answers`, best first: the
/// Labs features that fit and the gate lets through (not on once round
/// one's lines are saved, not answered or declined before), and the tech
/// terms for Russian code without vibe coding.
pub fn candidates(answers: &[Answer], settings: &AppSettings, state: &State) -> Vec<Ask> {
    let after = after(answers, settings, state);
    let gate = Gate::new(settings, state);
    let asked: Vec<&str> = answers.iter().map(|answer| answer.id.as_str()).collect();
    let mut asks = Vec::new();
    let code = picked(answers, "use").any(|id| id == "dictation_code");
    if code
        && tech(answers)
        && !after.labs.russian_vibe_coding
        && !asked.contains(&OWN[0].id)
        && gate.ask(OWN[0].id, &after, false) == Verdict::Show
    {
        asks.push(terms_ask());
    }
    for key in relevant(answers, &gate) {
        if asked.contains(&key.as_str()) || gate.ask(&key, &after, false) != Verdict::Show {
            continue;
        }
        let Some((article, _)) = askable(&gate).find(|(_, known)| *known == key) else {
            continue;
        };
        asks.push(yes_no(
            &key,
            article.question(&state.hardware),
            &article.title,
        ));
    }
    asks.truncate(ROUND * ROUNDS);
    asks
}

/// The follow-ups answered before that these `answers` would ask about,
/// as answered then, to show folded beside the round.
pub fn before(answers: &[Answer], settings: &AppSettings, state: &State) -> Vec<Ask> {
    let relevant = relevant(answers, &Gate::new(settings, state));
    let code = picked(answers, "use").any(|id| id == "dictation_code");
    state
        .answered
        .iter()
        .filter(|ask| relevant.contains(&ask.id) || (ask.id == OWN[0].id && code && tech(answers)))
        .cloned()
        .collect()
}

/// The keys a model may not ask about: answered in an earlier setup, or
/// declined there while the setting is as it was.
pub fn settled(settings: &AppSettings, state: &State) -> Vec<String> {
    let mut keys: Vec<String> = state.answered.iter().map(|ask| ask.id.clone()).collect();
    for line in &state.declined {
        if past::held(&state.declined, &line.key, &line.to, settings) && !keys.contains(&line.key) {
            keys.push(line.key.clone());
        }
    }
    keys
}

/// The candidates as `hex-ask` lines, the other Labs features a model may
/// add, `key | title | question`, and those it may not ask about again,
/// `key | title`, for the follow-up prompt.
pub fn prompt_lines(
    candidates: &[Ask],
    settings: &AppSettings,
    state: &State,
) -> (String, String, String) {
    let settled = settled(settings, state);
    let hardware = &state.hardware;
    let gate = Gate::new(settings, state);
    let asks = candidates
        .iter()
        .map(super::ask_line)
        .collect::<Vec<_>>()
        .join("\n");
    let others = askable(&gate)
        .filter(|(_, key)| {
            !candidates.iter().any(|ask| ask.id == *key)
                && gate.ask(key, settings, false) == Verdict::Show
        })
        .map(|(article, key)| format!("{key} | {} | {}", article.title, article.question(hardware)))
        .collect::<Vec<_>>()
        .join("\n");
    let before = settled
        .iter()
        .map(|key| {
            let title = askable(&gate)
                .find(|(_, known)| known == key)
                .map(|(article, _)| article.title.as_str())
                .unwrap_or_default();
            format!("{key} | {title}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    (asks, others, before)
}

/// The follow-up questions a model wrote, each through the gate
/// (`Gate::ask`): a candidate keeps its options' changes with the model's
/// words; a Labs switch from the catalog gets the app's Yes and No; any
/// other key needs each option's `value` to pass as a plan's line would.
/// At most [`ROUND`] × [`ROUNDS`]; none usable, the candidates as they
/// are. What was dropped and why comes back beside them.
pub fn follow_ups(written: &[Value], candidates: &[Ask], gate: &Gate) -> (Vec<Ask>, Vec<Dropped>) {
    let mut asks: Vec<Ask> = Vec::new();
    let mut dropped = Vec::new();
    for line in written {
        let Some(id) = line.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if asks.iter().any(|known| known.id == id) {
            continue;
        }
        let candidate = candidates.iter().find(|ask| ask.id == id);
        if let Verdict::Drop(why) = gate.ask(id, gate.settings, candidate.is_some()) {
            dropped.push(Dropped {
                what: format!("ask {id}"),
                why,
            });
            continue;
        }
        let found = candidate.cloned().or_else(|| {
            askable(gate)
                .find(|(_, key)| *key == id)
                .map(|(article, key)| {
                    let hardware = &gate.state.hardware;
                    yes_no(key, article.question(hardware), &article.title)
                })
        });
        let ask = match found {
            Some(ask) => reworded(ask, line),
            None => setting(id, line, gate),
        };
        if ask.is_none() {
            dropped.push(Dropped {
                what: format!("ask {id}"),
                why: Why::Invalid("no option that changes it".into()),
            });
        }
        asks.extend(ask);
        if asks.len() == ROUND * ROUNDS {
            break;
        }
    }
    if asks.is_empty() {
        (candidates.to_vec(), dropped)
    } else {
        (asks, dropped)
    }
}

/// `ask` in the words a model wrote for it, where it wrote any: its
/// options stay the app's.
fn reworded(mut ask: Ask, line: &Value) -> Option<Ask> {
    let q = line.get("q").and_then(Value::as_str).map(str::trim);
    let Some(q) = q.filter(|q| !q.is_empty()) else {
        return Some(ask);
    };
    ask.q = q.into();
    ask.own = false;
    // A Labs feature's title names its place: the window's words.
    ask.hint = tr_window(&ask.hint).to_owned();
    let written = line.get("options").and_then(Value::as_array);
    for option in &mut ask.options {
        let label = written
            .into_iter()
            .flatten()
            .find(|written| written.get("id").and_then(Value::as_str) == Some(&option.id))
            .and_then(|written| written.get("label").and_then(Value::as_str))
            .map(str::trim)
            .filter(|label| !label.is_empty());
        option.label = match label {
            Some(label) => label.into(),
            None => tr(&option.label).into(),
        };
    }
    Some(ask)
}

/// A question about any other setting a model added: each option's
/// `value` through the gate as a plan's line, an option without one
/// changing nothing.
fn setting(key: &str, line: &Value, gate: &Gate) -> Option<Ask> {
    let q = line.get("q").and_then(Value::as_str)?.trim();
    if q.is_empty() {
        return None;
    }
    let mut options = Vec::new();
    for option in line.get("options")?.as_array()? {
        let id = option.get("id").and_then(Value::as_str)?.trim();
        let label = option.get("label").and_then(Value::as_str)?.trim();
        let changes = match option.get("value") {
            None | Some(Value::Null) => Vec::new(),
            Some(raw) => {
                let value = names::normalize(gate.settings, key, raw);
                if let Verdict::Drop(_) = gate.setting(Origin::Plan, Offer::of(key, &value)) {
                    return None;
                }
                vec![(key.to_owned(), value)]
            }
        };
        if !id.is_empty() && !label.is_empty() {
            options.push(Choice {
                id: id.into(),
                label: label.into(),
                changes,
                ..Choice::default()
            });
        }
    }
    let changing = options.iter().any(|option| !option.changes.is_empty());
    (options.len() >= 2 && changing).then(|| Ask {
        id: key.into(),
        q: q.into(),
        hint: line
            .get("hint")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .into(),
        options,
        ..Ask::default()
    })
}

/// The English words this file shows, for the translation tables' test.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    vec![YES, NO, ADD, OWN[0].ask]
}
