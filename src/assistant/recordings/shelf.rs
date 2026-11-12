//! One turn's excerpts, numbered once. An answer that reads the
//! recordings several times (a search, then a meeting's notes, then more
//! of its transcript) cites all of it by `[n]` and `[n.m]`, so every
//! excerpt keeps one number for the whole turn: each read's excerpts are
//! numbered on from the last, and one read again (the same recording, the
//! same place, the same text) keeps the number it had. What the turn keeps
//! is the one [`Found`] the answer's citations are read against.

// Read through by the agent's tools, which come next.
#![allow(dead_code)]

use super::{Excerpt, Found, Step};

/// The excerpts a turn has read so far, in the order read.
#[derive(Debug, Default)]
pub struct Shelf {
    found: Found,
}

/// Where a read's excerpt stands on the shelf.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Placed {
    pub n: u32,
    /// It wasn't there before this read.
    pub new: bool,
}

impl Shelf {
    /// Puts one read's `excerpts` on the shelf, in order: a new one gets
    /// the next number, one already there keeps its own.
    pub fn put(&mut self, excerpts: Vec<Excerpt>) -> Vec<Placed> {
        excerpts
            .into_iter()
            .map(|mut excerpt| {
                if let Some(known) = self
                    .found
                    .excerpts
                    .iter()
                    .find(|known| same(known, &excerpt))
                {
                    return Placed {
                        n: known.n,
                        new: false,
                    };
                }
                excerpt.n = self.found.excerpts.len() as u32 + 1;
                let n = excerpt.n;
                self.found.excerpts.push(excerpt);
                Placed { n, new: true }
            })
            .collect()
    }

    /// Keeps a search's steps with the turn.
    pub fn stepped(&mut self, steps: impl IntoIterator<Item = Step>) {
        self.found.steps.extend(steps);
    }

    /// Everything read so far, as the answer's citations resolve it.
    pub fn found(&self) -> &Found {
        &self.found
    }

    /// The excerpt numbered `n`.
    pub fn excerpt(&self, n: u32) -> Option<&Excerpt> {
        self.found.excerpts.iter().find(|excerpt| excerpt.n == n)
    }

    pub fn is_empty(&self) -> bool {
        self.found.excerpts.is_empty()
    }

    pub fn into_found(self) -> Found {
        self.found
    }
}

/// The same recording at the same place with the same text, whatever
/// number either has.
fn same(one: &Excerpt, other: &Excerpt) -> bool {
    one.kind == other.kind
        && one.id == other.id
        && one.offset_ms == other.offset_ms
        && one.label == other.label
        && one.text == other.text
        && one.lines.len() == other.lines.len()
}

#[cfg(test)]
mod tests {
    use super::super::Kind;
    use super::*;

    fn excerpt(id: &str, offset: Option<u64>, text: &str) -> Excerpt {
        Excerpt {
            n: 0,
            kind: Kind::Meeting,
            id: id.into(),
            title: "Budget".into(),
            at_ms: 0,
            offset_ms: offset,
            label: String::new(),
            text: text.into(),
            lines: Vec::new(),
        }
    }

    /// Numbers run on from one read to the next, and a repeat keeps its
    /// number.
    #[test]
    fn numbers_continue_across_reads_and_a_repeat_keeps_its_own() {
        let mut shelf = Shelf::default();
        let first = shelf.put(vec![
            excerpt("a", None, "notes"),
            excerpt("a", Some(5), "one"),
        ]);
        assert_eq!(
            first.iter().map(|placed| placed.n).collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(first.iter().all(|placed| placed.new));
        let second = shelf.put(vec![
            excerpt("a", Some(5), "one"),
            excerpt("b", Some(5), "one"),
            excerpt("a", Some(5), "one, and more"),
        ]);
        assert_eq!(
            second,
            [
                Placed { n: 2, new: false },
                Placed { n: 3, new: true },
                Placed { n: 4, new: true },
            ]
        );
        let numbers: Vec<u32> = shelf
            .found()
            .excerpts
            .iter()
            .map(|excerpt| excerpt.n)
            .collect();
        assert_eq!(numbers, [1, 2, 3, 4]);
        assert_eq!(
            shelf.excerpt(3).map(|excerpt| excerpt.id.as_str()),
            Some("b")
        );
    }
}
