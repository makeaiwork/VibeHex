//! Setting changes an answer suggests, as cards the person decides on.
//!
//! The model writes plain text with fenced `hex-change` lines (`parse`),
//! naming only a key, a value, and why. The app does the rest: whether the
//! key exists and may be changed from a card (`catalog`), what it is called
//! in the window (`names`), its current value, what it needs first
//! (`control::settings::requires`), and the narrow parts of two JSON keys
//! (`parts`). Nothing is saved until the person presses the card's button,
//! and then only through `control::settings::check`, the path `hex settings`
//! takes. `state` derives what a card shows each frame.

pub mod catalog;
pub mod names;
pub mod parse;
pub mod parts;
pub mod state;
pub mod vocab;
pub mod vocab_edit;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::app_settings::AppSettings;
use crate::control::settings::{self, ShortcutRules};
use catalog::Class;
use parse::Proposal;

/// What a question's changes are offered and checked against.
#[derive(Clone, Debug)]
pub struct Context {
    /// The settings when it was asked.
    pub settings: AppSettings,
    /// What was read off this install then (`facts::gather`).
    pub state: crate::assistant::facts::State,
}

impl Context {
    /// The model may read the current values (`assistant.send_settings`).
    pub fn send(&self) -> bool {
        self.settings.assistant.send_settings
    }

    /// Everything Assistant knows of this install for these settings.
    pub fn facts(&self) -> crate::assistant::facts::Facts {
        crate::assistant::facts::Facts::of(&self.settings, &self.state)
    }
}

/// One suggestion the person decides on.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Card {
    pub kind: CardKind,
    pub items: Vec<Item>,
    pub decision: Decision,
    /// A permission it needs was put off with Later.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub later: bool,
    /// A setup plan's "Already set": what fits already and stays, each as
    /// an item whose `from` is its value now.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub kept: Vec<Item>,
}

impl Card {
    /// Whether `other` changes the same settings, in the same order.
    pub fn same_keys(&self, other: &Self) -> bool {
        self.items
            .iter()
            .map(|item| &item.key)
            .eq(other.items.iter().map(|item| &item.key))
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CardKind {
    /// One key to one value.
    #[default]
    Single,
    /// One key, a value picked from `options`; `chosen` starts at the one
    /// `suggested`.
    Choice {
        options: Vec<Value>,
        chosen: usize,
        #[serde(default)]
        suggested: usize,
    },
    /// Several keys, checked or not, applied together (setup).
    Plan,
    /// Instructions, shown line by line.
    Prompt,
    /// Vocabulary terms, checked or not, added together (`vocab`).
    Vocabulary,
    /// Entries already in Vocabulary, each before → after (`vocab_edit`).
    VocabularyEdit,
}

/// One key a card changes.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Item {
    /// A `hex settings` key, or a part of one (`parts`).
    pub key: String,
    /// Its value when suggested, as `hex settings get` shows it.
    pub from: Value,
    pub to: Value,
    /// The model's reason, in the question's language.
    pub why: String,
    /// Checked in a plan; always true elsewhere.
    pub included: bool,
    pub class: Class,
    /// What it needs first that wasn't set when suggested: `(key, value)`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<(String, Value)>,
    /// Why a plan's save left this item out, when its own check failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// A plan's line over a value the person chose themselves: it starts
    /// unchecked, with "You chose" and its value now.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub yours: bool,
    /// A plan's line that isn't ready yet: where to finish first, in
    /// English for the pane to translate.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
}

/// What the person did with a card.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum Decision {
    #[default]
    Proposed,
    Declined,
    Undone,
    Failed {
        why: String,
    },
    /// Saved: every key the save changed, rules' side effects included,
    /// before and after.
    Applied {
        before: Map<String, Value>,
        after: Map<String, Value>,
        at_ms: u64,
    },
}

/// A line the app refused: never offered as an action.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Refused {
    /// The line as written, shortened.
    pub line: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub why: Why,
}

impl Refused {
    /// `lines` the app couldn't read as change lines, each refused.
    pub fn malformed(lines: &[String]) -> impl Iterator<Item = Self> + '_ {
        lines.iter().map(|line| Self {
            line: line.clone(),
            key: None,
            why: Why::Malformed,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "why", content = "detail")]
pub enum Why {
    /// Not a change line the app could read.
    #[default]
    Malformed,
    /// No such key in this VibeHex.
    Unknown,
    /// The key exists; the value doesn't fit it.
    Invalid(String),
    /// A key a card never changes: a sign-in, a path, Assistant's own.
    Never,
}

impl Refused {
    /// What the model reads about it in a repair round or the next prompt.
    pub fn reason(&self) -> String {
        match &self.why {
            Why::Malformed => "not a JSON object with key, value, and why".into(),
            Why::Unknown => "no such setting".into(),
            Why::Invalid(why) => why.clone(),
            Why::Never => "this setting is never changed from a card".into(),
        }
    }
}

/// The cards and refusals `proposals` make against `settings`: each line
/// on its own, so one bad line drops only itself.
pub fn resolve(
    proposals: &[Proposal],
    settings: &AppSettings,
    shortcuts: &ShortcutRules,
) -> (Vec<Card>, Vec<Refused>) {
    let mut cards: Vec<Card> = Vec::new();
    let mut refused = Vec::new();
    for proposal in proposals {
        if proposal.key == vocab_edit::KEY {
            match vocab_edit::card(proposal, settings) {
                Ok(card) => match cards.last_mut() {
                    Some(edits) if edits.kind == CardKind::VocabularyEdit => {
                        vocab_edit::merge(edits, card)
                    }
                    _ => cards.push(card),
                },
                Err(why) => refused.push(Refused {
                    line: proposal.line.clone(),
                    key: Some(vocab_edit::KEY.to_owned()),
                    why,
                }),
            }
            continue;
        }
        if proposal.key == vocab::KEY {
            match vocab::card(proposal, settings) {
                // The vocabulary has it: dropped, never told the model.
                Ok(None) => {}
                Ok(Some(card)) => match cards.last_mut() {
                    Some(plan) if proposal.plan && plan.kind == CardKind::Plan => {
                        plan.items.extend(card.items)
                    }
                    Some(terms) if !proposal.plan && terms.kind == CardKind::Vocabulary => {
                        vocab::merge(terms, card)
                    }
                    _ if proposal.plan => cards.push(Card {
                        kind: CardKind::Plan,
                        ..card
                    }),
                    _ => cards.push(card),
                },
                Err(why) => refused.push(Refused {
                    line: proposal.line.clone(),
                    key: Some(vocab::KEY.to_owned()),
                    why,
                }),
            }
            continue;
        }
        match card(proposal, settings, shortcuts) {
            Ok(card) if proposal.plan => match cards.last_mut() {
                Some(plan) if plan.kind == CardKind::Plan => plan.items.extend(card.items),
                _ => cards.push(Card {
                    kind: CardKind::Plan,
                    ..card
                }),
            },
            Ok(card) => cards.push(card),
            Err(why) => refused.push(Refused {
                line: proposal.line.clone(),
                key: (!proposal.key.is_empty()).then(|| proposal.key.clone()),
                why,
            }),
        }
    }
    (cards, refused)
}

fn card(
    proposal: &Proposal,
    settings: &AppSettings,
    shortcuts: &ShortcutRules,
) -> Result<Card, Why> {
    let key = proposal.key.trim();
    let from = current(settings, key).ok_or(Why::Unknown)?;
    let mut options = Vec::new();
    for raw in &proposal.options {
        let value = names::normalize(settings, key, raw);
        if valid(settings, key, &value, shortcuts).is_ok() && !options.contains(&value) {
            options.push(value);
        }
    }
    let suggested = proposal
        .value
        .as_ref()
        .or(proposal.suggest.as_ref())
        .map(|raw| names::normalize(settings, key, raw));
    let (kind, to) = if options.len() >= 2 {
        let chosen = suggested
            .as_ref()
            .and_then(|value| options.iter().position(|option| option == value))
            .unwrap_or(0);
        let to = options[chosen].clone();
        (
            CardKind::Choice {
                options,
                chosen,
                suggested: chosen,
            },
            to,
        )
    } else {
        let to = suggested
            .or_else(|| options.pop())
            .ok_or_else(|| Why::Invalid("no value".into()))?;
        let kind = if key == parts::PROMPT {
            CardKind::Prompt
        } else {
            CardKind::Single
        };
        (kind, to)
    };
    let class = catalog::class(key, &to);
    if class == Class::Never {
        return Err(Why::Never);
    }
    valid(settings, key, &to, shortcuts)?;
    let requires = match parts::part(key, settings) {
        Some(part) => settings::missing(settings, part.parent(), &Value::Bool(true)),
        None => settings::missing(settings, key, &to),
    };
    Ok(Card {
        kind,
        items: vec![Item {
            key: key.to_owned(),
            from,
            to,
            why: proposal.why.trim().to_owned(),
            included: !proposal.optional,
            class,
            requires,
            ..Item::default()
        }],
        ..Card::default()
    })
}

/// Takes out of `cards` each one that would change nothing: a single
/// change (or the instructions) to the value the setting has now, compared
/// as `names::same` compares them, so `codex` over `codex:<that model>` is
/// no change either. Returns them as `key = value` lines, for a repair
/// round to tell the model; a plan, a choice, and terms are left alone.
pub fn already_set(cards: &mut Vec<Card>, settings: &AppSettings) -> Vec<String> {
    let mut lines = Vec::new();
    cards.retain(|card| {
        let no_change = matches!(card.kind, CardKind::Single | CardKind::Prompt)
            && !card.items.is_empty()
            && card.items.iter().all(|item| {
                let now = current(settings, &item.key).unwrap_or_else(|| item.from.clone());
                names::same(settings, &item.key, &now, &item.to)
            });
        if no_change {
            lines.extend(card.items.iter().map(|item| {
                let now = current(settings, &item.key).unwrap_or_else(|| item.from.clone());
                format!("{} = {} : already set to {now}", item.key, item.to)
            }));
        }
        !no_change
    });
    lines
}

/// `key`'s value now, as `hex settings get` shows it, or a part's; `None`
/// for a key this VibeHex doesn't have.
pub fn current(settings: &AppSettings, key: &str) -> Option<Value> {
    if let Some(part) = parts::part(key, settings) {
        return Some(part.value(settings));
    }
    settings::get(settings, key)
        .ok()?
        .into_iter()
        .find(|setting| setting.key == key)
        .map(|setting| setting.value)
}

/// Whether `value` fits `key` alone, as a save would check it. A key this
/// OS or an organization holds still makes a card, which shows why.
fn valid(
    settings: &AppSettings,
    key: &str,
    value: &Value,
    shortcuts: &ShortcutRules,
) -> Result<(), Why> {
    let values = match parts::part(key, settings) {
        Some(part) => {
            let (parent, json) = part.with(settings, value).map_err(Why::Invalid)?;
            Map::from_iter([(parent.to_owned(), json)])
        }
        None => Map::from_iter([(key.to_owned(), value.clone())]),
    };
    if values
        .keys()
        .any(|key| settings::unavailable(key).is_some())
    {
        return Ok(());
    }
    settings::check(settings, &values, shortcuts)
        .map(|_| ())
        .map_err(|error| match error.code {
            "unknown-setting" => Why::Unknown,
            "read-only" => Why::Never,
            _ => Why::Invalid(error.message),
        })
}

/// The values a card's save sends to `check`: what its items need first,
/// then the items (a part as its whole key), in that order.
pub fn values(card: &Card, settings: &AppSettings) -> Result<Map<String, Value>, String> {
    let mut values = Map::new();
    for item in card.items.iter().filter(|item| item.included) {
        for (key, value) in &item.requires {
            values.insert(key.clone(), value.clone());
        }
    }
    for item in card.items.iter().filter(|item| item.included) {
        let to = match &card.kind {
            CardKind::Choice {
                options, chosen, ..
            } => options.get(*chosen).unwrap_or(&item.to),
            _ => &item.to,
        };
        match parts::part(&item.key, settings) {
            Some(part) => {
                let (parent, json) = part.with(settings, to)?;
                values.insert(parent.to_owned(), json);
            }
            None => {
                values.insert(item.key.clone(), to.clone());
            }
        }
    }
    Ok(values)
}

/// Before and after of every key `diff` names.
pub fn applied(diff: Vec<(String, Value, Value)>, at_ms: u64) -> Decision {
    let mut before = Map::new();
    let mut after = Map::new();
    for (key, old, new) in diff {
        before.insert(key.clone(), old);
        after.insert(key, new);
    }
    Decision::Applied {
        before,
        after,
        at_ms,
    }
}

#[cfg(test)]
mod corpus;
#[cfg(test)]
mod tests;
