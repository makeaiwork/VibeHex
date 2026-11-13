//! Setting changes an answer suggests, as cards the person decides on.
//!
//! The model writes plain text with fenced `hex-change` lines (`parse`),
//! naming only a key, a value, and why. The app does the rest: whether the
//! key exists and may be changed from a card (`catalog`), what it is called
//! in the window (`names`), its current value, what it needs first
//! (`control::settings::requires`), and the narrow parts of two JSON keys
//! (`parts`). Whether a line becomes a card at all is the gate's
//! (`assistant::offer`), the one way anything is offered. Nothing is saved until the person presses the card's button,
//! and then only through `control::settings::check`, the path `hex settings`
//! takes. `state` derives what a card shows each frame.

pub mod built_in;
pub mod catalog;
pub mod companion;
pub mod counts;
pub mod names;
pub mod parse;
pub mod parts;
pub mod shortcuts;
pub mod state;
pub mod vocab;
pub mod vocab_edit;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::app_settings::AppSettings;
use crate::assistant::offer::{self, Dropped, Gate, Offer, Origin, Verdict};
use crate::control::settings;
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
    /// Shortcuts, each recorded in its row; the changed ones saved
    /// together (`shortcuts`).
    Shortcuts,
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
    /// The value that goes with a switch turned on (`companion`), picked
    /// in the row and saved with it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub companion: Option<companion::Companion>,
}

impl Item {
    /// Its companion, when picked other than the value saved now.
    pub fn companion_changed(&self) -> Option<&companion::Companion> {
        self.companion
            .as_ref()
            .filter(|companion| companion.changed())
    }
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

/// What `proposals` come to once each has passed the gate.
#[derive(Debug, Default)]
pub struct Resolved {
    pub cards: Vec<Card>,
    /// Lines the model got wrong (no such key, a value a save refuses, a
    /// key never changed from a card): shown muted, and told to it.
    pub refused: Vec<Refused>,
    /// Lines dropped for any other reason, for the turn's record.
    pub dropped: Vec<Dropped>,
}

impl Resolved {
    /// The settings dropped because they have that value now, as
    /// `key = value` lines for a round that tells the model.
    pub fn already(&self, settings: &AppSettings) -> Vec<String> {
        self.told(settings, |why| *why == offer::Why::Same)
    }

    /// The settings dropped because this computer can't have them.
    pub fn not_here(&self, settings: &AppSettings) -> Vec<String> {
        self.told(settings, |why| {
            matches!(why, offer::Why::Elsewhere(_) | offer::Why::Hardware)
        })
    }

    fn told(&self, settings: &AppSettings, which: impl Fn(&offer::Why) -> bool) -> Vec<String> {
        self.dropped
            .iter()
            .filter(|dropped| which(&dropped.why))
            .map(|dropped| match &dropped.why {
                offer::Why::Same => {
                    let key = dropped.what.split(" = ").next().unwrap_or_default();
                    let now = current(settings, key).unwrap_or(Value::Null);
                    format!("{} : already set to {now}", dropped.what)
                }
                why => format!("{} : {}", dropped.what, why.words()),
            })
            .collect()
    }
}

/// The cards `proposals` make once each line has passed `gate` as coming
/// from `origin`: each line on its own, so one bad line drops only itself.
pub fn resolve(proposals: &[Proposal], gate: &Gate, origin: Origin) -> Resolved {
    let mut resolved = Resolved::default();
    for proposal in proposals {
        let made = if proposal.key == vocab_edit::KEY {
            match gate.edit(origin) {
                Verdict::Drop(why) => Err(why),
                _ => vocab_edit::card(proposal, gate.settings).map_err(|why| match why {
                    Why::Invalid(why) => offer::Why::Invalid(why),
                    Why::Unknown | Why::Malformed => offer::Why::Unknown,
                    Why::Never => offer::Why::Never,
                }),
            }
        } else if proposal.key == vocab::KEY {
            vocab::card(proposal, gate, origin)
        } else {
            card(proposal, gate, origin)
        };
        let card = match made {
            Ok(card) => card,
            Err(why) => {
                let key = proposal.key.trim();
                match why.refusal() {
                    Some(why) => resolved.refused.push(Refused {
                        line: proposal.line.clone(),
                        key: (!key.is_empty()).then(|| key.to_owned()),
                        why,
                    }),
                    None => resolved.dropped.push(Dropped {
                        what: proposal.what(),
                        why,
                    }),
                }
                continue;
            }
        };
        let cards = &mut resolved.cards;
        match (card.kind.clone(), cards.last_mut()) {
            (CardKind::VocabularyEdit, Some(edits)) if edits.kind == CardKind::VocabularyEdit => {
                vocab_edit::merge(edits, card)
            }
            (CardKind::VocabularyEdit, _) => cards.push(card),
            // A plan is one list, its terms rows of it.
            (_, Some(plan)) if proposal.plan && plan.kind == CardKind::Plan => {
                plan.items.extend(card.items)
            }
            (_, _) if proposal.plan => cards.push(Card {
                kind: CardKind::Plan,
                ..card
            }),
            (CardKind::Vocabulary, Some(terms)) if terms.kind == CardKind::Vocabulary => {
                vocab::merge(terms, card)
            }
            _ => cards.push(card),
        }
    }
    // Cleanup on this computer and its model are one row.
    built_in::fold_cards(&mut resolved.cards, &gate.state.machine);
    resolved
}

/// The card one setting line makes, or why the gate drops it.
fn card(proposal: &Proposal, gate: &Gate, origin: Origin) -> Result<Card, offer::Why> {
    let settings = gate.settings;
    let key = proposal.key.trim();
    let from = current(settings, key).ok_or(offer::Why::Unknown)?;
    let mut options = Vec::new();
    for raw in &proposal.options {
        let value = names::normalize(settings, key, raw);
        if gate.allowed(origin, key, &value).is_ok() && !options.contains(&value) {
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
            .ok_or_else(|| offer::Why::Invalid("no value".into()))?;
        let kind = if key == parts::PROMPT {
            CardKind::Prompt
        } else {
            CardKind::Single
        };
        (kind, to)
    };
    let mut item = Item {
        key: key.to_owned(),
        from,
        why: proposal.why.trim().to_owned(),
        included: !proposal.optional,
        class: catalog::class(key, &to),
        requires: gate.requires(key, &to),
        companion: match kind {
            CardKind::Single => companion::of(settings, key, &to)
                .or_else(|| built_in::companion(settings, &gate.state.machine, key, &to)),
            _ => None,
        },
        ..Item::default()
    };
    // A choice shows its options whatever the value is now.
    let verdict = match kind {
        CardKind::Choice { .. } => match gate.allowed(origin, key, &to) {
            Ok(_) => Verdict::Show,
            Err(why) => Verdict::Drop(why),
        },
        _ => gate.setting(
            origin,
            Offer {
                key,
                to: &to,
                own: proposal.own,
                note: proposal.note,
            },
        ),
    };
    match verdict {
        Verdict::Drop(why) => return Err(why),
        Verdict::Unchecked(held) => Gate::start(&mut item, &held),
        Verdict::Show => {}
    }
    item.to = to;
    Ok(Card {
        kind,
        items: vec![item],
        ..Card::default()
    })
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

/// The values a card's save sends to `check`: what its items need first,
/// then the items (a part as its whole key) with their companions, then a
/// plan's companions changed on a row it keeps, in that order.
pub fn values(card: &Card, settings: &AppSettings) -> Result<Map<String, Value>, String> {
    let mut values = Map::new();
    for item in card.items.iter().filter(|item| item.included) {
        for (key, value) in &item.requires {
            values.insert(key.clone(), value.clone());
        }
    }
    for item in card.kept.iter().filter_map(Item::companion_changed) {
        values.insert(item.key.clone(), item.to().clone());
    }
    // Parts of one key build on each other: three apps' choices, or the
    // Global mode's switch and its instructions, all land in one save.
    let mut working = settings.clone();
    for item in card.items.iter().filter(|item| item.included) {
        let to = match &card.kind {
            CardKind::Choice {
                options, chosen, ..
            } => options.get(*chosen).unwrap_or(&item.to),
            _ => &item.to,
        };
        match parts::part(&item.key, settings) {
            Some(part) => {
                let (parent, json) = part.with(&working, to)?;
                working = with_value(&working, parent, &json)?;
                values.insert(parent.to_owned(), json);
            }
            None => {
                values.insert(item.key.clone(), to.clone());
            }
        }
        if let Some(companion) = &item.companion {
            values.insert(companion.key.clone(), companion.to().clone());
        }
    }
    Ok(values)
}

/// `settings` with the JSON key `key` (dotted, as `hex settings` names it)
/// set to `value`.
fn with_value(settings: &AppSettings, key: &str, value: &Value) -> Result<AppSettings, String> {
    let mut json = serde_json::to_value(settings).map_err(|error| error.to_string())?;
    let pointer = format!("/{}", key.replace('.', "/"));
    let slot = json
        .pointer_mut(&pointer)
        .ok_or_else(|| format!("`{key}` is missing"))?;
    *slot = value.clone();
    serde_json::from_value(json).map_err(|error| error.to_string())
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
