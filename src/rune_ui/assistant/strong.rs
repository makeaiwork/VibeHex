//! Bold words in an answer's Markdown: where they are, so they can stand
//! out from the soft reading colour, and the ones that name a place in the
//! window turned into links that open it.
//!
//! The scan is a small reading of CommonMark, enough for what models write:
//! `**words**` (or `__words__`) on one line, outside fenced code, inline
//! code, autolinks, and links. A run still arriving (`**Gene`) is no bold
//! yet, so a stream never rewrites half a name.

use std::ops::Range;

/// What a place link's destination starts with: `hex-place:2` is the
/// answer's third place name.
pub(in super::super) const PLACE: &str = "hex-place:";

/// One bold run: the whole `**…**` and the words inside its markers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct Bold {
    pub(in super::super) outer: Range<usize>,
    pub(in super::super) inner: Range<usize>,
    /// The words hold no code, link, or other markup: a name as written.
    plain: bool,
}

/// What a line holds that bold never reaches into, and its bold runs.
#[derive(Default)]
struct Scan {
    bolds: Vec<Bold>,
    /// Every link (`[words](url)`), whole.
    links: Vec<Range<usize>>,
}

/// The bold runs of `source`, in order.
pub(in super::super) fn bolds(source: &str) -> Vec<Bold> {
    scan(source).bolds
}

/// `source` with every plain bold run that `is_place` names written as a
/// link to it, `[**words**](hex-place:n)`, and the names linked, the n-th
/// for `hex-place:n`. The rest is left as written.
pub(in super::super) fn linked(
    source: &str,
    is_place: impl Fn(&str) -> bool,
) -> (String, Vec<String>) {
    let mut out = String::with_capacity(source.len());
    let mut names: Vec<String> = Vec::new();
    let mut from = 0;
    for bold in bolds(source) {
        let words = &source[bold.inner.clone()];
        if !bold.plain || !is_place(words) {
            continue;
        }
        let n = match names.iter().position(|name| name == words) {
            Some(n) => n,
            None => {
                names.push(words.to_owned());
                names.len() - 1
            }
        };
        out.push_str(&source[from..bold.outer.start]);
        out.push('[');
        out.push_str(&source[bold.outer.clone()]);
        out.push_str("](");
        out.push_str(PLACE);
        out.push_str(&n.to_string());
        out.push(')');
        from = bold.outer.end;
    }
    out.push_str(&source[from..]);
    (out, names)
}

/// The bytes of `source` that are bold words, outside any link inside the
/// bold (a citation's mark keeps its own colour): what draws at full
/// brightness.
pub(in super::super) fn bright(source: &str) -> Vec<Range<usize>> {
    let Scan { bolds, links } = scan(source);
    let mut out = Vec::new();
    for bold in bolds {
        let mut at = bold.inner.start;
        for link in links
            .iter()
            .filter(|link| link.start >= bold.inner.start && link.end <= bold.inner.end)
        {
            if link.start > at {
                out.push(at..link.start);
            }
            at = link.end;
        }
        if bold.inner.end > at {
            out.push(at..bold.inner.end);
        }
    }
    out
}

fn scan(source: &str) -> Scan {
    let mut found = Scan::default();
    let mut fence: Option<(u8, usize)> = None;
    let mut start = 0;
    for line in source.split_inclusive('\n') {
        let at = start;
        start += line.len();
        let body = line.trim_end_matches(['\n', '\r']);
        let opened = body.trim_start_matches([' ', '\t', '>']);
        let opened = opened
            .strip_prefix(['-', '*', '+'])
            .filter(|rest| rest.starts_with(' '))
            .unwrap_or(opened)
            .trim_start();
        if let Some(run) = fence_run(opened) {
            match fence {
                None => fence = Some(run),
                Some((mark, len)) if run.0 == mark && run.1 >= len => {
                    if opened[run.1..].trim().is_empty() {
                        fence = None;
                    }
                }
                Some(_) => {}
            }
            continue;
        }
        if fence.is_none() {
            inline(body, at, &mut found);
        }
    }
    found
}

/// A fence's mark and length, when `line` opens or closes one.
fn fence_run(line: &str) -> Option<(u8, usize)> {
    let mark = *line.as_bytes().first()?;
    if mark != b'`' && mark != b'~' {
        return None;
    }
    let len = line.bytes().take_while(|byte| *byte == mark).count();
    (len >= 3).then_some((mark, len))
}

/// The bold runs and links of one line, `at` bytes into the source.
fn inline(line: &str, at: usize, found: &mut Scan) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'`' => i = past_code(bytes, i),
            b'<' => i = past_autolink(bytes, i).unwrap_or(i + 1),
            b'[' => match link_end(bytes, i) {
                Some(end) => {
                    found.links.push(at + i..at + end);
                    i = end;
                }
                None => i += 1,
            },
            mark @ (b'*' | b'_') => {
                let run = run_len(bytes, i, mark);
                if run == 2
                    && opens(bytes, i, mark)
                    && let Some((close, plain)) = closer(bytes, i + 2, mark, found, at)
                {
                    found.bolds.push(Bold {
                        outer: at + i..at + close + 2,
                        inner: at + i + 2..at + close,
                        plain,
                    });
                    i = close + 2;
                } else {
                    i += run;
                }
            }
            _ => i += 1,
        }
    }
}

fn run_len(bytes: &[u8], i: usize, mark: u8) -> usize {
    bytes[i..].iter().take_while(|byte| **byte == mark).count()
}

/// A `**` at `i` can open bold: words follow it, and `__` starts a word.
fn opens(bytes: &[u8], i: usize, mark: u8) -> bool {
    let next = bytes.get(i + 2);
    let before = i.checked_sub(1).map(|at| bytes[at]);
    next.is_some_and(|byte| !byte.is_ascii_whitespace())
        && (mark == b'*' || before.is_none_or(|byte| !byte.is_ascii_alphanumeric()))
}

/// Where the bold opened before `from` closes on this line, and whether
/// the words between hold no markup. Links inside it are noted.
fn closer(
    bytes: &[u8],
    from: usize,
    mark: u8,
    found: &mut Scan,
    at: usize,
) -> Option<(usize, bool)> {
    let mut plain = true;
    let mut links = Vec::new();
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                plain = false;
                i += 2;
            }
            b'`' => {
                plain = false;
                i = past_code(bytes, i);
            }
            b'[' => {
                plain = false;
                match link_end(bytes, i) {
                    Some(end) => {
                        links.push(at + i..at + end);
                        i = end;
                    }
                    None => i += 1,
                }
            }
            byte if byte == mark => {
                let run = run_len(bytes, i, mark);
                let after = bytes.get(i + run);
                if run == 2
                    && !bytes[i - 1].is_ascii_whitespace()
                    && (mark == b'*' || after.is_none_or(|byte| !byte.is_ascii_alphanumeric()))
                {
                    found.links.extend(links);
                    return Some((i, plain));
                }
                plain = false;
                i += run;
            }
            b'*' | b'_' | b'<' => {
                plain = false;
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

/// Past the code span opening at `i`, or past its backticks when it
/// never closes on this line.
fn past_code(bytes: &[u8], i: usize) -> usize {
    let run = run_len(bytes, i, b'`');
    let mut j = i + run;
    while j < bytes.len() {
        if bytes[j] == b'`' {
            let close = run_len(bytes, j, b'`');
            if close == run {
                return j + close;
            }
            j += close;
        } else {
            j += 1;
        }
    }
    i + run
}

/// Past `<https://…>` or `<name@host>` opening at `i`.
fn past_autolink(bytes: &[u8], i: usize) -> Option<usize> {
    let close = bytes[i + 1..].iter().position(|byte| *byte == b'>')? + i + 1;
    let inside = &bytes[i + 1..close];
    let link = !inside.is_empty()
        && !inside.iter().any(u8::is_ascii_whitespace)
        && (inside.contains(&b':') || inside.contains(&b'@'));
    link.then_some(close + 1)
}

/// The end of the link `[words](url)` (or `[words][ref]`) opening at `i`.
fn link_end(bytes: &[u8], i: usize) -> Option<usize> {
    let mut depth = 0;
    let mut j = i;
    let close = loop {
        match bytes.get(j)? {
            b'\\' => j += 1,
            b'`' => {
                j = past_code(bytes, j);
                continue;
            }
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break j;
                }
            }
            _ => {}
        }
        j += 1;
    };
    let (open, shut) = match bytes.get(close + 1)? {
        b'(' => (b'(', b')'),
        b'[' => (b'[', b']'),
        _ => return None,
    };
    let mut depth = 0;
    for (k, byte) in bytes.iter().enumerate().skip(close + 1) {
        if *byte == open {
            depth += 1;
        } else if *byte == shut {
            depth -= 1;
            if depth == 0 {
                return Some(k + 1);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(source: &str) -> Vec<&str> {
        bolds(source)
            .into_iter()
            .map(|bold| &source[bold.inner])
            .collect()
    }

    fn place(words: &str) -> bool {
        ["General › Vocabulary", "Output", "Labs › Smart spacing"].contains(&words)
    }

    #[test]
    fn finds_bold_runs_and_leaves_lone_stars() {
        assert_eq!(
            words("Open **General › Vocabulary**, then **Add**. 2 * 3 ** 4 and __snake__case"),
            ["General › Vocabulary", "Add"]
        );
        assert_eq!(words("a **b** c __d__ e"), ["b", "d"]);
        assert!(words("** spaced ** and ***both***").is_empty());
    }

    #[test]
    fn a_place_becomes_a_link_and_other_bold_stays() {
        let (out, names) = linked(
            "Go to **General › Vocabulary**, press **Add**, then **Output**.",
            place,
        );
        assert_eq!(
            out,
            "Go to [**General › Vocabulary**](hex-place:0), press **Add**, then [**Output**](hex-place:1)."
        );
        assert_eq!(names, ["General › Vocabulary", "Output"]);
    }

    #[test]
    fn the_same_place_twice_is_one_name() {
        let (out, names) = linked("**Output** and **Output**", place);
        assert_eq!(
            out,
            "[**Output**](hex-place:0) and [**Output**](hex-place:0)"
        );
        assert_eq!(names, ["Output"]);
    }

    #[test]
    fn places_in_lists_quotes_and_tables_are_linked() {
        let source = "1. Turn on **Labs › Smart spacing**.\n- **Output**\n> **Output**\n\n| Where | |\n|---|---|\n| **General › Vocabulary** | **Add** |\n";
        let (out, names) = linked(source, place);
        assert!(out.contains("1. Turn on [**Labs › Smart spacing**](hex-place:0)."));
        assert!(out.contains("- [**Output**](hex-place:1)\n"));
        assert!(out.contains("> [**Output**](hex-place:1)\n"));
        assert!(out.contains("| [**General › Vocabulary**](hex-place:2) | **Add** |"));
        assert_eq!(names.len(), 3);
    }

    #[test]
    fn code_links_and_escapes_are_left_alone() {
        let source = concat!(
            "`**Output**` and ``a **Output** b``\n",
            "```\n**Output**\n```\n",
            "  ~~~md\n  **Output**\n  ~~~\n",
            "[**Output**](https://x) and [see **Output**][1] and \\**Output**\n",
            "<https://x/**Output**>\n",
        );
        let (out, names) = linked(source, place);
        assert_eq!(out, source);
        assert!(names.is_empty());
        assert!(bright(source).is_empty());
    }

    #[test]
    fn bold_with_markup_inside_is_no_name() {
        let source = "**Output `x`** and **see [1](hex-ref:1.0)** and **Out*put***";
        let (out, _) = linked(source, |_| true);
        assert_eq!(out, source);
    }

    #[test]
    fn a_bold_still_arriving_is_left_as_written() {
        for partial in [
            "Open **General › Voc",
            "Open **General › Vocabulary*",
            "Open **General › Vocabulary",
            "Open **",
        ] {
            let (out, names) = linked(partial, place);
            assert_eq!(out, partial);
            assert!(names.is_empty());
            assert!(bright(partial).is_empty());
        }
    }

    #[test]
    fn bright_is_the_words_without_links_inside() {
        let source = "**by Friday** and **see [2](hex-ref:2.0) here**";
        let lit: Vec<&str> = bright(source).into_iter().map(|at| &source[at]).collect();
        assert_eq!(lit, ["by Friday", "see ", " here"]);
    }

    #[test]
    fn a_linked_place_is_not_bright() {
        let (out, _) = linked("**Output** and **Add**", place);
        let lit: Vec<&str> = bright(&out).into_iter().map(|at| &out[at]).collect();
        assert_eq!(lit, ["Add"]);
    }

    #[test]
    fn non_ascii_words_keep_their_bytes() {
        let source = "Нажмите **Добавить** — **General › Vocabulary**";
        assert_eq!(words(source), ["Добавить", "General › Vocabulary"]);
        let (out, _) = linked(source, place);
        assert_eq!(
            out,
            "Нажмите **Добавить** — [**General › Vocabulary**](hex-place:0)"
        );
    }
}
