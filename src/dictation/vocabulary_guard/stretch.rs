//! One stretch cleanup changed, lined up word by word: which heard words
//! each term in it stands for, which heard words cleanup kept in another
//! spelling, which it dropped, and which words it added of its own.
//!
//! The stretch's heard words are matched in order against what cleanup
//! wrote there (its words, and each term as one unit), at the least cost: a
//! term takes the run of heard words that supports it, a written word takes
//! the heard word it sounds like, and everything else is dropped or added.
//! Between the words that matched lie the gaps the guard decides on.

use std::ops::Range;

use super::align::Word;
use super::sound::{Fit, Key, fit, fit_bound};
use super::{Guard, Mention, Term, plain_words};

/// How alike a heard run and a form must sound for the form's term to
/// stand for the run.
const SUPPORT: f32 = 0.7;
/// A pair is short when neither side weighs this much (a consonant weighs
/// 1, a vowel half). In a short pair one consonant is another word ("код"
/// is not "клод"), so a short pair must also have every consonant right.
const SHORT_WEIGHT: f32 = 4.0;
/// The most of any one word, heard or of the form, that may go unmatched:
/// "barn" is not "Night" however well "shift" fits, and "Auto" must be heard
/// for "Auto sync".
const WORD_MISS: f32 = 0.5;
/// How alike a heard word and a written word must sound to be the same
/// word, kept.
const SAME_WORD: f32 = 0.6;
/// A dropped word is a piece of a term (a stutter, a first try at it) when
/// it has every consonant of one of the term's words, or of its start.
/// The fewest sounds a dropped word needs to count as the start of a term.
const MIN_START_SOUNDS: usize = 3;
/// The most heard words one term stands for ("чат джи пи ти").
pub(super) const MAX_RUN_WORDS: usize = 5;
/// The longest stretch lined up, in words on either side; a longer one was
/// rewritten, not cleaned, and is left alone.
const MAX_STRETCH_WORDS: usize = 24;

/// What a term nothing heard supports costs the line-up: more than
/// dropping a heard word or two, so a supported reading always wins.
const BARE_TERM: f32 = 2.0;
/// What it costs to read a term as the shorter term inside it ("cleanup"
/// heard, "AI cleanup" written): less than a bare term, more than a term
/// heard whole.
const SHORTER_TERM: f32 = 0.9;

/// A run of the cleaned text between two kept words, with a term in it.
#[derive(Clone, Debug)]
pub(super) struct Stretch {
    /// The heard words between the same two kept words.
    pub heard: Range<usize>,
    pub cleaned: Range<usize>,
    /// Its terms, as indices into the open mentions.
    pub mentions: Range<usize>,
}

/// A part of a stretch between two heard words cleanup kept, with a term
/// in it: the unit the guard keeps, undoes, or mends.
#[derive(Clone, Debug)]
pub(super) struct Gap {
    pub heard: Range<usize>,
    pub cleaned: Range<usize>,
    pub terms: Vec<Placed>,
    /// Heard words nothing written stands for.
    pub dropped: Vec<usize>,
    /// How many words cleanup added that stand for nothing heard.
    pub strays: usize,
}

/// A term in a gap and the heard words it stands for.
#[derive(Clone, Debug)]
pub(super) struct Placed {
    /// Index into the open mentions.
    pub mention: usize,
    /// The heard words that support it; `None` when none do.
    pub run: Option<Range<usize>>,
    /// The transcript already had the term there.
    pub existing: bool,
    /// Its heard words were found elsewhere: cleanup moved it.
    pub moved: bool,
    /// The run supports only a shorter term inside this one: the words of
    /// the cleaned text that are that term.
    pub shorter: Option<Range<usize>>,
}

impl Placed {
    /// Whether what was heard supports the whole term.
    pub(super) fn supported(&self) -> bool {
        (self.run.is_some() && self.shorter.is_none()) || self.moved
    }
}

/// Reused tables.
#[derive(Default)]
pub(super) struct Scratch {
    sounds: Vec<f32>,
}

enum Unit {
    /// A word of the cleaned text.
    Word(usize),
    /// A term: an index into the open mentions.
    Term(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum Step {
    End,
    /// A heard word with nothing written for it.
    Dropped,
    /// A written word with nothing heard for it.
    Added,
    /// A heard word kept, in whatever spelling.
    Same,
    /// A term with no heard words.
    Bare,
    /// A term for this many heard words; whether they are the term itself.
    Run(u8, bool),
    /// This many heard words for the shorter term inside the term, by its
    /// place in the term's `inner`.
    Shorter(u8, u8),
}

/// Whether `run` supports `term`: how alike they sound, and whether the
/// run is the term itself, already written.
pub(super) fn support(term: &Term, run: &[Word<'_>], scratch: &mut Scratch) -> Option<(f32, bool)> {
    let key = Key::of(run.iter().map(|word| word.core), true);
    supported_by(term, run, &key, scratch)
}

fn supported_by(
    term: &Term,
    run: &[Word<'_>],
    key: &Key,
    scratch: &mut Scratch,
) -> Option<(f32, bool)> {
    let (first, last) = (run.first()?, run.last()?);
    if first.norm.is_empty() || last.norm.is_empty() {
        return None;
    }
    // The term itself, already written, with whatever hangs on its last
    // word ("MCP-сервера").
    if run.len() == term.words.len()
        && run
            .iter()
            .zip(&term.words)
            .enumerate()
            .all(|(place, (a, b))| {
                a.norm == *b
                    || (place + 1 == run.len() && super::uninflected(&a.norm) == Some(b.as_str()))
            })
    {
        return Some((1.0, true));
    }
    term.forms
        .iter()
        .filter(|form| fit_bound(key, form) >= SUPPORT)
        .filter_map(|form| {
            let Fit {
                consonant_off,
                similarity,
                heard_miss,
                form_miss,
            } = fit(key, form, &mut scratch.sounds);
            let short = key.weight().max(form.weight()) < SHORT_WEIGHT;
            (similarity >= SUPPORT
                && heard_miss <= WORD_MISS
                && form_miss <= WORD_MISS
                && !(short && consonant_off))
                .then_some(similarity)
        })
        .fold(None, |best: Option<f32>, similarity| {
            Some(best.map_or(similarity, |best| best.max(similarity)))
        })
        .map(|similarity| (similarity, false))
}

/// Whether a dropped heard word is a piece of `term` (one of its words, a
/// stutter or first try at it), which cleanup may drop beside the term.
pub(super) fn part_of(term: &Term, word: &Word<'_>, scratch: &mut Scratch) -> bool {
    if term.words.contains(&word.norm) {
        return true;
    }
    let key = Key::of([word.core], true);
    let near = |piece: &Key, scratch: &mut Scratch| {
        fit_bound(&key, piece) >= SAME_WORD && !fit(&key, piece, &mut scratch.sounds).consonant_off
    };
    term.pieces.iter().any(|piece| near(piece, scratch))
        || (key.len() >= MIN_START_SOUNDS
            && term
                .forms
                .iter()
                .any(|form| form.len() > key.len() && near(&form.start(key.len()), scratch)))
}

impl Stretch {
    /// The stretch's gaps that have a term; `None` when it is too long to
    /// line up.
    pub(super) fn line_up(
        &self,
        guard: &Guard,
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
        scratch: &mut Scratch,
    ) -> Option<Vec<Gap>> {
        let mut units = Vec::new();
        let mut at = self.cleaned.start;
        let mut mention = self.mentions.start;
        while at < self.cleaned.end {
            if mention < self.mentions.end && open[mention].from == at {
                units.push(Unit::Term(mention));
                at = open[mention].to;
                mention += 1;
            } else {
                units.push(Unit::Word(at));
                at += 1;
            }
        }
        let said = &heard[self.heard.clone()];
        let (rows, columns) = (said.len(), units.len());
        if rows > MAX_STRETCH_WORDS || columns > MAX_STRETCH_WORDS {
            return None;
        }
        let said_keys: Vec<Key> = said.iter().map(|word| Key::of([word.core], true)).collect();
        let run_keys: Vec<Key> = (0..rows)
            .flat_map(|first| {
                (1..=MAX_RUN_WORDS).map(move |count| (first, (first + count).min(rows)))
            })
            .map(|(first, end)| Key::of(said[first..end].iter().map(|word| word.core), true))
            .collect();
        let width = columns + 1;
        let mut costs = vec![0.0_f32; (rows + 1) * width];
        let mut steps = vec![Step::End; (rows + 1) * width];
        for row in (0..=rows).rev() {
            if row < rows {
                costs[row * width + columns] =
                    drop_cost(&said[row]) + costs[(row + 1) * width + columns];
                steps[row * width + columns] = Step::Dropped;
            }
            for (column, unit) in units.iter().enumerate().rev() {
                let mut best = (f32::INFINITY, Step::End);
                let mut consider = |cost: f32, step: Step| {
                    if cost < best.0 {
                        best = (cost, step);
                    }
                };
                match unit {
                    Unit::Term(mention) => {
                        let term = &guard.terms[open[*mention].term];
                        for count in 1..=MAX_RUN_WORDS.min(rows - row) {
                            let run = &said[row..row + count];
                            let key = &run_keys[row * MAX_RUN_WORDS + count - 1];
                            if let Some((similarity, existing)) =
                                supported_by(term, run, key, scratch)
                            {
                                consider(
                                    (1.0 - similarity) * 0.5
                                        + costs[(row + count) * width + column + 1],
                                    Step::Run(count as u8, existing),
                                );
                            }
                        }
                        for (place, (inner, _)) in term.inner.iter().enumerate() {
                            for count in 1..=MAX_RUN_WORDS.min(rows - row) {
                                let run = &said[row..row + count];
                                let key = &run_keys[row * MAX_RUN_WORDS + count - 1];
                                if let Some((similarity, _)) =
                                    supported_by(&guard.terms[*inner], run, key, scratch)
                                {
                                    consider(
                                        SHORTER_TERM
                                            + (1.0 - similarity) * 0.5
                                            + costs[(row + count) * width + column + 1],
                                        Step::Shorter(count as u8, place as u8),
                                    );
                                }
                            }
                        }
                        if row < rows {
                            consider(
                                drop_cost(&said[row]) + costs[(row + 1) * width + column],
                                Step::Dropped,
                            );
                        }
                        consider(BARE_TERM + costs[row * width + column + 1], Step::Bare);
                    }
                    Unit::Word(index) => {
                        let wrote = &cleaned[*index];
                        if row < rows {
                            let alike = same_word(&said[row], &said_keys[row], wrote, scratch);
                            if alike >= SAME_WORD {
                                consider(
                                    1.0 - alike + costs[(row + 1) * width + column + 1],
                                    Step::Same,
                                );
                            }
                            consider(
                                drop_cost(&said[row]) + costs[(row + 1) * width + column],
                                Step::Dropped,
                            );
                        }
                        let added = if wrote.norm.is_empty() { 0.0 } else { 1.0 };
                        consider(added + costs[row * width + column + 1], Step::Added);
                    }
                }
                costs[row * width + column] = best.0;
                steps[row * width + column] = best.1;
            }
        }
        Some(self.gaps(guard, &units, &steps, heard, cleaned, open))
    }

    /// Walks the line-up and cuts it at each kept word.
    fn gaps(
        &self,
        guard: &Guard,
        units: &[Unit],
        steps: &[Step],
        heard: &[Word<'_>],
        cleaned: &[Word<'_>],
        open: &[Mention],
    ) -> Vec<Gap> {
        let width = units.len() + 1;
        let unit_start = |column: usize| match units.get(column) {
            Some(Unit::Word(index)) => *index,
            Some(Unit::Term(mention)) => open[*mention].from,
            None => self.cleaned.end,
        };
        let fresh = |row: usize, column: usize| Gap {
            heard: self.heard.start + row..self.heard.start + row,
            cleaned: unit_start(column)..unit_start(column),
            terms: Vec::new(),
            dropped: Vec::new(),
            strays: 0,
        };
        let mut gaps = Vec::new();
        let (mut row, mut column) = (0, 0);
        let mut gap = fresh(0, 0);
        loop {
            let step = steps[row * width + column];
            match step {
                Step::End | Step::Same => {
                    gap.heard.end = self.heard.start + row;
                    gap.cleaned.end = unit_start(column);
                    if !gap.terms.is_empty() {
                        // Bare punctuation at either end (a list's dash)
                        // is cleanup's layout, not part of the gap.
                        while cleaned[gap.cleaned.start].norm.is_empty() {
                            gap.cleaned.start += 1;
                        }
                        while cleaned[gap.cleaned.end - 1].norm.is_empty() {
                            gap.cleaned.end -= 1;
                        }
                        gaps.push(gap);
                    }
                    if step == Step::End {
                        return gaps;
                    }
                    row += 1;
                    column += 1;
                    gap = fresh(row, column);
                }
                Step::Dropped => {
                    let index = self.heard.start + row;
                    if !heard[index].norm.is_empty() {
                        gap.dropped.push(index);
                    }
                    row += 1;
                }
                Step::Added => {
                    if let Some(Unit::Word(index)) = units.get(column)
                        && !cleaned[*index].norm.is_empty()
                    {
                        gap.strays += 1;
                    }
                    column += 1;
                }
                Step::Bare | Step::Run(..) | Step::Shorter(..) => {
                    let Some(Unit::Term(mention)) = units.get(column) else {
                        return gaps;
                    };
                    let (count, existing, shorter) = match step {
                        Step::Run(count, existing) => (count as usize, existing, None),
                        Step::Shorter(count, place) => {
                            let term = &guard.terms[open[*mention].term];
                            let (inner, offset) = term.inner[place as usize];
                            let from = open[*mention].from + offset;
                            let to = from + guard.terms[inner].words.len();
                            (count as usize, false, Some(from..to))
                        }
                        _ => (0, false, None),
                    };
                    let first = self.heard.start + row;
                    gap.terms.push(Placed {
                        mention: *mention,
                        run: (count > 0).then(|| first..first + count),
                        existing,
                        moved: false,
                        shorter,
                    });
                    row += count;
                    column += 1;
                }
            }
        }
    }
}

/// What dropping a heard word costs the line-up: nothing for bare
/// punctuation, little for a filler, and a whole step for a word that
/// means something.
fn drop_cost(word: &Word<'_>) -> f32 {
    if word.norm.is_empty() {
        0.0
    } else if plain_words::is_content(&word.norm) {
        1.0
    } else {
        0.5
    }
}

/// How alike a heard word and a written word sound.
fn same_word(said: &Word<'_>, said_key: &Key, wrote: &Word<'_>, scratch: &mut Scratch) -> f32 {
    if said.norm.is_empty() || wrote.norm.is_empty() {
        return 0.0;
    }
    if said.norm == wrote.norm {
        return 1.0;
    }
    let key = Key::of([wrote.core], true);
    if fit_bound(said_key, &key) < SAME_WORD {
        return 0.0;
    }
    fit(said_key, &key, &mut scratch.sounds).similarity
}
