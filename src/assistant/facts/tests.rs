//! Tests of the facts and their one formatter.

use super::text::{self, Sent};
use super::{Cleanup, Fact, Facts, Fallback, FilesCleanup, Kind, Reads, State};
use crate::app_settings::{AppSettings, CleanupRunsOn, MeetingSummaries};
use crate::desktop::hardware::{Hardware, Pointer, Screens};
use crate::dictation::vocabulary::VocabularyEntry;

/// Settings a model may read.
fn sent() -> AppSettings {
    let mut settings = AppSettings::default();
    settings.assistant.send_settings = true;
    settings
}

/// The owner's sample setup, read.
fn sample() -> AppSettings {
    let mut settings = sent();
    crate::assistant::setup::sample(&mut settings);
    settings
}

/// A Mac mini with one outside screen and a Magic Mouse.
fn mini() -> Hardware {
    Hardware {
        screens: Some(Screens {
            built_in: false,
            external: 1,
            notch: false,
        }),
        pointers: Some(vec![Pointer {
            name: "Magic Mouse".into(),
            extra_buttons: false,
        }]),
        model: Some("Mac16,11".into()),
    }
}

fn told(settings: &AppSettings, state: &State) -> String {
    text::told(&Facts::of(settings, state))
}

#[test]
fn a_fact_is_known_or_unknown_and_never_guessed() {
    assert_eq!(Fact::from(Some(false)), Fact::Known(false));
    assert_eq!(Fact::<bool>::from(None), Fact::Unknown);
    assert!(Fact::Known(false).is(&false));
    assert!(!Fact::<bool>::Unknown.is(&false));
    assert!(!Fact::<bool>::Unknown.is(&true));
    assert_eq!(Fact::<Vec<String>>::Unknown.known(), None);
}

#[test]
fn what_could_not_be_read_says_unknown_and_an_empty_list_says_none() {
    let unknown = told(&sent(), &State::default());
    for line in [
        "Speech models downloaded: unknown\n",
        "Codex signed in: unknown\n",
        "OpenCode running: unknown\n",
        "Computer model: unknown\n",
        "Screens: unknown\n",
        "Pointing devices: unknown\n",
    ] {
        assert!(unknown.contains(line), "{line:?} in:\n{unknown}");
    }
    assert!(!unknown.contains("downloaded: none"), "{unknown}");

    let read = State {
        os: "macos",
        installed: Fact::Known(Vec::new()),
        codex_signed_in: Fact::Known(false),
        opencode_running: Fact::Known(true),
        hardware: Hardware {
            pointers: Some(Vec::new()),
            ..mini()
        },
        ..State::default()
    };
    let known = told(&sent(), &read);
    for line in [
        "OS: macos\n",
        "Speech models downloaded: none\n",
        "Codex signed in: no\n",
        "OpenCode running: yes\n",
        "Computer model: Mac16,11\n",
        "Screens: 1 external, no notch\n",
        "Pointing devices: none found\n",
    ] {
        assert!(known.contains(line), "{line:?} in:\n{known}");
    }
    assert!(!known.contains("unknown"), "{known}");
}

#[test]
fn the_models_on_disk_are_named_as_the_picker_names_them() {
    let state = State {
        installed: Fact::Known(vec!["orukeet".into(), "parakeet_v3".into()]),
        ..State::default()
    };
    let facts = Facts::of(&sample(), &state);
    let downloaded = facts.speech.downloaded.known().expect("known");
    assert_eq!(downloaded.len(), 2);
    assert_eq!(facts.speech.model.id, "orukeet");
    let told = text::told(&facts);
    assert!(
        told.contains(
            "Speech model in use: Orukeet (orukeet); recognition language: Russian (ru)\n"
        ),
        "{told}"
    );
    assert!(
        told.contains(
            "Speech models downloaded: Orukeet (orukeet, in use), Parakeet v3 (parakeet_v3)\n"
        ),
        "{told}"
    );
}

#[test]
fn the_notes_writer_is_the_one_the_generator_would_use() {
    // The owner's case: a Codex writer, with Summaries left at "On this
    // Mac". The writer writes; the fallback isn't used.
    let mut settings = sample();
    settings.meetings.summaries = MeetingSummaries::Mac;
    let notes = super::notes(&settings);
    let writer = notes.writer.as_ref().expect("a writer");
    assert_eq!(writer.kind, Kind::Codex);
    assert_eq!(writer.id, "codex:gpt-6.1-terra");
    assert_eq!(notes.fallback, Fallback::AppleIntelligence);
    let told = told(&settings, &State::default());
    assert!(told.contains("Meeting notes: written by Codex"), "{told}");
    assert!(told.contains("is not used now."), "{told}");

    // With Labs › AI cleanup server off the generator doesn't use that
    // writer, so it isn't the writer here either.
    settings.labs.ai_cleanup_server = false;
    assert_eq!(super::notes(&settings).writer, None);
    let told = self::told(&settings, &State::default());
    assert!(
        told.contains(
            "Meeting notes: no writer chosen in meetings.notes.writer, so meetings.summaries \
             decides: On this Mac (Apple Intelligence).\n"
        ),
        "{told}"
    );

    // OpenCode writes without that lab.
    settings.meetings.notes.writer = Some("opencode".into());
    let writer = super::notes(&settings).writer.expect("OpenCode");
    assert_eq!(writer.kind, Kind::OpenCode);
}

#[test]
fn the_engines_are_the_ones_in_use_not_the_keys_saved() {
    let settings = sample();
    let facts = Facts::of(&settings, &State::default());
    let Cleanup::On(cleanup) = &facts.cleanup else {
        panic!("cleanup is on: {:?}", facts.cleanup);
    };
    assert_eq!(cleanup.kind, Kind::BuiltIn);
    assert_eq!(cleanup.id, "local:gemma-4-e4b-it");
    let FilesCleanup::Own(files) = &facts.files else {
        panic!("Files has its own engine: {:?}", facts.files);
    };
    assert_eq!(files.kind, Kind::Codex);
    // Assistant picked nothing of its own: dictation's engine.
    assert_eq!(&facts.assistant, cleanup);

    // Files without an engine of its own cleans up where dictation does.
    let mut plain = settings.clone();
    plain.files.cleanup_engine = None;
    assert_eq!(
        super::files(&plain),
        FilesCleanup::AsDictation(cleanup.clone())
    );
    plain.labs.files = false;
    assert_eq!(super::files(&plain), FilesCleanup::Off);

    // Clean up with AI off: no engine named for dictation, whatever is
    // saved for it.
    let mut off = settings.clone();
    off.dictation_processing
        .default_mode
        .post_processing
        .enabled = false;
    assert_eq!(super::cleanup(&off), Cleanup::Off);
    let told = told(&off, &State::default());
    assert!(
        told.contains("Dictation cleanup: off (Clean up with AI is off)\n"),
        "{told}"
    );
}

#[test]
fn a_shortcut_is_listed_with_its_keycaps_while_its_feature_is_on() {
    let mut settings = sample();
    let facts = Facts::of(&settings, &State::default());
    let keys: Vec<&str> = facts
        .shortcuts
        .iter()
        .map(|shortcut| shortcut.key)
        .collect();
    assert!(keys.contains(&"dictation_hotkey"), "{keys:?}");
    // Add to vocabulary from a selection is a Mac feature: elsewhere its
    // shortcut isn't one the person has.
    let here = |key: &str| crate::control::settings::unavailable(key).is_none();
    let quick_add = here("labs.vocabulary_quick_add") && here("vocabulary_add_hotkey");
    assert_eq!(
        keys.contains(&"vocabulary_add_hotkey"),
        quick_add,
        "{keys:?}"
    );
    let stored = serde_json::to_value(&settings).expect("settings");
    for shortcut in &facts.shortcuts {
        assert_eq!(
            Some(&shortcut.keys),
            super::keycaps(&stored, shortcut.key).as_ref(),
            "{}",
            shortcut.key
        );
        assert!(!shortcut.keys.is_empty(), "{}", shortcut.key);
    }
    // Each says what it is for: none is a bare "Shortcut", no two share a
    // name.
    let mut names: Vec<&str> = facts
        .shortcuts
        .iter()
        .map(|shortcut| shortcut.name.as_str())
        .collect();
    assert!(!names.contains(&"Shortcut"), "{names:?}");
    assert!(names.contains(&"Dictation"), "{names:?}");
    names.sort_unstable();
    let all = names.len();
    names.dedup();
    assert_eq!(names.len(), all, "{names:?}");

    // A feature turned off takes its shortcut out of the list.
    settings.labs.vocabulary_quick_add = false;
    let off = Facts::of(&settings, &State::default());
    assert!(
        off.shortcuts
            .iter()
            .all(|shortcut| shortcut.key != "vocabulary_add_hotkey")
    );
    // And one that is cleared isn't there.
    settings.labs.vocabulary_quick_add = true;
    settings.vocabulary_add_hotkey = None;
    let cleared = Facts::of(&settings, &State::default());
    assert!(
        cleared
            .shortcuts
            .iter()
            .all(|shortcut| shortcut.key != "vocabulary_add_hotkey")
    );
}

#[test]
fn labs_and_vocabulary_are_counted_from_the_settings() {
    let settings = sample();
    let facts = Facts::of(&settings, &State::default());
    assert_eq!(facts.vocabulary.terms, 94);
    let on = crate::labs::Feature::ALL
        .into_iter()
        .filter(|feature| feature.on(&settings))
        .count();
    assert_eq!(facts.labs_on.len(), on);
    assert!(on >= 4, "{:?}", facts.labs_on);
    let told = text::told(&facts);
    assert!(told.contains("Vocabulary: 94 terms; "), "{told}");
    assert!(
        told.contains(&format!("Labs on: {}\n", facts.labs_on.join(", "))),
        "{told}"
    );

    let mut bare = sent();
    for feature in crate::labs::Feature::ALL {
        feature.set(&mut bare, false);
    }
    let none = self::told(&bare, &State::default());
    assert!(none.contains("Vocabulary: 0 terms\n"), "{none}");
    assert!(none.contains("Labs on: none\n"), "{none}");
}

/// The Vocabulary line says who reads how much: the speech model only its
/// first terms while it transcribes, AI cleanup the whole list. Told only
/// "the first 64", a model answered that the other terms are lost.
#[test]
fn the_vocabulary_line_says_what_the_model_and_cleanup_each_read() {
    use crate::transcription::models::TranscriptionModelId;
    let mut settings = sent();
    settings.vocabulary = (0..97)
        .map(|n| VocabularyEntry {
            term: format!("Term {n}"),
            sounds_like: Vec::new(),
        })
        .collect();
    settings.transcription.model = TranscriptionModelId::Orukeet;
    settings.labs.vocabulary_biasing = true;
    let cleanup = |settings: &mut AppSettings, on: bool| {
        settings
            .dictation_processing
            .default_mode
            .post_processing
            .enabled = on;
    };
    let line = |settings: &AppSettings| {
        told(settings, &State::default())
            .lines()
            .find(|line| line.starts_with("Vocabulary: "))
            .expect("a Vocabulary line")
            .to_owned()
    };

    cleanup(&mut settings, true);
    let facts = Facts::of(&settings, &State::default());
    // The number the Vocabulary sheet shows for this model.
    let read = crate::dictation::vocabulary::terms_read(&settings.vocabulary, "parakeet");
    assert_eq!(read, 64);
    assert_eq!(facts.vocabulary.speech, Reads::Bias { read, on: true });
    assert_eq!(facts.vocabulary.unused(), 0);
    assert_eq!(
        line(&settings),
        "Vocabulary: 97 terms; while transcribing, Orukeet is biased toward only the first 64 \
         (Labs › Vocabulary biasing is on); AI cleanup is on and reads all 97 with their \
         sounds-like spellings, so every term is still corrected after transcription"
    );

    // Cleanup off: what the model doesn't take is used nowhere.
    cleanup(&mut settings, false);
    assert_eq!(
        Facts::of(&settings, &State::default()).vocabulary.unused(),
        33
    );
    let off = line(&settings);
    assert!(off.contains("only the first 64"), "{off}");
    assert!(
        off.ends_with("AI cleanup is off, so the other 33 are not used in dictation"),
        "{off}"
    );

    // Biasing off: a Parakeet-family model takes none.
    settings.labs.vocabulary_biasing = false;
    let unbiased = line(&settings);
    assert!(
        unbiased.contains("Orukeet uses none of them (Labs › Vocabulary biasing is off"),
        "{unbiased}"
    );
    assert!(unbiased.contains("the other 97 are not used"), "{unbiased}");

    // A short list fits whole; Whisper reads a hint with no switch; GigaAM
    // reads nothing.
    settings.labs.vocabulary_biasing = true;
    settings.vocabulary.truncate(10);
    let short = line(&settings);
    assert!(short.contains("biased toward all of them"), "{short}");
    assert!(short.ends_with("AI cleanup is off"), "{short}");
    settings.transcription.model = TranscriptionModelId::WhisperLargeV3Turbo;
    assert_eq!(
        Facts::of(&settings, &State::default()).vocabulary.speech,
        Reads::Prompt { read: 10 }
    );
    assert!(line(&settings).contains("reads all of them as a hint"));
    settings.transcription.model = TranscriptionModelId::GigaAmV3;
    assert_eq!(
        Facts::of(&settings, &State::default()).vocabulary.speech,
        Reads::None
    );
    assert!(line(&settings).contains("doesn't use them while transcribing"));
}

#[test]
fn settings_not_sent_means_no_state_and_no_value() {
    let mut settings = sample();
    settings.assistant.send_settings = false;
    let state = State {
        installed: Fact::Known(vec!["orukeet".into()]),
        hardware: mini(),
        ..State::default()
    };
    let facts = Facts::of(&settings, &state);
    let told = text::told(&facts);
    assert!(!told.contains("<state>"), "{told}");
    for word in ["Orukeet", "Codex", "Gemma", "Magic Mouse", "Mac16", "94"] {
        assert!(!told.contains(word), "{word} in:\n{told}");
    }
    assert!(told.contains("You don't know the person's current values"));
    // What this Mac can't use still goes: it names nothing of theirs.
    assert!(told.contains("Not on this Mac: "), "{told}");
    assert!(told.contains("Notch indicator"), "{told}");

    let knew = text::knew(&facts, &Sent::default());
    assert!(
        knew.contains("Your current settings weren't sent"),
        "{knew}"
    );
    assert!(!knew.contains("Orukeet"), "{knew}");
    assert!(!knew.contains("each changeable setting"), "{knew}");
}

#[test]
fn nothing_private_is_written_for_a_model_or_kept_in_the_record() {
    let mut settings = sample();
    let server = &mut settings.dictation_processing.server;
    server.url = "http://192.168.7.7:1234/v1".into();
    server.openrouter_model = "vendor/secret-model".into();
    server.custom_model = "my-own-model".into();
    settings.microphone = Some("Studio Mic of Anna".into());
    settings.vocabulary.push(VocabularyEntry {
        term: "Zyxterm".into(),
        sounds_like: vec!["зикстерм".into()],
    });
    let state = State {
        installed: Fact::Known(vec!["orukeet".into()]),
        codex_signed_in: Fact::Known(true),
        openrouter_key: Fact::Known(true),
        servers: Fact::Known(vec!["LM Studio", "Ollama"]),
        hardware: mini(),
        ..State::default()
    };
    // On each engine a person may type a model for, too.
    for runs_on in [
        CleanupRunsOn::Local,
        CleanupRunsOn::Openrouter,
        CleanupRunsOn::Custom,
    ] {
        settings.dictation_processing.server.runs_on = runs_on;
        let facts = Facts::of(&settings, &state);
        let sent = Sent {
            articles: 3,
            instructions: true,
            vocabulary: Some((1, 95)),
            recordings: 2,
        };
        for text in [text::told(&facts), text::knew(&facts, &sent)] {
            for private in [
                "http",
                "192.168",
                "1234",
                "secret-model",
                "my-own-model",
                "Studio Mic",
                "Anna",
                "Zyxterm",
                "зикстерм",
                "LM Studio",
                "Ollama",
                " key",
                "/Users",
                "~/",
            ] {
                assert!(!text.contains(private), "{private:?} in:\n{text}");
            }
        }
    }
}

#[test]
fn the_record_names_what_else_went_and_never_repeats_it() {
    let state = State {
        hardware: mini(),
        ..State::default()
    };
    let facts = Facts::of(&sample(), &state);
    let sent = Sent {
        articles: 2,
        instructions: true,
        vocabulary: Some((1, 94)),
        recordings: 3,
    };
    let knew = text::knew(&facts, &sent);
    assert!(!knew.contains("<state>"), "{knew}");
    assert!(knew.starts_with("OS: unknown\n"), "{knew}");
    assert!(
        knew.contains(
            "Not on this Mac: Mouse shortcuts (needs a mouse with extra buttons), Notch \
             indicator (needs a screen with a notch)\n"
        ),
        "{knew}"
    );
    assert!(
        knew.ends_with(
            "Also sent: 2 guide articles; each changeable setting's value now; your cleanup \
             instructions; 1 of 94 Vocabulary entries; 3 excerpts of your recordings.\n"
        ),
        "{knew}"
    );
    // The same lines the model read, one for one.
    let told = text::told(&facts);
    for line in knew.lines().take_while(|line| !line.starts_with("Not on")) {
        assert!(told.contains(line), "{line:?} in:\n{told}");
    }
    let nothing = text::knew(&facts, &Sent::default());
    assert!(
        nothing.ends_with("Also sent: each changeable setting's value now.\n"),
        "{nothing}"
    );
}
