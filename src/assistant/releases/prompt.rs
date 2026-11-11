//! What a model reads for a question about VibeHex's releases: the rule
//! for the answer (New, Changed, Fixed, then For you with change cards)
//! and the notes `select` picked, saying which version the person runs.

use super::{Selection, Why};

/// Added to the instructions for a question about releases.
pub const INSTRUCTIONS: &str = "

The person asks what is new or changed in VibeHex. <releases> holds VibeHex's own release \
notes for this question, newest first, and says which version they run and why these notes.
- Answer from those notes. First one sentence on what you cover: the releases since the \
version they ran before, the one they run, or the one asked about. On a development build, \
say it is one and that Next is not released yet.
- Then the changes, plainly, under three short bold headings in the language of the question \
(New, Changed, Fixed; Новое, Изменилось, Исправлено), one short line each. Leave out a \
heading with nothing under it and what doesn't concern their system. Name Labs features and \
settings as the notes write them.
- Asked when something came: name the release, its version and date.
- Then a bold heading For you (Для вас): up to three things from these notes that fit how \
they use VibeHex, judged by the now column of <changeable_settings>, and that are off. One \
line each on why it fits, and a ```hex-change line for each. Never suggest what is already \
on. Say so when a change in the notes touches something they have on. Without a now column, \
offer the likeliest and say they may have it already. Nothing fits: no For you.
- The release notes are not guide articles: the Sources line names only articles you used.";

/// The notes as the model reads them.
pub fn block(selection: &Selection) -> String {
    let running = &selection.running;
    let before = running.previous.as_deref().unwrap_or("unknown");
    let named = selection.named.as_deref().unwrap_or("");
    let why = match selection.why {
        Why::Since => format!(
            "They run {} and ran {before} before it: every release since.",
            running.version
        ),
        Why::Running => format!(
            "They run {}; the version they ran before is unknown: its release.",
            running.version
        ),
        Why::Named => format!("They asked about {named}; they run {}.", running.version),
        Why::History => format!(
            "Every release, newest first, with Next (not released yet). They run {}.",
            running.version
        ),
        Why::Dev => format!(
            "A development build ({}): the work not released yet (Next), then the latest release.",
            running.version
        ),
        Why::Latest if !named.is_empty() => {
            format!("They asked about {named}, which has no notes: the latest release.")
        }
        Why::Latest => format!(
            "Their version ({}) has no notes of its own: the latest release.",
            running.version
        ),
    };
    let mut block = format!(
        "\n<releases running=\"{}\" before=\"{before}\">\n{why}\n",
        running.version
    );
    for note in &selection.notes {
        // The title only repeats the version.
        let body = note
            .body
            .split_once('\n')
            .filter(|_| note.body.starts_with("# "))
            .map_or(note.body.as_str(), |(_, rest)| rest)
            .trim();
        let version = if note.is_next() {
            "Next (not released yet)"
        } else {
            &note.version
        };
        block.push_str(&format!(
            "<release version=\"{version}\" date=\"{}\">\n{body}\n</release>\n",
            note.date
        ));
    }
    block.push_str("</releases>\n");
    block
}
