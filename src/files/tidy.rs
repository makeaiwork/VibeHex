//! Cleans a meeting's transcript turn by turn. Each turn goes to the model
//! as one numbered line and comes back as one, so no turn runs into the
//! next and every turn keeps who said it and when; a turn the model leaves
//! out keeps its words. Turns go a few at a time, as cleanup's pieces do.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use color_eyre::eyre::{Result, eyre};

use super::rewrite::{DraftKind, Drafts, Engine};
use crate::dictation::cleanup_pieces::{PIECE_WORDS, Streamed};
use crate::dictation::cleanup_server::engine;
use crate::dictation::vocabulary::VocabularyEntry;

/// One turn: who spoke, when (as the transcript shows it), and the words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub speaker: String,
    pub clock: String,
    pub text: String,
}

/// What a tidy made: the turns it reached, cleaned or as they were.
#[derive(Debug)]
pub struct Tidied {
    pub lines: Vec<Line>,
    /// Stopped before the last turn: `lines` ends where it stopped.
    pub stopped: bool,
    /// Why some turns kept their words.
    pub failed: Option<String>,
}

/// The model's instructions: the cleanup prompt and vocabulary for the
/// words, and the numbered lines' form.
pub fn instructions(prompt: &str, vocabulary: &[VocabularyEntry]) -> String {
    format!(
        "You clean up a meeting's transcript. Each line is one turn, starting with its number \
         in brackets, like [3]. Clean the words of each turn following these instructions:\n{}\n\n\
         {}Clean each turn on its own, in its language. Return every line in the same order, \
         each on one line starting with its number as given. Never merge, split, drop, or add \
         lines. Return only the lines, without an explanation, a label, or a Markdown fence.",
        prompt.trim(),
        crate::dictation::processor::vocabulary_lines(vocabulary)
    )
}

/// The turns as Markdown: who and when in bold over each turn's words.
pub fn markdown(lines: &[Line]) -> String {
    lines
        .iter()
        .map(|line| {
            format!(
                "**{}** `{}`  \n{}",
                line.speaker,
                line.clock,
                line.text.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Turn indexes in pieces of about `words` words, each at least one turn.
fn pieces(lines: &[Line], words: usize) -> Vec<std::ops::Range<usize>> {
    let mut pieces = Vec::new();
    let (mut start, mut count) = (0, 0);
    for (index, line) in lines.iter().enumerate() {
        let size = line.text.split_whitespace().count();
        if index > start && count + size > words {
            pieces.push(start..index);
            (start, count) = (index, 0);
        }
        count += size;
    }
    if start < lines.len() {
        pieces.push(start..lines.len());
    }
    pieces
}

/// The piece's numbered lines; a turn's own line breaks become spaces.
fn numbered(lines: &[Line], piece: std::ops::Range<usize>) -> String {
    piece
        .map(|index| {
            let words: Vec<&str> = lines[index].text.split_whitespace().collect();
            format!("[{}] {}", index + 1, words.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The model's lines by turn index, for turns in `piece` only; a line
/// without a number continues the one before it.
fn parse(reply: &str, piece: &std::ops::Range<usize>) -> HashMap<usize, String> {
    let mut found: HashMap<usize, String> = HashMap::new();
    let mut current = None;
    for raw in reply.lines() {
        let line = raw.trim().trim_start_matches(['-', '*', ' ']);
        let numbered = line.strip_prefix('[').and_then(|rest| {
            let (number, words) = rest.split_once(']')?;
            let index = number.trim().parse::<usize>().ok()?.checked_sub(1)?;
            Some((index, words.trim()))
        });
        match numbered {
            Some((index, words)) if piece.contains(&index) => {
                found.insert(index, words.to_owned());
                current = Some(index);
            }
            Some(_) => current = None,
            None if !line.is_empty() => {
                if let Some(text) = current.and_then(|index| found.get_mut(&index)) {
                    text.push(' ');
                    text.push_str(line);
                }
            }
            None => {}
        }
    }
    found.retain(|_, text| !text.trim().is_empty());
    found
}

/// `lines` with the piece's turns replaced by the model's, where it gave them.
fn apply(lines: &mut [Line], found: HashMap<usize, String>) {
    for (index, text) in found {
        if let Some(line) = lines.get_mut(index) {
            line.text = text;
        }
    }
}

/// Cleans `lines` a piece at a time; `progress` hears the fraction done and
/// `drafts` the transcript so far as Markdown, the turns arriving as they
/// come. A piece that fails keeps its words; a stop keeps the turns done.
pub fn tidy(
    engine: &Engine,
    instructions: &str,
    lines: &[Line],
    cancelled: &AtomicBool,
    progress: &mut dyn FnMut(f32),
    drafts: &Drafts,
) -> Result<Tidied> {
    let pieces = pieces(lines, PIECE_WORDS);
    let mut made = lines.to_vec();
    let mut failures = Vec::new();
    for (number, piece) in pieces.iter().enumerate() {
        let input = format!(
            "Transcript:\n<transcript>\n{}\n</transcript>",
            numbered(lines, piece.clone())
        );
        let streamed = {
            let drafts = drafts.clone();
            let before = made[..piece.start].to_vec();
            let original = lines.to_vec();
            let piece = piece.clone();
            let streamed: Streamed = Arc::new(move |_: usize, text: &str| {
                let mut shown = before.clone();
                let mut arriving = original[piece.clone()].to_vec();
                let found = parse(text, &piece);
                let reached = found.keys().max().map_or(piece.start, |last| last + 1);
                for (index, words) in found {
                    arriving[index - piece.start].text = words;
                }
                shown.extend(arriving.into_iter().take(reached - piece.start));
                drafts(DraftKind::Result, &markdown(&shown));
            });
            streamed
        };
        let reply = engine.generate(
            engine::CONTEXT_TOKENS,
            instructions,
            &input,
            cancelled,
            Some(streamed),
        );
        if cancelled.load(Ordering::Acquire) {
            if piece.start == 0 {
                return Err(eyre!("stopped"));
            }
            made.truncate(piece.start);
            return Ok(Tidied {
                lines: made,
                stopped: true,
                failed: None,
            });
        }
        match reply {
            Ok(reply) => {
                let found = parse(&reply, piece);
                if found.is_empty() {
                    failures.push("the model returned no turns".to_string());
                }
                apply(&mut made, found);
            }
            Err(error) => failures.push(error.to_string()),
        }
        progress((number + 1) as f32 / pieces.len().max(1) as f32);
        drafts(DraftKind::Result, &markdown(&made[..piece.end]));
    }
    if !pieces.is_empty() && failures.len() == pieces.len() {
        return Err(eyre!("{}", failures[0]));
    }
    Ok(Tidied {
        lines: made,
        stopped: false,
        failed: (!failures.is_empty()).then(|| {
            format!(
                "{} of {} parts kept as transcribed: {}",
                failures.len(),
                pieces.len(),
                failures[0]
            )
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(speaker: &str, clock: &str, text: &str) -> Line {
        Line {
            speaker: speaker.into(),
            clock: clock.into(),
            text: text.into(),
        }
    }

    #[test]
    fn each_turn_comes_back_to_its_own_place_and_keeps_its_time() {
        let mut lines = vec![
            line("You", "00:01", "so um we we start on\nmonday"),
            line("Them", "00:07", "yeah okay"),
            line("You", "00:12", "and the login uh first"),
        ];
        assert_eq!(
            numbered(&lines, 0..3),
            "[1] so um we we start on monday\n[2] yeah okay\n[3] and the login uh first"
        );
        // The model bullets one line, wraps another, and drops the second.
        let reply = "- [1] We start on Monday.\n[3] And the login\nfirst.";
        let found = parse(reply, &(0..3));
        assert_eq!(found.len(), 2);
        apply(&mut lines, found);
        assert_eq!(lines[0].text, "We start on Monday.");
        assert_eq!(lines[1].text, "yeah okay");
        assert_eq!(lines[2].text, "And the login first.");
        assert_eq!(lines[2].clock, "00:12");
        assert_eq!(
            markdown(&lines[..2]),
            "**You** `00:01`  \nWe start on Monday.\n\n**Them** `00:07`  \nyeah okay"
        );
    }

    #[test]
    fn a_number_outside_the_piece_is_ignored() {
        let found = parse("[1] Earlier.\n[4] Mine.\n[9] Later.", &(3..5));
        assert_eq!(found.len(), 1);
        assert_eq!(found[&3], "Mine.");
    }

    #[test]
    fn turns_go_in_pieces_and_a_long_turn_stands_alone() {
        let words = |n: usize| vec!["w"; n].join(" ");
        let lines = vec![
            line("You", "00:01", &words(40)),
            line("Them", "00:02", &words(40)),
            line("You", "00:03", &words(300)),
            line("Them", "00:04", &words(5)),
        ];
        assert_eq!(pieces(&lines, 100), vec![0..2, 2..3, 3..4]);
        assert!(pieces(&[], 100).is_empty());
    }
}
