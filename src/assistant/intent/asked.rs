//! A search of the recordings asked for outright, with its words, kinds,
//! and time already told apart, rather than read from a question: what a
//! model asks through Assistant's `recordings_search` tool. The words go
//! through the same reading a question's do (stop words out, a Russian
//! word cut to its stem, a Latin name's spoken forms), so the search
//! matches as it does for a typed question.

// Called by the agent's `recordings_search` tool, which comes next.
#![allow(dead_code)]

use super::{Intent, Kinds, latin_forms, terms, words};

/// The most "the last few" one search asks for.
const MOST_LATEST: usize = 20;

impl Intent {
    /// A search for `query`'s words in the recordings `kinds` names (none
    /// named: all of them), in `[since_ms, until_ms)`, the newest `latest`
    /// when given. It is surely about recordings: with no words to search
    /// for, the newest are read.
    pub fn asked(
        kinds: Kinds,
        query: &str,
        (since_ms, until_ms): (Option<u64>, Option<u64>),
        latest: Option<usize>,
    ) -> Self {
        let terms = terms(&words(query));
        Self {
            kinds: if kinds == Kinds::default() {
                Kinds::ALL
            } else {
                kinds
            },
            since_ms,
            until_ms,
            latest: latest.map(|latest| latest.clamp(1, MOST_LATEST)),
            spoken: latin_forms(query, &terms),
            terms,
            sure: true,
            written: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_asked_for_reads_its_words_as_a_question_does() {
        let asked = Intent::asked(
            Kinds {
                dictations: true,
                ..Kinds::default()
            },
            "VibeHex бюджету",
            (Some(10), Some(20)),
            Some(50),
        );
        assert!(asked.sure);
        assert!(asked.kinds.dictations && !asked.kinds.meetings && !asked.kinds.files);
        assert_eq!(asked.terms, ["vibehex", "бюджет"]);
        assert_eq!((asked.since_ms, asked.until_ms), (Some(10), Some(20)));
        assert_eq!(asked.latest, Some(20), "clamped");
        let (term, forms) = &asked.spoken[0];
        assert_eq!(term, "vibehex");
        assert!(forms.contains(&"вайб хекс".to_owned()), "{forms:?}");
    }

    #[test]
    fn no_kind_named_means_every_kind_and_no_words_the_newest() {
        let asked = Intent::asked(Kinds::default(), "", (None, None), None);
        assert_eq!(asked.kinds, Kinds::ALL);
        assert!(asked.terms.is_empty() && asked.sure && asked.latest.is_none());
    }
}
