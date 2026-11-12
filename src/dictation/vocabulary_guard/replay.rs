//! A local replay of the guard over a person's own History. Nothing it
//! reads is ever committed: the paths come from the environment and the
//! findings go to stderr.

use std::collections::HashMap;
use std::time::Instant;

use super::{Guard, Note, Verdict};
use crate::dictation::vocabulary::VocabularyEntry;

/// The saved vocabulary (the shared list, then each mode's own terms),
/// read straight from a settings.json.
fn saved_vocabulary(settings: &serde_json::Value) -> Vec<VocabularyEntry> {
    let lists = std::iter::once(&settings["vocabulary"]).chain(
        settings["dictation_processing"]["modes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|mode| &mode["vocabulary"]),
    );
    let mut vocabulary: Vec<VocabularyEntry> = Vec::new();
    for entry in lists.filter_map(|list| list.as_array()).flatten() {
        let (term, sounds_like) = match entry {
            serde_json::Value::String(term) => (term.clone(), Vec::new()),
            entry => (
                entry["term"].as_str().unwrap_or_default().to_owned(),
                entry["sounds_like"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|spelling| spelling.as_str().map(str::to_owned))
                    .collect(),
            ),
        };
        if !term.is_empty() && !vocabulary.iter().any(|known| known.term == term) {
            vocabulary.push(VocabularyEntry { term, sounds_like });
        }
    }
    vocabulary
}

/// Runs the guard over every dictation cleanup changed and prints how many
/// of the terms cleanup wrote it kept and undid, with samples of each:
/// `HEX_USAGE_HISTORY=…/history.json HEX_USAGE_SETTINGS=…/settings.json
/// scripts/check --run-ignored only replays_a_history -- --nocapture`
/// (`HEX_USAGE_SAMPLES` sets how many of each verdict to print, 20 unless
/// set).
#[test]
#[ignore = "reads a real History named by HEX_USAGE_HISTORY"]
fn replays_a_history() {
    let read = |name: &str| -> serde_json::Value {
        let path = std::env::var(name).unwrap_or_else(|_| panic!("set {name}"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    };
    let history = read("HEX_USAGE_HISTORY");
    let vocabulary = saved_vocabulary(&read("HEX_USAGE_SETTINGS"));
    let samples: usize = std::env::var("HEX_USAGE_SAMPLES")
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(20);
    let guard = Guard::new(&vocabulary);
    let entries = history["entries"].as_array().unwrap();
    let (mut dictations, mut changed, mut touched) = (0, 0, 0);
    let (mut as_heard, mut spelled_exact) = (0, 0);
    let mut notes: HashMap<Verdict, Vec<Note>> = HashMap::new();
    let (mut slowest, mut total, mut words) = (0_u128, 0_u128, 0);
    // Every exact spelling the code writes, and those where cleanup had
    // not written the term itself.
    let (mut spelled_all, mut spelled_alone) = (0, Vec::new());
    for entry in entries {
        let (Some(raw), Some(cleaned)) = (entry["raw_text"].as_str(), entry["final_text"].as_str())
        else {
            continue;
        };
        if entry["kind"].as_str() != Some("dictation") {
            continue;
        }
        dictations += 1;
        if raw == cleaned {
            continue;
        }
        changed += 1;
        // Against the bare transcript, to count every term cleanup wrote
        // that it lacked.
        let bare = guard.check(raw, cleaned);
        as_heard += bare
            .notes
            .iter()
            .filter(|note| note.verdict != Verdict::Restored)
            .count();
        let started = Instant::now();
        let spelled = guard.spell(raw);
        let checked = guard.check(&spelled, cleaned);
        let took = started.elapsed().as_micros();
        slowest = slowest.max(took);
        total += took;
        words += raw.split_whitespace().count();
        spelled_exact += cleaned_terms(&bare).saturating_sub(cleaned_terms(&checked));
        let cleaned_lower = cleaned.to_lowercase();
        for (range, term) in guard.spellings.matches(
            raw,
            |term| guard.terms[term].text.as_str(),
            |term| guard.terms[term].letters.as_str(),
        ) {
            let term = &guard.terms[term].text;
            spelled_all += 1;
            if !cleaned_lower.contains(&term.to_lowercase()) {
                spelled_alone.push(format!("{:?} -> {term}", &raw[range]));
            }
        }
        if checked.text != cleaned {
            touched += 1;
        }
        for note in &checked.notes {
            if matches!(note.verdict, Verdict::Moved | Verdict::Restored)
                || (note.verdict == Verdict::Undone
                    && std::env::var("HEX_USAGE_SHOW")
                        .is_ok_and(|shown| shown.split(',').any(|term| term == note.term)))
            {
                eprintln!(
                    "{:?} {:?} for {:?}:\n  heard {spelled}\n  wrote {cleaned}\n  now   {}",
                    note.verdict, note.term, note.heard, checked.text
                );
            }
            notes.entry(note.verdict).or_default().push(note.clone());
        }
    }
    let count = |verdict| notes.get(&verdict).map_or(0, Vec::len);
    eprintln!(
        "{dictations} dictations, {changed} changed by cleanup, {touched} changed by the guard\n\
         {as_heard} terms written that the transcript lacked; {spelled_exact} are exact listed \
         spellings (now written in code)\n\
         kept {} (moved {}), undone {}, unjudged {}, stretches with a word put back {}\n\
         spell + check: {} µs a dictation on average ({} words), {slowest} µs the slowest",
        count(Verdict::Kept) + count(Verdict::Moved),
        count(Verdict::Moved),
        count(Verdict::Undone),
        count(Verdict::Unjudged),
        count(Verdict::Restored),
        total / changed.max(1) as u128,
        words / changed.max(1),
    );
    eprintln!(
        "\n{spelled_all} exact spellings written in code; in {} cleanup had not written the term:",
        spelled_alone.len()
    );
    for line in spelled_alone.iter().take(samples) {
        eprintln!("{line}");
    }
    for verdict in [
        Verdict::Undone,
        Verdict::Restored,
        Verdict::Unjudged,
        Verdict::Moved,
        Verdict::Kept,
    ] {
        let found = notes.get(&verdict).map_or(&[][..], Vec::as_slice);
        eprintln!("\n== {verdict:?}: {} ==", found.len());
        // Spread the sample over the whole History.
        let step = (found.len() / samples.max(1)).max(1);
        for note in found.iter().step_by(step).take(samples) {
            match &note.now {
                Some(now) => eprintln!(
                    "{:?} -> {:?} [{}], now {now:?} (closest {:.2})",
                    note.heard,
                    note.wrote,
                    note.term,
                    closest(&guard, note)
                ),
                None => eprintln!(
                    "{:?} -> {:?} [{}] (closest {:.2})",
                    note.heard,
                    note.wrote,
                    note.term,
                    closest(&guard, note)
                ),
            }
        }
    }
}

/// How many terms a check judged as written by cleanup.
fn cleaned_terms(checked: &super::Checked) -> usize {
    checked
        .notes
        .iter()
        .filter(|note| note.verdict != Verdict::Restored)
        .count()
}

/// How alike the note's term and the closest run of its heard words sound,
/// whatever the word-by-word rule says: where the threshold sits.
fn closest(guard: &Guard, note: &Note) -> f32 {
    let Some(term) = guard.terms.iter().find(|term| term.text == note.term) else {
        return 0.0;
    };
    let heard = super::words(&note.heard);
    let mut table = Vec::new();
    let mut best = 0.0_f32;
    for first in 0..heard.len() {
        for end in first + 1..=heard.len().min(first + super::stretch::MAX_RUN_WORDS) {
            let key = super::Key::of(heard[first..end].iter().map(|word| word.core), true);
            for form in &term.forms {
                best = best.max(super::sound::fit(&key, form, &mut table).similarity);
            }
        }
    }
    best
}
