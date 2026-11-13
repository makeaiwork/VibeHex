//! A plan's changes counted, and the words for the count: settings and
//! Vocabulary terms apart, "3 changes and 81 terms" (owner decision
//! 2026-11-12; the preset brings 90 terms, and "4 changes" hid them). A
//! plan with no terms reads as it always did, and one with only terms
//! adds terms: "Add 81 terms".
//!
//! One and many are separate rows of the translations, each a whole line,
//! so every language orders and declines its own.

use super::{Card, Item, vocab};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// Settings changed, and terms added.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub changes: usize,
    pub terms: usize,
}

impl Counts {
    fn of<'a>(items: impl Iterator<Item = &'a Item>) -> Self {
        let mut counts = Self::default();
        for item in items {
            if item.key == vocab::KEY {
                counts.terms += 1;
            } else {
                counts.changes += 1;
            }
        }
        counts
    }

    /// Nothing of either.
    pub fn none(self) -> bool {
        self == Self::default()
    }
}

/// The rows a plan keeps as they are whose companion was picked other
/// than the value saved (`companion`): one change each, always selected.
fn kept(card: &Card) -> impl Iterator<Item = &Item> + Clone {
    card.kept
        .iter()
        .filter(|item| item.companion_changed().is_some())
}

/// What a plan holds, and what of it is selected. A row's companion is
/// part of its change; a kept row's, changed, is a change of its own.
pub fn held(card: &Card) -> (Counts, Counts) {
    (
        Counts::of(card.items.iter().chain(kept(card))),
        Counts::of(
            card.items
                .iter()
                .filter(|item| item.included)
                .chain(kept(card)),
        ),
    )
}

/// What a plan's save applied, of what was selected.
pub fn applied(card: &Card) -> (Counts, Counts) {
    let selected = || {
        card.items
            .iter()
            .filter(|item| item.included)
            .chain(kept(card))
    };
    (
        Counts::of(selected().filter(|item| item.failed.is_none())),
        Counts::of(selected()),
    )
}

/// The plan's header: "3 changes, 2 selected", "81 terms, 76 selected",
/// "3 changes and 81 terms, 2 and 76 selected".
pub fn header(all: Counts, selected: Counts) -> String {
    let (changes, terms) = (all.changes.to_string(), all.terms.to_string());
    let (one, two) = (selected.changes.to_string(), selected.terms.to_string());
    let both = |template: &str| {
        tr_named(
            template,
            &[
                ("changes", &changes),
                ("terms", &terms),
                ("one", &one),
                ("two", &two),
            ],
        )
    };
    match (all.changes, all.terms) {
        (1, 0) => tr_fill("1 change, {} selected", &one),
        (_, 0) => tr_named(
            "{all} changes, {selected} selected",
            &[("all", &changes), ("selected", &one)],
        ),
        (0, 1) => tr_fill("1 term, {} selected", &two),
        (0, _) => tr_named(
            "{all} terms, {selected} selected",
            &[("all", &terms), ("selected", &two)],
        ),
        (1, 1) => both("1 change and 1 term, {one} and {two} selected"),
        (1, _) => both("1 change and {terms} terms, {one} and {two} selected"),
        (_, 1) => both("{changes} changes and 1 term, {one} and {two} selected"),
        _ => both("{changes} changes and {terms} terms, {one} and {two} selected"),
    }
}

/// The plan's button, of what is selected: "Apply 3 changes", "Add 81
/// terms", "Apply 3 changes and 81 terms". With nothing selected it names
/// what the plan holds, and the caller turns it off.
pub fn apply(all: Counts, selected: Counts) -> String {
    let (changes, terms) = (selected.changes.to_string(), selected.terms.to_string());
    let both = |template: &str| tr_named(template, &[("changes", &changes), ("terms", &terms)]);
    let only_terms = selected.changes == 0 && (selected.terms > 0 || all.changes == 0);
    match (selected.changes, selected.terms) {
        (_, 1) if only_terms => tr("Add 1 term").to_owned(),
        (_, _) if only_terms => tr_fill("Add {} terms", &terms),
        (1, 0) => tr("Apply 1 change").to_owned(),
        (_, 0) => tr_fill("Apply {} changes", &changes),
        (1, 1) => both("Apply 1 change and 1 term"),
        (1, _) => both("Apply 1 change and {terms} terms"),
        (_, 1) => both("Apply {changes} changes and 1 term"),
        _ => both("Apply {changes} changes and {terms} terms"),
    }
}

/// How the save went: "4 of 5 changes applied", "81 of 81 terms added",
/// or both in one line.
pub fn went(done: Counts, of: Counts) -> String {
    let values = [
        ("done", done.changes.to_string()),
        ("total", of.changes.to_string()),
        ("added", done.terms.to_string()),
        ("terms", of.terms.to_string()),
    ];
    let fill = |template: &str, values: &[(&str, String)]| {
        let values: Vec<(&str, &str)> = values
            .iter()
            .map(|(name, value)| (*name, value.as_str()))
            .collect();
        tr_named(template, &values)
    };
    match (of.changes, of.terms) {
        (_, 0) => fill("{done} of {total} changes applied", &values[..2]),
        (0, _) => fill(
            "{done} of {total} terms added",
            &[
                ("done", values[2].1.clone()),
                ("total", values[3].1.clone()),
            ],
        ),
        _ => fill(
            "{done} of {total} changes applied, {added} of {terms} terms added",
            &values,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ON: (bool, bool) = (true, false);
    const OFF: (bool, bool) = (false, false);
    const FAILED: (bool, bool) = (true, true);

    /// A plan of `settings` setting rows and `terms` term rows, each as
    /// `(checked, failed)`.
    fn plan_of(settings: &[(bool, bool)], terms: &[(bool, bool)]) -> Card {
        let item = |key: &str, (included, failed): (bool, bool)| Item {
            key: key.into(),
            to: json!(true),
            included,
            failed: failed.then(|| "no".to_owned()),
            ..Item::default()
        };
        Card {
            kind: super::super::CardKind::Plan,
            items: settings
                .iter()
                .map(|row| item("labs.smart_spacing", *row))
                .chain(terms.iter().map(|row| item(vocab::KEY, *row)))
                .collect(),
            ..Card::default()
        }
    }

    const fn counts(changes: usize, terms: usize) -> Counts {
        Counts { changes, terms }
    }

    /// The owner's "3 changes and 81 terms": the header, the button and
    /// the applied line each count terms apart from settings; with no
    /// terms they read as before, with only terms they add terms.
    #[test]
    fn a_plans_terms_are_counted_apart_from_its_settings() {
        type Row<'a> = (
            &'a [(bool, bool)],
            &'a [(bool, bool)],
            &'a str,
            &'a str,
            &'a str,
        );
        let preset = [ON; 81];
        let mut some = [ON; 81];
        some[..5].fill(OFF);
        let rows: &[Row] = &[
            // The preset: three settings and 81 terms.
            (
                &[ON, ON, ON],
                &preset,
                "3 changes and 81 terms, 3 and 81 selected",
                "Apply 3 changes and 81 terms",
                "3 of 3 changes applied, 81 of 81 terms added",
            ),
            (
                &[ON, OFF, ON],
                &some,
                "3 changes and 81 terms, 2 and 76 selected",
                "Apply 2 changes and 76 terms",
                "2 of 2 changes applied, 76 of 76 terms added",
            ),
            // No terms: as it always read.
            (
                &[ON, ON],
                &[],
                "2 changes, 2 selected",
                "Apply 2 changes",
                "2 of 2 changes applied",
            ),
            (
                &[ON],
                &[],
                "1 change, 1 selected",
                "Apply 1 change",
                "1 of 1 changes applied",
            ),
            (
                &[OFF, OFF],
                &[],
                "2 changes, 0 selected",
                "Apply 0 changes",
                "0 of 0 changes applied",
            ),
            // Only terms: it adds terms.
            (
                &[],
                &preset,
                "81 terms, 81 selected",
                "Add 81 terms",
                "81 of 81 terms added",
            ),
            (
                &[],
                &[ON],
                "1 term, 1 selected",
                "Add 1 term",
                "1 of 1 terms added",
            ),
            (
                &[],
                &[OFF, OFF],
                "2 terms, 0 selected",
                "Add 0 terms",
                "0 of 0 changes applied",
            ),
            // One of either reads as one.
            (
                &[ON],
                &[ON],
                "1 change and 1 term, 1 and 1 selected",
                "Apply 1 change and 1 term",
                "1 of 1 changes applied, 1 of 1 terms added",
            ),
            (
                &[ON],
                &[ON, ON],
                "1 change and 2 terms, 1 and 2 selected",
                "Apply 1 change and 2 terms",
                "1 of 1 changes applied, 2 of 2 terms added",
            ),
            (
                &[ON, ON],
                &[ON],
                "2 changes and 1 term, 2 and 1 selected",
                "Apply 2 changes and 1 term",
                "2 of 2 changes applied, 1 of 1 terms added",
            ),
            // The button names what is selected: settings unticked leave
            // the terms to add, terms unticked leave the settings.
            (
                &[OFF, OFF],
                &[ON, ON],
                "2 changes and 2 terms, 0 and 2 selected",
                "Add 2 terms",
                "2 of 2 terms added",
            ),
            (
                &[ON, ON],
                &[OFF, ON],
                "2 changes and 2 terms, 2 and 1 selected",
                "Apply 2 changes and 1 term",
                "2 of 2 changes applied, 1 of 1 terms added",
            ),
            (
                &[ON, ON],
                &[OFF, OFF],
                "2 changes and 2 terms, 2 and 0 selected",
                "Apply 2 changes",
                "2 of 2 changes applied",
            ),
            // What failed isn't counted as done, term by term.
            (
                &[ON, FAILED],
                &[ON, FAILED, FAILED],
                "2 changes and 3 terms, 2 and 3 selected",
                "Apply 2 changes and 3 terms",
                "1 of 2 changes applied, 1 of 3 terms added",
            ),
        ];
        crate::desktop::i18n::speaking(Some("en"), || {
            for (settings, terms, head, button, line) in rows {
                let plan = plan_of(settings, terms);
                let (all, selected) = held(&plan);
                let (done, of) = applied(&plan);
                let what = format!("{settings:?} {terms:?}");
                assert_eq!(header(all, selected), *head, "{what}");
                assert_eq!(apply(all, selected), *button, "{what}");
                assert_eq!(went(done, of), *line, "{what}");
            }
        });
        let plan = plan_of(&[ON, OFF], &[ON, OFF, FAILED]);
        assert_eq!(held(&plan), (counts(2, 3), counts(1, 2)));
        assert_eq!(applied(&plan), (counts(1, 1), counts(1, 2)));
        assert!(held(&plan_of(&[OFF], &[OFF])).1.none());
    }

    /// Every count line is a row of every language's table, with its
    /// numbers kept: a language may reorder them, never drop one.
    #[test]
    fn the_counts_speak_every_language() {
        const LINES: &[&str] = &[
            "1 change, {} selected",
            "{all} changes, {selected} selected",
            "1 term, {} selected",
            "{all} terms, {selected} selected",
            "1 change and 1 term, {one} and {two} selected",
            "1 change and {terms} terms, {one} and {two} selected",
            "{changes} changes and 1 term, {one} and {two} selected",
            "{changes} changes and {terms} terms, {one} and {two} selected",
            "Apply 1 change",
            "Apply {} changes",
            "Add 1 term",
            "Add {} terms",
            "Apply 1 change and 1 term",
            "Apply 1 change and {terms} terms",
            "Apply {changes} changes and 1 term",
            "Apply {changes} changes and {terms} terms",
            "{done} of {total} changes applied",
            "{done} of {total} terms added",
            "{done} of {total} changes applied, {added} of {terms} terms added",
        ];
        for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
            crate::desktop::i18n::speaking(Some(code), || {
                for line in LINES {
                    let said = tr(line);
                    assert_ne!(said, *line, "{code}: {line}");
                    let mut rest = *line;
                    while let Some(start) = rest.find('{') {
                        let end = start + rest[start..].find('}').unwrap() + 1;
                        let slot = &rest[start..end];
                        assert!(said.contains(slot), "{code}: {line} lost {slot}");
                        rest = &rest[end..];
                    }
                }
            });
        }
    }
}
