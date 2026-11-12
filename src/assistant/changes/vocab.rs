//! Vocabulary terms an answer suggests: a card listing each term with its
//! "sounds like" spellings, added through `hex vocabulary add`'s own path
//! (`control::edit_vocabulary`).
//!
//! The model writes one JSON line per term in a `hex-vocab` block,
//! `{"term":"Kubernetes","sounds_like":["кубернетис"],"why":"…"}`, or the
//! older `{"vocabulary_add":[{"term":…}]}` shape. It never reads the
//! person's vocabulary (the settings it sees leave it out), so a term
//! already there is dropped here, in any case, and never told back to it.

use serde_json::{Map, Value, json};

use super::parse::Proposal;
use super::{Card, CardKind, Item};
use crate::assistant::offer::{self, Gate, Origin, Verdict};
use crate::dictation::vocabulary::{self, VocabularyEntry};

/// The key a vocabulary item carries: not a `hex settings` key.
pub const KEY: &str = "vocabulary";

/// One term and its spellings, as an item's `to` holds it (a plan's row
/// holds several, as an array).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Term {
    pub term: String,
    pub sounds_like: Vec<String>,
}

impl Term {
    pub fn value(&self) -> Value {
        json!({"term": self.term, "sounds_like": self.sounds_like})
    }
}

/// The terms a value holds: one object, or an array of them.
pub fn terms(value: &Value) -> Vec<Term> {
    match value {
        Value::Array(items) => items.iter().filter_map(term).collect(),
        value => term(value).into_iter().collect(),
    }
}

fn term(value: &Value) -> Option<Term> {
    let object = value.as_object()?;
    let term = object.get("term")?.as_str()?.trim().to_owned();
    Some(Term {
        term,
        sounds_like: spellings(object.get("sounds_like")),
    })
}

/// Spellings as a model writes them: a list, or one comma-separated string.
fn spellings(value: Option<&Value>) -> Vec<String> {
    let raw: Vec<String> = match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .flat_map(vocabulary::parse_sounds_like)
            .collect(),
        Some(Value::String(text)) => vocabulary::parse_sounds_like(text),
        _ => Vec::new(),
    };
    let mut kept: Vec<String> = Vec::new();
    for spelling in raw {
        if !kept
            .iter()
            .any(|known| known.to_lowercase() == spelling.to_lowercase())
        {
            kept.push(spelling);
        }
    }
    kept
}

/// Whether a change line is a vocabulary term rather than a setting.
pub fn is_term(object: &Map<String, Value>) -> bool {
    object.contains_key("term") || object.contains_key("vocabulary_add")
}

/// The proposals a term line makes: one per term.
pub fn proposals(object: &Map<String, Value>, plan: bool, line: &str) -> Option<Vec<Proposal>> {
    let why = object
        .get("why")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned();
    let optional = object.get("optional").and_then(Value::as_bool) == Some(true);
    let objects: Vec<Value> = match object.get("vocabulary_add") {
        Some(Value::Array(items)) => items.clone(),
        Some(_) => return None,
        None => vec![Value::Object(object.clone())],
    };
    let proposals: Vec<Proposal> = objects
        .iter()
        .filter_map(|value| {
            let term = term(value)?;
            let own = value.get("why").and_then(Value::as_str).map(str::trim);
            Some(Proposal {
                key: KEY.to_owned(),
                value: Some(term.value()),
                why: own.filter(|why| !why.is_empty()).unwrap_or(&why).to_owned(),
                optional,
                plan,
                line: line.to_owned(),
                ..Proposal::default()
            })
        })
        .collect();
    (!proposals.is_empty()).then_some(proposals)
}

/// A term's card, once it has passed the gate: one `hex vocabulary add`
/// takes, that Vocabulary lacks, and that is a term at all.
pub fn card(proposal: &Proposal, gate: &Gate, origin: Origin) -> Result<Card, offer::Why> {
    let term = proposal
        .value
        .as_ref()
        .and_then(term)
        .ok_or_else(|| offer::Why::Invalid("no term".into()))?;
    if let Verdict::Drop(why) = gate.term(origin, &term, proposal.own) {
        return Err(why);
    }
    // As `hex vocabulary add` keeps it.
    let mut scratch: Vec<VocabularyEntry> = Vec::new();
    vocabulary::add(&mut scratch, &term.term, &term.sounds_like).map_err(offer::Why::Invalid)?;
    let entry = scratch.remove(0);
    let kept = Term {
        term: entry.term,
        sounds_like: entry.sounds_like,
    };
    Ok(Card {
        kind: CardKind::Vocabulary,
        items: vec![Item {
            key: KEY.to_owned(),
            from: Value::Null,
            to: kept.value(),
            why: proposal.why.trim().to_owned(),
            included: !proposal.optional,
            ..Item::default()
        }],
        ..Card::default()
    })
}

/// Adds `card`'s items to `into`, a vocabulary card, leaving out a term it
/// lists already.
pub fn merge(into: &mut Card, card: Card) {
    for item in card.items {
        let term = terms(&item.to);
        let known = into.items.iter().any(|kept| {
            terms(&kept.to)
                .iter()
                .any(|one| term.iter().any(|other| same(&one.term, &other.term)))
        });
        if !known {
            into.items.push(item);
        }
    }
}

/// A plan's rows for `items`' terms: one per term, each checked or not
/// on its own, a term listed twice once.
pub fn split(items: Vec<Item>) -> Vec<Item> {
    let mut rows: Vec<Item> = Vec::new();
    for item in items {
        for term in terms(&item.to) {
            let listed = rows
                .iter()
                .any(|row| terms(&row.to).iter().any(|one| same(&one.term, &term.term)));
            if !listed {
                rows.push(Item {
                    to: term.value(),
                    ..item.clone()
                });
            }
        }
    }
    rows
}

/// The terms as a row names them: `Kubernetes, kubectl, +2`.
pub fn label(value: &Value) -> String {
    let terms = terms(value);
    let shown: Vec<&str> = terms
        .iter()
        .take(3)
        .map(|term| term.term.as_str())
        .collect();
    match terms.len().saturating_sub(3) {
        0 => shown.join(", "),
        more => format!("{}, +{more}", shown.join(", ")),
    }
}

fn same(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(value: Value) -> Proposal {
        Proposal {
            key: KEY.into(),
            value: Some(value),
            why: "misheard".into(),
            ..Proposal::default()
        }
    }

    #[test]
    fn a_term_reads_from_either_shape() {
        let line = json!({"term":"Kubernetes","sounds_like":["кубернетис","кубер"],"why":"w"});
        let made = proposals(line.as_object().unwrap(), false, "l").unwrap();
        assert_eq!(made.len(), 1);
        assert_eq!(
            terms(made[0].value.as_ref().unwrap()),
            [Term {
                term: "Kubernetes".into(),
                sounds_like: vec!["кубернетис".into(), "кубер".into()],
            }]
        );
        let old = json!({"vocabulary_add":[{"term":"kubectl","sounds_like":"куб контрол, кубкатл"},{"term":"Grafana"}],"why":"w"});
        let made = proposals(old.as_object().unwrap(), true, "l").unwrap();
        assert_eq!(made.len(), 2);
        assert!(made.iter().all(|made| made.plan && made.why == "w"));
        assert_eq!(
            terms(made[0].value.as_ref().unwrap())[0].sounds_like.len(),
            2
        );
    }

    /// The model never sees the vocabulary, so a term it has is dropped
    /// here, in any case; a bad one is refused as the CLI refuses it.
    #[test]
    fn a_known_term_is_dropped_and_a_bad_one_refused() {
        use crate::assistant::offer::Why;
        let settings = crate::app_settings::AppSettings {
            vocabulary: vec![VocabularyEntry {
                term: "Kubernetes".into(),
                sounds_like: Vec::new(),
            }],
            ..Default::default()
        };
        let state = crate::assistant::facts::State::default();
        let gate = Gate::new(&settings, &state);
        let card = |value: Value| card(&proposal(value), &gate, Origin::Answer);
        let known = card(json!({"term":"kubernetes","sounds_like":["кубер"]}));
        assert_eq!(known, Err(Why::Has));
        let long = card(json!({"term": "x".repeat(80)}));
        assert!(matches!(long, Err(Why::Invalid(_))), "{long:?}");
        assert!(matches!(card(json!({"term":"  "})), Err(Why::Invalid(_))));
        let new =
            card(json!({"term":"Grafana","sounds_like":["графана","Графана","Grafana"]})).unwrap();
        assert_eq!(new.kind, CardKind::Vocabulary);
        assert_eq!(terms(&new.items[0].to)[0].sounds_like, ["графана"]);
    }

    #[test]
    fn a_plan_holds_each_term_on_its_own_row() {
        let item = |value: Value, included| Item {
            key: KEY.into(),
            to: value,
            included,
            ..Item::default()
        };
        let rows = split(vec![
            item(json!({"term": "GitHub", "sounds_like": ["гитхаб"]}), true),
            item(
                json!([{"term": "Docker"}, {"term": "github"}, {"term": "TypeScript"}]),
                false,
            ),
        ]);
        let shown: Vec<(String, bool)> = rows
            .iter()
            .map(|row| (label(&row.to), row.included))
            .collect();
        assert_eq!(
            shown,
            [
                ("GitHub".to_owned(), true),
                ("Docker".to_owned(), false),
                ("TypeScript".to_owned(), false),
            ]
        );
        assert_eq!(terms(&rows[0].to)[0].sounds_like, ["гитхаб"]);
        assert_eq!(
            label(&json!([{"term": "a"}, {"term": "b"}, {"term": "c"}, {"term": "d"}])),
            "a, b, c, +1"
        );
    }
}
