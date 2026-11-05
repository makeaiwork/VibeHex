//! Labs › Russian vibe coding: one switch that writes the Russian
//! vibe-coding preset in `docs/vocabulary/` into the person's own settings
//! (shared vocabulary, Global's word fixes, Global's cleanup instructions,
//! and Clean up with AI on), turns on every other Lab VibeHex added (the
//! starred rows: Clean up on your own server, Vocabulary text view, Add to
//! vocabulary, Cleanup progress, Files, Stop shortcut, those this Hex has),
//! and takes back, when turned off, only what still reads as the preset
//! wrote it; those Labs stay on, since a server, key, or shortcut set up
//! there is the person's own. Everything it writes stays editable
//! in Vocabulary and Output like anything typed there.

use crate::app_settings::{AppSettings, DictationPostProcessing};
use crate::dictation::text_replacements::TextReplacement;
use crate::dictation::vocabulary::{self, VocabularyEntry};
use crate::labs::Feature;

const VOCABULARY: &str = include_str!("../../docs/vocabulary/vibe-coding-ru.txt");
const REPLACEMENTS: &str = include_str!("../../docs/vocabulary/replacements-ru.json");
const INSTRUCTIONS: &str = include_str!("../../docs/vocabulary/cleanup-instructions-ru.txt");

/// Writes the preset into `settings` (`on`) or takes it back out.
pub fn russian_vibe_coding(settings: &mut AppSettings, on: bool) {
    if on {
        add(settings);
    } else {
        take_back(settings);
    }
}

fn add(settings: &mut AppSettings) {
    for (term, sounds_like) in vocabulary::parse_list(VOCABULARY) {
        vocabulary::add_clamped(&mut settings.vocabulary, &term, &sounds_like);
    }
    let mode = &mut settings.dictation_processing.default_mode;
    for fix in replacements() {
        let taken = mode
            .replacements
            .iter()
            .any(|kept| same(&kept.matched_phrase, &fix.matched_phrase));
        if !taken {
            mode.replacements.push(fix);
        }
    }
    let rewrite = &mut mode.post_processing;
    let instructions = INSTRUCTIONS.trim();
    let prompt = rewrite.prompt.trim();
    if prompt.is_empty() || prompt == default_prompt() {
        rewrite.prompt = instructions.to_owned();
    } else if !prompt.contains(instructions) {
        rewrite.prompt = format!("{prompt}\n\n{instructions}");
    }
    rewrite.enabled = true;
    for feature in vibehex_labs() {
        feature.set(settings, true);
    }
}

/// The other Labs VibeHex added (the starred rows) that this Hex has and
/// no organization locks: the preset turns them all on.
fn vibehex_labs() -> impl Iterator<Item = Feature> {
    Feature::available().filter(|feature| {
        feature.vibehex() && *feature != Feature::RussianVibeCoding && feature.managed().is_none()
    })
}

fn take_back(settings: &mut AppSettings) {
    let preset = preset_entries();
    settings.vocabulary.retain(|entry| !preset.contains(entry));
    let mode = &mut settings.dictation_processing.default_mode;
    let fixes = replacements();
    mode.replacements.retain(|kept| !fixes.contains(kept));
    let rewrite = &mut mode.post_processing;
    let instructions = INSTRUCTIONS.trim();
    let prompt = rewrite.prompt.trim();
    if prompt == instructions {
        // The preset's instructions were all there was: back to Hex's own
        // prompt, and cleanup off, as before the preset turned it on.
        rewrite.prompt = default_prompt().to_owned();
        rewrite.enabled = false;
    } else if prompt.contains(instructions) {
        rewrite.prompt = prompt.replace(instructions, "").trim().to_owned();
    }
}

/// The preset's vocabulary as `add` writes it into an empty list.
fn preset_entries() -> Vec<VocabularyEntry> {
    let mut list = Vec::new();
    for (term, sounds_like) in vocabulary::parse_list(VOCABULARY) {
        vocabulary::add_clamped(&mut list, &term, &sounds_like);
    }
    list
}

fn replacements() -> Vec<TextReplacement> {
    serde_json::from_str(REPLACEMENTS).expect("docs/vocabulary/replacements-ru.json is valid")
}

fn default_prompt() -> String {
    DictationPostProcessing::default().prompt
}

fn same(left: &str, right: &str) -> bool {
    left.trim().to_lowercase() == right.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(matched: &str, output: &str) -> TextReplacement {
        TextReplacement {
            matched_phrase: matched.into(),
            output: output.into(),
        }
    }

    #[test]
    fn turning_it_on_then_off_leaves_a_fresh_install_as_it_was() {
        let fresh = AppSettings::default();
        let mut settings = fresh.clone();
        russian_vibe_coding(&mut settings, true);
        let mode = &settings.dictation_processing.default_mode;
        assert!(vocabulary::contains(&settings.vocabulary, "Claude Code"));
        assert_eq!(mode.replacements.len(), replacements().len());
        assert_eq!(mode.post_processing.prompt, INSTRUCTIONS.trim());
        assert!(mode.post_processing.enabled);
        assert!(settings.labs.ai_cleanup_server);
        assert!(settings.labs.vocabulary_paste_list);

        // On again adds nothing twice.
        let once = settings.clone();
        russian_vibe_coding(&mut settings, true);
        assert_eq!(settings, once);

        russian_vibe_coding(&mut settings, false);
        // The Labs it turned on stay on; everything else is as it was.
        assert!(settings.labs.ai_cleanup_server);
        assert!(settings.labs.vocabulary_paste_list);
        settings.labs = fresh.labs.clone();
        assert_eq!(settings, fresh);
    }

    #[test]
    fn it_turns_on_every_starred_lab_this_hex_has() {
        let mut settings = AppSettings::default();
        russian_vibe_coding(&mut settings, true);
        // Its own switch is the Lab's, not the preset's.
        let starred: Vec<Feature> = Feature::available()
            .filter(|feature| feature.vibehex() && *feature != Feature::RussianVibeCoding)
            .collect();
        assert!(starred.contains(&Feature::AiCleanupServer));
        for feature in starred {
            assert!(feature.on(&settings), "{} stayed off", feature.name());
        }
        // Labs Hex itself has stay as they were.
        for feature in Feature::ALL.into_iter().filter(|f| !f.vibehex()) {
            assert_eq!(
                feature.on(&settings),
                feature.on(&AppSettings::default()),
                "{} changed",
                feature.name()
            );
        }
    }

    #[test]
    fn it_keeps_what_the_person_wrote_and_changed() {
        let mut settings = AppSettings::default();
        vocabulary::add(&mut settings.vocabulary, "Kit", &[]).unwrap();
        let mode = &mut settings.dictation_processing.default_mode;
        mode.replacements.push(fix("тот есть", "то бишь"));
        mode.replacements.push(fix("гугл", "Google"));
        mode.post_processing.prompt = "Keep it short.".into();
        mode.post_processing.enabled = true;

        russian_vibe_coding(&mut settings, true);
        let mode = &settings.dictation_processing.default_mode;
        // The person's own fix for the same phrase wins.
        assert!(mode.replacements.contains(&fix("тот есть", "то бишь")));
        assert!(!mode.replacements.contains(&fix("тот есть", "то есть")));
        assert!(
            mode.post_processing
                .prompt
                .starts_with("Keep it short.\n\n")
        );
        assert!(mode.post_processing.prompt.ends_with(INSTRUCTIONS.trim()));

        // A preset term the person gave a spelling of their own stays.
        vocabulary::add(&mut settings.vocabulary, "GitHub", &["гидхаб".into()]).unwrap();

        russian_vibe_coding(&mut settings, false);
        let mode = &settings.dictation_processing.default_mode;
        assert_eq!(
            mode.replacements,
            vec![fix("тот есть", "то бишь"), fix("гугл", "Google")]
        );
        assert_eq!(mode.post_processing.prompt, "Keep it short.");
        assert!(mode.post_processing.enabled);
        let terms: Vec<_> = settings
            .vocabulary
            .iter()
            .map(|e| e.term.as_str())
            .collect();
        assert_eq!(terms, ["Kit", "GitHub"]);
    }

    #[test]
    fn the_preset_files_load() {
        assert!(preset_entries().len() >= 60);
        assert!(
            replacements()
                .iter()
                .all(|fix| !fix.matched_phrase.is_empty())
        );
        assert!(!INSTRUCTIONS.trim().is_empty());
    }
}
