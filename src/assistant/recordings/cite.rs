//! An answer's citations: `[2]` for an excerpt, `[2.3]` for its third
//! line, `[2.3-2.5]` for a run of lines, several in one pair of brackets
//! (`[2.3, 4.1]`) or side by side (`[2.3][4.1]`). The answer shows each as
//! the excerpt's `[n]`, once per excerpt where they stand together.

use std::ops::Range;

/// One citation: excerpt `n`, and its lines `first..=last` when it names
/// them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cite {
    pub n: u32,
    pub lines: Option<(u32, u32)>,
}

/// Citations standing together, `[2.3][4.1]`, and their bytes in the text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Run {
    pub at: Range<usize>,
    pub cites: Vec<Cite>,
}

impl Run {
    /// The excerpts it cites, each once, in order.
    pub fn numbers(&self) -> Vec<u32> {
        let mut numbers: Vec<u32> = Vec::new();
        for cite in &self.cites {
            if !numbers.contains(&cite.n) {
                numbers.push(cite.n);
            }
        }
        numbers
    }

    /// Whether it names lines, so the answer shows it otherwise than written.
    fn names_lines(&self) -> bool {
        self.cites.iter().any(|cite| cite.lines.is_some())
    }
}

/// Every run of citations in `text`, in order. Brackets holding anything
/// else (`[see below]`, a Markdown link's text) are left alone.
pub fn runs(text: &str) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find('[').map(|at| from + at) {
        let Some((cites, len)) = group(&text[open..]) else {
            from = open + 1;
            continue;
        };
        let end = open + len;
        // `[2] [3]` and `[2][3]` stand together.
        match runs.last_mut() {
            Some(run) if text[run.at.end..open].trim_matches(' ').is_empty() => {
                run.at.end = end;
                run.cites.extend(cites);
            }
            _ => runs.push(Run {
                at: open..end,
                cites,
            }),
        }
        from = end;
    }
    runs
}

/// `text` as the answer shows it: each run that names lines as its
/// excerpts' `[n]`, each once (`[2.3][2.5][4.1]` reads `[2][4]`).
pub fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for run in runs(text).iter().filter(|run| run.names_lines()) {
        out.push_str(&text[from..run.at.start]);
        for n in run.numbers() {
            out.push_str(&format!("[{n}]"));
        }
        from = run.at.end;
    }
    out.push_str(&text[from..]);
    out
}

/// How a citation still being written can end, so a streaming draft hides
/// it: `[2`, `[2.`, `[2.3-`, `[2.3, 4`.
pub fn unfinished(tail: &str) -> bool {
    tail.chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | ';' | ' ' | '-' | '–' | '—'))
}

/// `[…]` at the start of `text` when it holds only citations and isn't a
/// Markdown link's text: the citations and its length with the brackets.
fn group(text: &str) -> Option<(Vec<Cite>, usize)> {
    let after = text.strip_prefix('[')?;
    let inside = &after[..after.find(']')?];
    if after[inside.len() + 1..].starts_with('(') {
        return None;
    }
    let mut cites = Vec::new();
    for item in inside.split([',', ';']) {
        cites.extend(item_cites(item.trim())?);
    }
    (!cites.is_empty()).then_some((cites, inside.len() + 2))
}

/// `2`, `2.3`, `2.3-2.5`, or `2.3-5`. A run from one excerpt into another
/// cites both, without lines.
fn item_cites(item: &str) -> Option<Vec<Cite>> {
    let (from, to) = match item.split_once(['-', '–', '—']) {
        Some((from, to)) => (from.trim(), Some(to.trim())),
        None => (item, None),
    };
    let (n, first) = id(from)?;
    let Some(to) = to else {
        return Some(vec![Cite {
            n,
            lines: first.map(|first| (first, first)),
        }]);
    };
    let first = first?;
    let (other, last) = match id(to)? {
        (last, None) => (n, last),
        (other, Some(last)) => (other, last),
    };
    if other != n {
        return Some(vec![
            Cite { n, lines: None },
            Cite {
                n: other,
                lines: None,
            },
        ]);
    }
    let lines = (first.min(last), first.max(last));
    Some(vec![Cite {
        n,
        lines: Some(lines),
    }])
}

/// `2` or `2.3`.
fn id(text: &str) -> Option<(u32, Option<u32>)> {
    let number = |text: &str| {
        (!text.is_empty() && text.len() <= 4 && text.bytes().all(|b| b.is_ascii_digit()))
            .then(|| text.parse::<u32>().ok())
            .flatten()
    };
    match text.split_once('.') {
        Some((n, line)) => Some((number(n)?, Some(number(line)?))),
        None => Some((number(text)?, None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cite(n: u32, lines: Option<(u32, u32)>) -> Cite {
        Cite { n, lines }
    }

    #[test]
    fn lines_ranges_and_several_ids_are_read() {
        let found = runs("Limit [2.3], cut [2.3-2.5][4.1] and [1, 3.2-4] then [5.2–5.1].");
        let cites: Vec<Vec<Cite>> = found.iter().map(|run| run.cites.clone()).collect();
        assert_eq!(
            cites,
            [
                vec![cite(2, Some((3, 3)))],
                vec![cite(2, Some((3, 5))), cite(4, Some((1, 1)))],
                vec![cite(1, None), cite(3, Some((2, 4)))],
                vec![cite(5, Some((1, 2)))],
            ]
        );
        assert_eq!(found[1].numbers(), [2, 4]);
    }

    #[test]
    fn junk_is_left_as_written() {
        for text in [
            "[see below]",
            "[x]",
            "[]",
            "[12",
            "[2.]",
            "[.3]",
            "[2.3.4]",
            "[2.3-]",
            "[link](https://x)",
            "[2.3](x)",
            "[12345]",
        ] {
            assert!(runs(text).is_empty(), "{text}");
            assert_eq!(plain(text), text);
        }
        // From one excerpt into another: both, without lines.
        assert_eq!(runs("[2.3-3.1]")[0].cites, [cite(2, None), cite(3, None)]);
    }

    #[test]
    fn the_answer_shows_each_excerpt_once_where_they_stand_together() {
        assert_eq!(
            plain("Cut [2.3][2.5] [4.1-4.2]. Kept [1] and [1, 3]."),
            "Cut [2][4]. Kept [1] and [1, 3]."
        );
        assert_eq!(plain("Да [2.3, 2.4]."), "Да [2].");
    }

    #[test]
    fn a_citation_being_written_is_unfinished() {
        for tail in ["2", "2.", "2.3-", "2.3-2.", "2.3, 4", "2.3–"] {
            assert!(unfinished(tail), "{tail}");
        }
        assert!(!unfinished("link"));
    }
}
