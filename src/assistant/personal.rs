//! What of the person's own cleanup setup a model reads to answer why a
//! word came out as it did, while Assistant › Settings › Your current
//! settings is on: the cleanup instructions (the Global mode's, each other
//! mode's, Files' own) with the replacements modes make, and the
//! Vocabulary entries the question and the turns before it name, with how
//! many there are. Never the whole Vocabulary; a turn that carries any of
//! it never searches the web (`worker::offline`).

use crate::app_settings::AppSettings;
use crate::dictation::vocabulary::VocabularyEntry;

/// The most characters of one set of instructions.
const INSTRUCTIONS_CHARS: usize = 4_000;
/// The most Vocabulary entries a question reads.
const ENTRIES: usize = 30;
/// The most replacements a mode lists.
const REPLACEMENTS: usize = 20;

fn capped(text: &str, chars: usize) -> String {
    let text = text.trim();
    match text.char_indices().nth(chars) {
        Some((cut, _)) => format!("{}… (cut)", &text[..cut]),
        None => text.to_owned(),
    }
}

/// Every set of cleanup instructions dictation and Files use, as one block.
pub fn instructions(settings: &AppSettings) -> String {
    let processing = &settings.dictation_processing;
    let global = &processing.default_mode;
    let mut block = String::from("<cleanup_instructions>\n");
    block.push_str(&format!(
        "Global mode (Output › AI cleanup › Instructions), Clean up with AI {}:\n{}\n",
        if global.post_processing.enabled {
            "on"
        } else {
            "off"
        },
        capped(&global.post_processing.prompt, INSTRUCTIONS_CHARS)
    ));
    block.push_str(&replacements(&global.replacements));
    for mode in &processing.modes {
        block.push_str(&format!(
            "\nMode \"{}\" (its own apps), Clean up with AI {}:\n",
            mode.name.trim(),
            if mode.post_processing.enabled {
                "on"
            } else {
                "off"
            },
        ));
        if !mode.post_processing.prompt.trim().is_empty() {
            block.push_str(&capped(&mode.post_processing.prompt, INSTRUCTIONS_CHARS));
            block.push('\n');
        }
        block.push_str(&replacements(&mode.replacements));
        if !mode.vocabulary.is_empty() {
            block.push_str(&format!("{} terms of its own\n", mode.vocabulary.len()));
        }
    }
    if settings.labs.files {
        if settings.files.use_default_prompt {
            block.push_str("\nFiles cleanup follows the Global mode's instructions.\n");
        } else {
            block.push_str(&format!(
                "\nFiles cleanup's own instructions:\n{}\n",
                capped(&settings.files.prompt, INSTRUCTIONS_CHARS)
            ));
        }
    }
    block.push_str("</cleanup_instructions>\n");
    block
}

fn replacements(replacements: &[crate::dictation::text_replacements::TextReplacement]) -> String {
    if replacements.is_empty() {
        return String::new();
    }
    let mut lines = String::from("Replacements (said → written):\n");
    for replacement in replacements.iter().take(REPLACEMENTS) {
        lines.push_str(&format!(
            "\"{}\" → \"{}\"\n",
            replacement.matched_phrase.trim(),
            replacement.output.trim()
        ));
    }
    if replacements.len() > REPLACEMENTS {
        lines.push_str(&format!("and {} more\n", replacements.len() - REPLACEMENTS));
    }
    lines
}

/// The words of `text`, lowercased, at least two characters.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect()
}

/// Whether `entry` bears on `text`: its term or a "sounds like" spelling
/// is in it, or a word of the term or of a spelling is.
fn names(entry: &VocabularyEntry, text: &str, said: &[String]) -> bool {
    std::iter::once(&entry.term)
        .chain(&entry.sounds_like)
        .any(|form| {
            let form = form.trim().to_lowercase();
            !form.is_empty()
                && (text.contains(&form) || words(&form).iter().any(|word| said.contains(word)))
        })
}

/// The Vocabulary entries `texts` (the question, then the turns before it)
/// name, at most 30, the question's first, with how many there are in all;
/// and how many of them it holds.
pub fn vocabulary(settings: &AppSettings, texts: &[&str]) -> (String, usize) {
    let all = &settings.vocabulary;
    let mut found: Vec<&VocabularyEntry> = Vec::new();
    for text in texts {
        let lower = text.to_lowercase();
        let said = words(text);
        for entry in all {
            if found.len() < ENTRIES
                && !found.iter().any(|known| std::ptr::eq(*known, entry))
                && names(entry, &lower, &said)
            {
                found.push(entry);
            }
        }
    }
    let mut block = format!(
        "<vocabulary total=\"{}\">\nThe entries the question and the conversation name; the rest aren't shown.\n",
        all.len()
    );
    if found.is_empty() {
        block.push_str("None of the entries is named.\n");
    }
    let named = found.len();
    for entry in found {
        block.push_str(entry.term.trim());
        if !entry.sounds_like.is_empty() {
            block.push_str(&format!(" (sounds like: {})", entry.sounds_like.join(", ")));
        }
        block.push('\n');
    }
    block.push_str("</vocabulary>\n");
    (block, named)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(term: &str, sounds_like: &[&str]) -> VocabularyEntry {
        VocabularyEntry {
            term: term.into(),
            sounds_like: sounds_like.iter().map(|text| (*text).into()).collect(),
        }
    }

    #[test]
    fn only_the_entries_the_words_name_go_in() {
        let settings = AppSettings {
            vocabulary: vec![
                entry("AI cleanup", &["эй ай клинап", "AI"]),
                entry("Kubernetes", &["кубер"]),
                entry("Claude Code", &[]),
            ],
            ..AppSettings::default()
        };
        let (block, named) = vocabulary(
            &settings,
            &["Если я просто скажу AI, он может AI cleanup вставить"],
        );
        assert_eq!(named, 1);
        assert!(block.starts_with("<vocabulary total=\"3\">"));
        assert!(block.contains("AI cleanup (sounds like: эй ай клинап, AI)\n"));
        assert!(!block.contains("Kubernetes"));
        assert!(!block.contains("Claude Code"));
        let (none, _) = vocabulary(&settings, &["почему так"]);
        assert!(none.contains("None of the entries is named."));
    }

    #[test]
    fn every_mode_says_its_instructions_and_replacements() {
        let mut settings = AppSettings::default();
        settings
            .dictation_processing
            .default_mode
            .post_processing
            .prompt = "Fix punctuation.".into();
        let mut code = settings.dictation_processing.default_mode.clone();
        code.name = "Code".into();
        code.post_processing.prompt = "Keep identifiers.".into();
        code.replacements = vec![crate::dictation::text_replacements::TextReplacement {
            matched_phrase: "AI".into(),
            output: "AI cleanup".into(),
        }];
        settings.dictation_processing.modes = vec![code];
        let block = instructions(&settings);
        assert!(block.contains("Global mode (Output › AI cleanup › Instructions)"));
        assert!(block.contains("Fix punctuation."));
        assert!(block.contains("Mode \"Code\""));
        assert!(block.contains("\"AI\" → \"AI cleanup\""));
        assert!(block.ends_with("</cleanup_instructions>\n"));
    }
}
