//! Labs › Russian vibe coding: one switch that writes the Russian
//! vibe-coding preset into the person's own settings (the shared
//! vocabulary: the owner's own list, `presets/deniskey.json`; Global's
//! word fixes and cleanup instructions from `docs/vocabulary/`; and Clean
//! up with AI on), turns on every other Lab VibeHex added (the
//! starred rows: Clean up on your own server, Vocabulary text view, Add to
//! vocabulary, Cleanup progress, Files, those this Hex has; not Stop
//! shortcut, which takes a key and is left to the person) and Hex's
//! Vocabulary in meeting notes,
//! and takes back, when turned off, only what still reads as the preset
//! wrote it (today's list, or the one it wrote through 3.3.0); those Labs
//! stay on, since a server, key, or shortcut set up
//! there is the person's own. Everything it writes stays editable
//! in Vocabulary and Output like anything typed there.

use crate::app_settings::{AppSettings, DictationPostProcessing};
use crate::dictation::text_replacements::TextReplacement;
use crate::dictation::vocabulary::{self, VocabularyEntry};
use crate::labs::Feature;

/// The owner's preset (`scripts/export-preset`). Its Vocabulary is the
/// starter list, the same terms "Set me up like Denis" offers
/// (`assistant::setup::preset`): one file, so the two can't differ.
const PRESET: &str = include_str!("../../presets/deniskey.json");
/// The starter list before it became the preset's, for [`take_back`].
const EARLIER: &str = include_str!("presets/starter-through-3.3.0.txt");
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

/// The starter list: the preset's Vocabulary, in its order.
pub fn starter() -> Vec<VocabularyEntry> {
    #[derive(serde::Deserialize)]
    struct Shipped {
        vocabulary: Vec<VocabularyEntry>,
    }
    serde_json::from_str::<Shipped>(PRESET)
        .expect("presets/deniskey.json is valid")
        .vocabulary
}

fn add(settings: &mut AppSettings) {
    for entry in starter() {
        vocabulary::add_clamped(&mut settings.vocabulary, &entry.term, &entry.sounds_like);
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

/// The other Labs VibeHex added (the starred rows), and Hex's Vocabulary
/// in meeting notes, so the preset's terms reach meeting notes too, that
/// this Hex has and no organization locks: the preset turns them all on,
/// except Stop shortcut, which takes a key (Return) and is left to the
/// person.
fn vibehex_labs() -> impl Iterator<Item = Feature> {
    Feature::available().filter(|feature| {
        (feature.vibehex() || *feature == Feature::MeetingNotesVocabulary)
            && !matches!(feature, Feature::RussianVibeCoding | Feature::StopShortcut)
            && feature.managed().is_none()
    })
}

/// Takes back what the preset wrote and the person didn't change: an
/// entry exactly as the starter list writes it, today's or the one before
/// it (someone who turned this on then has those). An entry they edited,
/// or added themselves, stays.
fn take_back(settings: &mut AppSettings) {
    let mut preset = written(starter());
    let earlier = vocabulary::parse_list(EARLIER)
        .into_iter()
        .map(|(term, sounds_like)| VocabularyEntry { term, sounds_like });
    preset.extend(written(earlier));
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

/// A starter list as `add` writes it into an empty list.
fn written(list: impl IntoIterator<Item = VocabularyEntry>) -> Vec<VocabularyEntry> {
    let mut all = Vec::new();
    for entry in list {
        vocabulary::add_clamped(&mut all, &entry.term, &entry.sounds_like);
    }
    all
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
        // Its own switch is the Lab's, not the preset's, and Stop shortcut
        // is left to the person.
        let starred: Vec<Feature> = Feature::available()
            .filter(|feature| {
                feature.vibehex()
                    && !matches!(feature, Feature::RussianVibeCoding | Feature::StopShortcut)
            })
            .collect();
        assert!(starred.contains(&Feature::AiCleanupServer));
        for feature in starred {
            assert!(feature.on(&settings), "{} stayed off", feature.name());
        }
        assert!(!Feature::StopShortcut.on(&settings));
        // Vocabulary in meeting notes carries the terms into the notes.
        assert!(Feature::MeetingNotesVocabulary.on(&settings));
        // Other Labs Hex itself has stay as they were.
        for feature in Feature::ALL
            .into_iter()
            .filter(|f| !f.vibehex() && *f != Feature::MeetingNotesVocabulary)
        {
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

    fn terms(settings: &AppSettings) -> Vec<&str> {
        settings
            .vocabulary
            .iter()
            .map(|entry| entry.term.as_str())
            .collect()
    }

    fn listed(text: &str) -> Vec<VocabularyEntry> {
        vocabulary::parse_list(text)
            .into_iter()
            .map(|(term, sounds_like)| VocabularyEntry { term, sounds_like })
            .collect()
    }

    /// One list, read from one file: what the switch writes on a fresh
    /// install is the Vocabulary "Set me up like Denis" offers, term for
    /// term with its spellings, and the docs' copy to paste reads the same.
    #[test]
    fn the_starter_list_is_the_presets_vocabulary() {
        let starter = starter();
        // The count the Labs row, the guide, and docs/features state.
        assert_eq!(starter.len(), 90);
        let mut settings = AppSettings::default();
        russian_vibe_coding(&mut settings, true);
        assert_eq!(settings.vocabulary, starter);
        let offered: Vec<VocabularyEntry> = crate::assistant::setup::preset::denis()
            .vocabulary
            .into_iter()
            .map(|term| serde_json::from_value(term).unwrap())
            .collect();
        assert_eq!(offered, starter);
        // Names that read as ordinary words install as they are; the
        // owner's own app never ships (`scripts/export-preset`).
        for term in ["New Session", "AskTool", "Cancel", "Labs"] {
            assert!(vocabulary::contains(&settings.vocabulary, term), "{term}");
        }
        assert!(!vocabulary::contains(&settings.vocabulary, "Monki"));
        assert_eq!(
            listed(include_str!("../../docs/vocabulary/vibe-coding-ru.txt")),
            starter,
            "docs/vocabulary/vibe-coding-ru.txt: scripts/export-preset writes it"
        );
    }

    /// Someone who turned it on through 3.3.0 has the 80 terms it wrote
    /// then. Off takes back the ones they left alone, those today's list
    /// no longer has among them, and keeps what they added or edited; on
    /// again writes today's list beside those.
    #[test]
    fn the_list_it_wrote_before_is_still_taken_back() {
        let earlier = written(listed(EARLIER));
        assert_eq!(earlier.len(), 80);
        let mut settings = AppSettings::default();
        russian_vibe_coding(&mut settings, true);
        settings.vocabulary = earlier.clone();
        // A term of their own, and a starter term given a spelling.
        vocabulary::add(&mut settings.vocabulary, "Kit", &[]).unwrap();
        vocabulary::add(&mut settings.vocabulary, "Trello", &["трелло борд".into()]).unwrap();
        assert!(vocabulary::contains(&settings.vocabulary, "Postman"));

        russian_vibe_coding(&mut settings, false);
        assert_eq!(terms(&settings), ["Trello", "Kit"]);

        russian_vibe_coding(&mut settings, true);
        assert_eq!(settings.vocabulary.len(), 92);
        for term in ["Trello", "Kit", "Claude Code", "New Session", "AskTool"] {
            assert!(vocabulary::contains(&settings.vocabulary, term), "{term}");
        }
        assert!(!vocabulary::contains(&settings.vocabulary, "Postman"));

        russian_vibe_coding(&mut settings, false);
        assert_eq!(terms(&settings), ["Trello", "Kit"]);
    }

    /// With both lists in Vocabulary (the earlier one, then the preset's
    /// terms added by Assistant), off takes back both and nothing else.
    #[test]
    fn both_lists_side_by_side_are_taken_back() {
        let mut settings = AppSettings::default();
        russian_vibe_coding(&mut settings, true);
        settings.vocabulary = written(listed(EARLIER));
        vocabulary::add(&mut settings.vocabulary, "Kit", &[]).unwrap();
        for entry in starter() {
            if !vocabulary::contains(&settings.vocabulary, &entry.term) {
                settings.vocabulary.push(entry);
            }
        }
        assert_eq!(settings.vocabulary.len(), 80 + 1 + 16);
        russian_vibe_coding(&mut settings, false);
        assert_eq!(terms(&settings), ["Kit"]);
    }

    #[test]
    fn the_preset_files_load() {
        assert!(written(starter()).len() >= 60);
        assert!(
            replacements()
                .iter()
                .all(|fix| !fix.matched_phrase.is_empty())
        );
        assert!(!INSTRUCTIONS.trim().is_empty());
    }
}
