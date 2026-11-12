//! An existing Vocabulary entry an answer changes: a "sounds like" spelling
//! that catches a word it shouldn't ("AI cleanup" sounding like «ай» writes
//! AI cleanup for a plain AI), or a term spelled wrong. A card shows each
//! entry before → after and saves through `hex vocabulary edit`'s own path
//! (`control::edit_vocabulary`), the entry rewritten where it stands; Undo
//! writes it back exactly as it was.
//!
//! The model writes one JSON line per entry in a `hex-vocab` block:
//! `{"term":"AI cleanup","remove_sounds_like":["ай"],"why":"…"}`, with
//! `add_sounds_like` and `rename` as it needs. It sees only the entries in
//! `<vocabulary>`; a term that isn't one, or a spelling the entry doesn't
//! have, is refused, so the repair round tells it.

use serde_json::{Map, Value, json};

use super::parse::Proposal;
use super::vocab::Term;
use super::{Card, CardKind, Item, Why};
use crate::app_settings::AppSettings;
use crate::dictation::vocabulary::{self, VocabularyEntry};

/// The key an edit's proposal and items carry: not a `hex settings` key.
pub const KEY: &str = "vocabulary.edit";

/// The fields that make a `hex-vocab` line an edit of an entry.
const FIELDS: [&str; 3] = ["remove_sounds_like", "add_sounds_like", "rename"];

/// Whether a `hex-vocab` line edits an entry rather than adds a term.
pub fn is_edit(object: &Map<String, Value>) -> bool {
    object.contains_key("term") && FIELDS.iter().any(|field| object.contains_key(*field))
}

/// The proposal an edit line makes.
pub fn proposal(object: &Map<String, Value>, line: &str) -> Proposal {
    Proposal {
        key: KEY.to_owned(),
        value: Some(Value::Object(object.clone())),
        why: object
            .get("why")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned(),
        line: line.to_owned(),
        ..Proposal::default()
    }
}

/// Spellings as written: a list, or one comma-separated string.
fn spellings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .flat_map(vocabulary::parse_sounds_like)
            .collect(),
        Some(Value::String(text)) => vocabulary::parse_sounds_like(text),
        _ => Vec::new(),
    }
}

fn same(one: &str, other: &str) -> bool {
    one.trim().to_lowercase() == other.trim().to_lowercase()
}

/// An entry as an item holds it.
pub fn entry(value: &Value) -> Option<Term> {
    let object = value.as_object()?;
    Some(Term {
        term: object.get("term")?.as_str()?.to_owned(),
        sounds_like: spellings(object.get("sounds_like")),
    })
}

/// The card for an edit of an entry in `settings`' vocabulary, or why not:
/// no such entry, a spelling it doesn't have, or nothing that changes.
pub fn card(proposal: &Proposal, settings: &AppSettings) -> Result<Card, Why> {
    let object = proposal
        .value
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| Why::Invalid("not an edit".into()))?;
    let term = object
        .get("term")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let Some(now) = settings
        .vocabulary
        .iter()
        .find(|entry| same(&entry.term, term))
    else {
        return Err(Why::Invalid(format!(
            "“{term}” isn't in their vocabulary: edit only an entry <vocabulary> lists"
        )));
    };
    let remove = spellings(object.get("remove_sounds_like"));
    if let Some(missing) = remove
        .iter()
        .find(|spelling| !now.sounds_like.iter().any(|kept| same(kept, spelling)))
    {
        return Err(Why::Invalid(format!(
            "“{}” has no spelling “{missing}”; its spellings: {}",
            now.term,
            now.sounds_like.join(", ")
        )));
    }
    let mut sounds_like: Vec<String> = now
        .sounds_like
        .iter()
        .filter(|kept| !remove.iter().any(|gone| same(gone, kept)))
        .cloned()
        .collect();
    sounds_like.extend(spellings(object.get("add_sounds_like")));
    let rename = object
        .get("rename")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty());
    // The check the save makes, on a copy.
    let mut scratch: Vec<VocabularyEntry> = settings.vocabulary.clone();
    match vocabulary::edit(&mut scratch, &now.term, rename, &sounds_like) {
        Ok(true) => {}
        Ok(false) => return Err(Why::Invalid(format!("“{}” is so already", now.term))),
        Err(why) => return Err(Why::Invalid(why)),
    }
    let after = scratch
        .iter()
        .find(|entry| same(&entry.term, rename.unwrap_or(&now.term)))
        .cloned()
        .unwrap_or_default();
    Ok(Card {
        kind: CardKind::VocabularyEdit,
        items: vec![Item {
            key: KEY.to_owned(),
            from: value(now),
            to: value(&after),
            why: proposal.why.clone(),
            included: true,
            ..Item::default()
        }],
        ..Card::default()
    })
}

pub fn value(entry: &VocabularyEntry) -> Value {
    json!({"term": entry.term, "sounds_like": entry.sounds_like})
}

/// Adds `card`'s entries to `into`, an edit card, leaving out an entry it
/// edits already.
pub fn merge(into: &mut Card, card: Card) {
    for item in card.items {
        let term = entry(&item.from).unwrap_or_default().term;
        let known = into
            .items
            .iter()
            .any(|kept| entry(&kept.from).is_some_and(|kept| same(&kept.term, &term)));
        if !known {
            into.items.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AppSettings {
        AppSettings {
            vocabulary: vec![
                VocabularyEntry {
                    term: "AI cleanup".into(),
                    sounds_like: vec!["эй ай клинап".into(), "ай".into()],
                },
                VocabularyEntry {
                    term: "Kubernetes".into(),
                    sounds_like: vec!["кубер".into()],
                },
            ],
            ..AppSettings::default()
        }
    }

    fn line(value: Value) -> Proposal {
        proposal(value.as_object().unwrap(), &value.to_string())
    }

    #[test]
    fn an_edit_shows_the_entry_before_and_after() {
        let edit = json!({"term":"ai cleanup","remove_sounds_like":["Ай"],"add_sounds_like":"клинап","why":"w"});
        assert!(is_edit(edit.as_object().unwrap()));
        assert!(!is_edit(json!({"term":"Grafana"}).as_object().unwrap()));
        let card = card(&line(edit), &settings()).unwrap();
        assert_eq!(card.kind, CardKind::VocabularyEdit);
        assert_eq!(
            card.items[0].from,
            json!({"term":"AI cleanup","sounds_like":["эй ай клинап","ай"]})
        );
        assert_eq!(
            card.items[0].to,
            json!({"term":"AI cleanup","sounds_like":["эй ай клинап","клинап"]})
        );
        let renamed = card_of(json!({"term":"Kubernetes","rename":"K8s"}));
        assert_eq!(renamed.items[0].to["term"], "K8s");
    }

    fn card_of(value: Value) -> Card {
        card(&line(value), &settings()).unwrap()
    }

    /// Only an entry that exists, a spelling it has, and a real change.
    #[test]
    fn an_edit_of_what_isnt_there_is_refused() {
        let refused = |value: Value| card(&line(value), &settings()).unwrap_err();
        assert!(matches!(
            refused(json!({"term":"Grafana","remove_sounds_like":["графана"]})),
            Why::Invalid(why) if why.contains("isn't in their vocabulary")
        ));
        assert!(matches!(
            refused(json!({"term":"AI cleanup","remove_sounds_like":["эй"]})),
            Why::Invalid(why) if why.contains("no spelling “эй”")
        ));
        assert!(matches!(
            refused(json!({"term":"AI cleanup","add_sounds_like":["ай"]})),
            Why::Invalid(why) if why.contains("so already")
        ));
        assert!(matches!(
            refused(json!({"term":"AI cleanup","rename":"kubernetes"})),
            Why::Invalid(_)
        ));
    }
}
