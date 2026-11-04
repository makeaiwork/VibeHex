//! Long dictations' AI cleanup: the transcript is cleaned in pieces of
//! about `PIECE_WORDS` words, each cut after a sentence, so a small model
//! keeps every word to the end and each piece answers within the mode's
//! deadline however long the dictation. A piece that fails keeps its
//! transcribed text; the others are still cleaned.
//!
//! While Labs › Cleanup progress is on, a cleanup that runs past
//! `SHOW_AFTER` reports how far it is to its watchers (`watch`, the plate
//! beside the HUD), and Paste now (`paste_now`) ends it early: the pieces
//! already cleaned, then the rest as transcribed.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use color_eyre::Result;

/// About three minutes of speech: short enough that a small local model
/// copies it whole and answers well within a deadline, long enough that a
/// short dictation stays one piece.
pub(crate) const PIECE_WORDS: usize = 400;
/// How long a cleanup runs before its progress shows, so a quick one
/// never flashes a plate.
const SHOW_AFTER: Duration = Duration::from_millis(1500);
const TICK: Duration = Duration::from_millis(200);
/// The most of a piece the streamed answer fills before the piece ends,
/// since an answer may run a little longer than its transcript.
const STREAMED_CAP: f32 = 0.95;

/// Called with how many characters of a piece's answer have arrived.
pub(crate) type Streamed = Arc<dyn Fn(usize) + Send + Sync>;

/// How far a cleanup is: `done` of `total` pieces, and the whole as a
/// fraction from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Progress {
    pub done: usize,
    pub total: usize,
    pub fraction: f32,
}

/// What cleaning a transcript gave.
#[derive(Debug, PartialEq)]
pub(crate) struct Cleaned {
    pub text: String,
    /// Why one or more pieces kept their transcribed text.
    pub failed: Option<String>,
    /// Paste now ended it before every piece was cleaned.
    pub early: bool,
}

/// Cleans `text` piece by piece with `generate`, which gets the piece, a
/// flag that ends its request, and (while Cleanup progress is on) where to
/// report its streamed answer. `cancelled` ends the work like Paste now.
pub(crate) fn clean(
    text: &str,
    cancelled: &AtomicBool,
    mut generate: impl FnMut(&str, &AtomicBool, Option<Streamed>) -> Result<String>,
) -> Cleaned {
    let pieces = split(text, PIECE_WORDS);
    let stop = Arc::new(AtomicBool::new(false));
    let job = crate::labs::cleanup_progress().then(|| Job::begin(pieces.len(), stop.clone()));
    let finished = AtomicBool::new(false);
    std::thread::scope(|scope| {
        // The caller's cancellation reaches each piece's request through
        // `stop`, which Paste now also sets.
        scope.spawn(|| {
            while !finished.load(Ordering::Acquire) {
                if cancelled.load(Ordering::Acquire) {
                    stop.store(true, Ordering::Release);
                    return;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        let cleaned = clean_pieces(&pieces, &stop, job.as_ref(), &mut generate);
        finished.store(true, Ordering::Release);
        cleaned
    })
}

fn clean_pieces(
    pieces: &[&str],
    stop: &AtomicBool,
    job: Option<&Job>,
    generate: &mut impl FnMut(&str, &AtomicBool, Option<Streamed>) -> Result<String>,
) -> Cleaned {
    let mut text = String::new();
    let mut reasons: Vec<String> = Vec::new();
    let mut early = false;
    for piece in pieces {
        let body = piece.trim_end();
        let tail = &piece[body.len()..];
        if stop.load(Ordering::Acquire) {
            early = true;
            text.push_str(piece);
            continue;
        }
        let streamed = job.map(|job| job.streamed(body.chars().count()));
        match generate(body, stop, streamed) {
            Ok(cleaned) if !cleaned.trim().is_empty() => {
                text.push_str(cleaned.trim());
                text.push_str(tail);
            }
            // Stopped mid-piece: what it had is dropped, the piece kept.
            _ if stop.load(Ordering::Acquire) => {
                early = true;
                text.push_str(piece);
            }
            Ok(_) => {
                reasons.push("processor returned empty text".into());
                text.push_str(piece);
            }
            Err(error) => {
                reasons.push(error.to_string());
                text.push_str(piece);
            }
        }
        if let Some(job) = job {
            job.piece_done();
        }
    }
    let failed = match reasons.len() {
        0 => None,
        n if n == pieces.len() => reasons.into_iter().next(),
        n => Some(format!(
            "{n} of {} pieces kept as transcribed: {}",
            pieces.len(),
            reasons[0]
        )),
    };
    Cleaned {
        text,
        failed,
        early,
    }
}

/// `text` cut into pieces of at most about `max_words` words that join
/// back into it exactly: after a sentence's end (`.`, `!`, `?`, `…`, or a
/// line break) where one fits, else between words.
pub(crate) fn split(text: &str, max_words: usize) -> Vec<&str> {
    let max_words = max_words.max(1);
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut words = 0;
    let mut sentence_start = 0;
    for end in sentence_ends(text).into_iter().chain([text.len()]) {
        if end <= sentence_start {
            continue;
        }
        let count = text[sentence_start..end].split_whitespace().count();
        if words > 0 && words + count > max_words {
            pieces.push(&text[start..sentence_start]);
            start = sentence_start;
            words = 0;
        }
        words += count;
        // One sentence longer than a piece: cut it between words.
        while words > max_words {
            let cut = start + word_cut(&text[start..end], max_words);
            pieces.push(&text[start..cut]);
            start = cut;
            words = text[start..end].split_whitespace().count();
        }
        sentence_start = end;
    }
    if start < text.len() {
        pieces.push(&text[start..]);
    }
    pieces
}

/// Where each sentence ends: just past its closing marks and the space
/// after them, so the next sentence starts there.
fn sentence_ends(text: &str) -> Vec<usize> {
    let mut ends = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        let closes = matches!(c, '.' | '!' | '?' | '…' | '\n');
        if !closes {
            continue;
        }
        // Closing quotes and brackets stay with their sentence.
        while let Some((_, next)) = chars.peek() {
            if matches!(next, '.' | '!' | '?' | '…' | '"' | '»' | '”' | ')' | '\'') {
                chars.next();
            } else {
                break;
            }
        }
        let mut spaced = false;
        while let Some((_, next)) = chars.peek() {
            if next.is_whitespace() {
                spaced = true;
                chars.next();
            } else {
                break;
            }
        }
        if spaced || c == '\n' {
            ends.push(chars.peek().map_or(text.len(), |(index, _)| *index));
        }
    }
    ends
}

/// The byte offset in `text` where its word after the first `words` starts.
fn word_cut(text: &str, words: usize) -> usize {
    let mut seen = 0;
    let mut in_word = false;
    for (index, c) in text.char_indices() {
        if c.is_whitespace() {
            in_word = false;
        } else if !in_word {
            in_word = true;
            if seen == words {
                return index;
            }
            seen += 1;
        }
    }
    text.len()
}

struct State {
    id: u64,
    done: usize,
    total: usize,
    /// How much of the current piece has arrived, 0 to `STREAMED_CAP`.
    partial: f32,
    started: Instant,
    stop: Arc<AtomicBool>,
}

impl State {
    fn progress(&self) -> Progress {
        let total = self.total.max(1);
        Progress {
            done: self.done,
            total: self.total,
            fraction: ((self.done as f32 + self.partial) / total as f32).clamp(0.0, 1.0),
        }
    }
}

type Watcher = Arc<dyn Fn(Option<Progress>) + Send + Sync>;

static STATE: Mutex<Option<State>> = Mutex::new(None);
static WATCHERS: Mutex<Vec<Watcher>> = Mutex::new(Vec::new());
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|error| error.into_inner())
}

/// Calls `watcher` with a long cleanup's progress, about five times a
/// second once it has run `SHOW_AFTER`, and with `None` when it ends.
pub(crate) fn watch(watcher: impl Fn(Option<Progress>) + Send + Sync + 'static) {
    WATCHERS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .push(Arc::new(watcher));
}

fn notify(progress: Option<Progress>) {
    let watchers = WATCHERS
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    for watcher in watchers {
        watcher(progress);
    }
}

/// Paste now: ends the cleanup in progress, which then pastes the pieces
/// already cleaned and the rest as transcribed. Nothing while none runs.
pub(crate) fn paste_now() {
    if let Some(state) = state().as_ref() {
        tracing::info!(
            done = state.done,
            total = state.total,
            "Paste now: pasting the cleanup so far"
        );
        state.stop.store(true, Ordering::Release);
    }
}

/// One cleanup's progress while it runs; dropping it ends it.
struct Job {
    id: u64,
}

impl Job {
    fn begin(total: usize, stop: Arc<AtomicBool>) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        *state() = Some(State {
            id,
            done: 0,
            total,
            partial: 0.0,
            started: Instant::now(),
            stop,
        });
        let ticker = std::thread::Builder::new()
            .name("hex-cleanup-progress".into())
            .spawn(move || tick(id));
        if let Err(error) = ticker {
            tracing::warn!(%error, "couldn't start the cleanup progress thread");
        }
        Self { id }
    }

    fn streamed(&self, piece_chars: usize) -> Streamed {
        let id = self.id;
        let piece_chars = piece_chars.max(1) as f32;
        Arc::new(move |chars| {
            if let Some(state) = state().as_mut().filter(|state| state.id == id) {
                state.partial = (chars as f32 / piece_chars).min(STREAMED_CAP);
            }
        })
    }

    fn piece_done(&self) {
        if let Some(state) = state().as_mut().filter(|state| state.id == self.id) {
            state.done += 1;
            state.partial = 0.0;
        }
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        let mut state = state();
        if state.as_ref().is_some_and(|state| state.id == self.id) {
            *state = None;
        }
    }
}

/// Reports job `id`'s progress until it ends, then `None` once.
fn tick(id: u64) {
    loop {
        std::thread::sleep(TICK);
        let progress = match state().as_ref() {
            Some(state) if state.id == id => {
                (state.started.elapsed() >= SHOW_AFTER).then(|| state.progress())
            }
            _ => break,
        };
        if progress.is_some() {
            notify(progress);
        }
    }
    notify(None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;

    fn words(n: usize) -> String {
        (0..n)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn pieces_join_back_into_the_text_and_end_at_sentences() {
        let text = format!(
            "{}. {}! {}? {}",
            words(150),
            words(150),
            words(150),
            words(30)
        );
        let pieces = split(&text, 400);
        assert_eq!(pieces.concat(), text);
        assert_eq!(pieces.len(), 2);
        assert!(
            pieces[0].ends_with("! "),
            "{:?}",
            &pieces[0][pieces[0].len() - 8..]
        );
        assert!(
            pieces
                .iter()
                .all(|piece| piece.split_whitespace().count() <= 400)
        );
    }

    #[test]
    fn a_short_text_is_one_piece_and_a_long_sentence_cuts_between_words() {
        assert_eq!(
            split("Привет, как дела? Хорошо.", 400),
            ["Привет, как дела? Хорошо."]
        );
        let long = words(950);
        let pieces = split(&long, 400);
        assert_eq!(pieces.concat(), long);
        let counts: Vec<usize> = pieces
            .iter()
            .map(|p| p.split_whitespace().count())
            .collect();
        assert_eq!(counts, [400, 400, 150]);
        assert!(split("", 400).is_empty());
    }

    #[test]
    fn closing_quotes_and_line_breaks_end_sentences() {
        let text = "Он сказал «да.» Потом\nушёл.";
        assert_eq!(
            sentence_ends(text),
            ["Он сказал «да.» ".len(), "Он сказал «да.» Потом\n".len()]
        );
    }

    #[test]
    fn each_piece_is_cleaned_and_a_failed_one_keeps_its_text() {
        let text = format!("{}. {}. {}.", words(300), words(300), words(300));
        let mut calls = 0;
        let cleaned = clean(&text, &AtomicBool::new(false), |piece, _, _| {
            calls += 1;
            if calls == 2 {
                Err(eyre!("timed out"))
            } else {
                Ok(piece.to_uppercase())
            }
        });
        let pieces = split(&text, PIECE_WORDS);
        assert_eq!(calls, 3);
        assert_eq!(
            cleaned.text,
            format!(
                "{} {} {}",
                pieces[0].trim_end().to_uppercase(),
                pieces[1].trim_end(),
                pieces[2].trim_end().to_uppercase()
            )
        );
        assert_eq!(
            cleaned.failed.as_deref(),
            Some("1 of 3 pieces kept as transcribed: timed out")
        );
        assert!(!cleaned.early);
    }

    #[test]
    fn stopping_keeps_the_cleaned_pieces_and_the_rest_as_transcribed() {
        let text = format!("{}. {}. {}.", words(300), words(300), words(300));
        let pieces = split(&text, PIECE_WORDS);
        let stop = AtomicBool::new(false);
        let mut calls = 0;
        let cleaned = clean_pieces(&pieces, &stop, None, &mut |piece, stop, _| {
            calls += 1;
            if calls == 2 {
                // Paste now while the second piece is cleaned.
                stop.store(true, Ordering::Release);
                return Err(eyre!("cancelled"));
            }
            Ok(piece.to_uppercase())
        });
        assert_eq!(calls, 2);
        assert_eq!(
            cleaned.text,
            format!(
                "{} {}{}",
                pieces[0].trim_end().to_uppercase(),
                pieces[1],
                pieces[2]
            )
        );
        assert!(cleaned.early);
        assert_eq!(cleaned.failed, None);
    }

    #[test]
    fn every_piece_failing_reports_the_first_reason() {
        let cleaned = clean("Один. Два.", &AtomicBool::new(false), |_, _, _| {
            Err(eyre!("no server"))
        });
        assert_eq!(cleaned.text, "Один. Два.");
        assert_eq!(cleaned.failed.as_deref(), Some("no server"));
    }
}
