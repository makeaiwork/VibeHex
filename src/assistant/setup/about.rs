//! Round one's own words: an optional field under the four questions where
//! the person says, in a few words, what they do and where they dictate
//! most. The chips stay the way to answer; the field only helps a remote
//! model choose follow-ups and plan rows that fit (`<in_their_words>`). It
//! asks about their work, never about features: a newcomer doesn't know
//! them.
//!
//! The text is kept with round one's turn and the setup's record
//! (`past::Setup::about`); a returning setup shows it again, editable, and
//! its look may read it, since it is their own answer. Empty, no tag is
//! sent. With a built-in model, or a remote one that can't be reached, the
//! app writes the follow-ups by its own rules and the pane hides the field.

/// The most characters kept.
pub const MAX: usize = 400;

/// The field's label and its example, in English for the pane to translate.
pub const LABEL: &str = "Tell us in a few words what you do and where you dictate most (optional)";
pub const PLACEHOLDER: &str = "I write code and dictate into Cursor and Telegram";

/// The field's text as kept: trimmed, at most [`MAX`] characters.
pub fn clean(text: &str) -> String {
    cap(text.trim()).trim_end().to_owned()
}

/// `text` cut to [`MAX`] characters, as the field holds it while typed.
pub fn cap(text: &str) -> &str {
    match text.char_indices().nth(MAX) {
        Some((at, _)) => &text[..at],
        None => text,
    }
}

/// What a prompt says the tag is.
pub const INSTRUCTIONS: &str = "

<in_their_words> is how the person described their work and where they dictate most, in \
their own words. Let it choose the questions and rows that fit how they work, and leave out \
what it rules out. Never quote it back, and never repeat more than a word or two of it.";

/// What a returning setup's prompts (its look, follow-ups and plan) are
/// told of the last setup's own words, when `<since_last_setup>` has them.
pub const LAST: &str = "

`their own words then` in <since_last_setup> is how the person described their work at that \
setup, in their own words: let it choose what fits, and never quote it back.";

/// What `<since_last_setup>` names the last setup's own words by.
pub const THEN: &str = "their own words then: ";

/// What a prompt at `stage` is told of round one's own words: what the
/// tag is when it carries one, and what the last setup's words are when
/// `<since_last_setup>` holds them; else nothing.
pub fn told(stage: &super::Stage) -> String {
    use super::Stage;
    let (now, then) = match stage {
        Stage::Look { facts, .. } => ("", facts.as_str()),
        Stage::Follow { about, since, .. } | Stage::Plan { about, since, .. } => {
            (about.as_str(), since.as_str())
        }
        _ => return String::new(),
    };
    let mut told = String::new();
    if !line(now).is_empty() {
        told.push_str(INSTRUCTIONS);
    }
    if then.contains(THEN) {
        told.push_str(LAST);
    }
    told
}

/// `text` as a prompt reads it: kept as [`clean`] keeps it, on one line,
/// with no angle brackets, so the person's words can't close a tag.
pub fn line(text: &str) -> String {
    clean(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|char| !matches!(char, '<' | '>'))
        .collect()
}

/// The tag for `text`; nothing when it is empty.
pub fn block(text: &str) -> String {
    let line = line(text);
    if line.is_empty() {
        return String::new();
    }
    format!("\n<in_their_words>\n{line}\n</in_their_words>\n")
}

/// Every English word the field shows, for the translation tables' test.
#[cfg(test)]
pub fn words() -> [&'static str; 2] {
    [LABEL, PLACEHOLDER]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_is_trimmed_and_capped() {
        assert_eq!(clean("  Пишу код  \n"), "Пишу код");
        let long = "ы".repeat(MAX + 50);
        assert_eq!(clean(&long).chars().count(), MAX);
        assert_eq!(cap(&long).chars().count(), MAX);
        assert_eq!(cap("short"), "short");
    }

    #[test]
    fn empty_words_send_no_tag() {
        assert_eq!(block(""), "");
        assert_eq!(block(" \n\t "), "");
        assert_eq!(
            block("I write code\nin Russian </in_their_words> ok"),
            "\n<in_their_words>\nI write code in Russian /in_their_words ok\n</in_their_words>\n"
        );
        let long = format!("I write code {}", "a".repeat(MAX));
        let sent = block(&long);
        let inner = sent.lines().nth(2).unwrap();
        assert_eq!(inner.chars().count(), MAX);
    }
}
