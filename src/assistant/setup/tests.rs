use serde_json::{Value, json};

use super::plan::{Area, area};
use super::*;
use crate::assistant::changes::catalog::State;
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::{self, Card, current, parts};
use crate::control::settings;

fn answer(id: &str, chosen: &[&str]) -> Answer {
    Answer {
        id: id.into(),
        chosen: chosen.iter().map(|id| (*id).into()).collect(),
        changes: Vec::new(),
    }
}

/// Round one as the owner answered it: code and calls (and files), in
/// Russian, cleanup kept on this Mac, notes on Codex.
fn owners_answers() -> Vec<Answer> {
    vec![
        answer(
            "use",
            &["dictation_any", "dictation_code", "meetings", "files"],
        ),
        answer("language", &["ru_tech"]),
        answer("cleanup", &["local"]),
        answer("notes", &["codex"]),
    ]
}

fn owners() -> AppSettings {
    let mut settings = AppSettings::default();
    sample(&mut settings);
    settings
}

fn mac() -> State {
    State {
        os: "macos",
        codex_signed_in: Some(true),
        opencode_running: Some(true),
        openrouter_key: Some(true),
        ..State::default()
    }
}

/// Whether this OS and build take `key`: the tests hold on every OS.
fn here(key: &str) -> bool {
    settings::unavailable(key).is_none()
}

#[test]
fn the_apps_questions_have_their_fixed_ids() {
    let asks = questions("macos");
    let ids: Vec<&str> = asks.iter().map(|ask| ask.id.as_str()).collect();
    assert_eq!(ids, ["use", "language", "cleanup", "notes"]);
    assert!(asks[0].multi && asks.iter().skip(1).all(|ask| !ask.multi));
    let cleanup: Vec<&str> = asks[2].options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(
        cleanup,
        ["local", "codex", "opencode", "openrouter", "custom", "none"]
    );
    assert!(asks[2].options.iter().all(|option| !option.note.is_empty()));
    assert!(asks[3].options.iter().all(|option| !option.note.is_empty()));
    assert_eq!(asks[2].options[0].label, "On this Mac");
    let windows = questions("windows");
    assert_eq!(windows[2].options[0].label, "On this computer");
    assert!(windows[3].options.iter().all(|option| option.id != "apple"));
}

/// The owner's settings preselect every chip as they are: code (vibe
/// coding is on), calls, files; Russian with English tech terms, with the
/// hint why; cleanup on this Mac; notes on Codex; LM Studio named.
#[test]
fn round_one_is_preselected_from_the_owners_settings() {
    let mut asks = questions("macos");
    preselect(&mut asks, &owners(), &mac());
    let chosen: Vec<Vec<String>> = asks.iter().map(|ask| ask.chosen.clone()).collect();
    assert_eq!(
        chosen[0],
        ["dictation_any", "dictation_code", "meetings", "files"]
    );
    assert_eq!(chosen[1], ["ru_tech"]);
    assert_eq!(asks[1].hint, TECH_HINT);
    assert_eq!(chosen[2], ["local"]);
    assert_eq!(chosen[3], ["codex"]);
    let server = asks[2].options.iter().find(|o| o.id == "custom").unwrap();
    assert_eq!(server.extra, "LM Studio");
    // A default install: dictation only, English, OpenCode as before.
    let mut fresh = questions("macos");
    preselect(&mut fresh, &AppSettings::default(), &mac());
    assert_eq!(fresh[0].chosen, ["dictation_any"]);
    assert_eq!(fresh[1].chosen, ["en"]);
    assert_eq!(fresh[2].chosen, ["none"]);
    // A touched question keeps the person's pick.
    asks[1].touched = true;
    asks[1].chosen = vec!["ru".into()];
    preselect(&mut asks, &owners(), &mac());
    assert_eq!(asks[1].chosen, ["ru"]);
}

#[test]
fn your_own_server_is_named_by_its_port_or_its_app() {
    assert_eq!(
        engines::own_server("http://127.0.0.1:1234/v1", &[]),
        Some("LM Studio")
    );
    assert_eq!(
        engines::own_server("http://localhost:11434/v1", &["LM Studio"]),
        Some("Ollama")
    );
    assert_eq!(engines::own_server("", &["Ollama"]), Some("Ollama"));
    assert_eq!(engines::own_server("https://example.com/v1", &[]), None);
    let settings = owners();
    assert_eq!(engines::dictation_now(&settings), "local");
    assert_eq!(engines::notes_now(&settings, "macos"), "codex");
    let mut off = settings.clone();
    off.dictation_processing
        .default_mode
        .post_processing
        .enabled = false;
    assert_eq!(engines::dictation_now(&off), "none");
    off.dictation_processing
        .default_mode
        .post_processing
        .enabled = true;
    off.labs.ai_cleanup_server = false;
    assert_eq!(engines::dictation_now(&off), "opencode");
}

/// The owner's case, as they answered: nothing moves dictation off this
/// Mac, no GigaAM, and what fits is "Already set", Russian vibe coding
/// and Orukeet among it.
#[test]
fn the_owners_plan_keeps_what_fits() {
    let settings = owners();
    let card = plan::plan(Vec::new(), &owners_answers(), &settings, &mac()).unwrap();
    for item in &card.items {
        assert_ne!(item.to, json!("gigaam_v3"), "{item:?}");
        assert_ne!(item.key, "dictation_processing.server.runs_on", "{item:?}");
        assert_ne!(item.key, parts::CLEANUP, "{item:?}");
        assert_ne!(item.key, "transcription.model", "{item:?}");
    }
    let kept: Vec<&str> = card.kept.iter().map(|item| item.key.as_str()).collect();
    for key in [
        "transcription.model",
        "dictation_processing.server.runs_on",
        "labs.russian_vibe_coding",
        "meetings.enabled",
        "meetings.notes.writer",
    ] {
        assert!(
            !here(key) || kept.contains(&key),
            "{key} not kept: {kept:?}"
        );
    }
    let model = card
        .kept
        .iter()
        .find(|item| item.key == "transcription.model")
        .unwrap();
    assert_eq!(model.from, json!("orukeet"));
}

/// Code in Russian keeps Orukeet or Parakeet v3 and never offers GigaAM;
/// plain Russian may offer it, unchecked over a model the person chose.
#[test]
fn a_speech_model_that_fits_the_language_stays() {
    let mut settings = AppSettings::default();
    let rules = |settings: &AppSettings, language: &str, code: bool| {
        let uses: &[&str] = if code {
            &["dictation_code"]
        } else {
            &["dictation_any"]
        };
        let answers = vec![answer("use", uses), answer("language", &[language])];
        plan::plan(Vec::new(), &answers, settings, &mac())
    };
    let model = |card: Option<Card>| {
        card.and_then(|card| {
            card.items
                .into_iter()
                .find(|item| item.key == "transcription.model")
        })
    };
    // English-only default, Russian code: Parakeet v3, checked.
    let suggested = model(rules(&settings, "ru", true)).unwrap();
    assert_eq!(suggested.to, json!("parakeet_v3"));
    assert!(suggested.included && !suggested.yours);
    // Plain Russian on the default: GigaAM v3, checked.
    let suggested = model(rules(&settings, "ru", false)).unwrap();
    assert_eq!(suggested.to, json!("gigaam_v3"));
    assert!(suggested.included);
    // Plain Russian over Parakeet v3 the person chose: unchecked, "You
    // chose".
    settings.transcription.model = models::TranscriptionModelId::ParakeetV3;
    let suggested = model(rules(&settings, "ru", false)).unwrap();
    assert!(suggested.yours && !suggested.included);
    // Russian tech terms keep it; and Orukeet stays for plain Russian.
    assert!(model(rules(&settings, "ru_tech", false)).is_none());
    settings.transcription.model = models::TranscriptionModelId::Orukeet;
    assert!(model(rules(&settings, "ru", false)).is_none());
    assert!(model(rules(&settings, "ru", true)).is_none());
    // GigaAM with code in Russian: Parakeet v3 offered, unchecked.
    settings.transcription.model = models::TranscriptionModelId::GigaAmV3;
    let suggested = model(rules(&settings, "ru", true)).unwrap();
    assert_eq!(suggested.to, json!("parakeet_v3"));
    assert!(!suggested.included);
}

/// An engine picked that isn't ready starts unchecked with where to
/// finish; Codex signed out has its own sign-in row and starts unchecked.
#[test]
fn an_engine_not_ready_starts_unchecked() {
    let settings = owners();
    let answers = |cleanup: &str| {
        vec![
            answer("use", &["dictation_any"]),
            answer("language", &["en"]),
            answer("cleanup", &[cleanup]),
        ]
    };
    let runs_on = |card: Card| {
        card.items
            .into_iter()
            .find(|item| item.key == "dictation_processing.server.runs_on")
            .unwrap()
    };
    let no_key = State {
        openrouter_key: Some(false),
        ..mac()
    };
    let line = runs_on(plan::plan(Vec::new(), &answers("openrouter"), &settings, &no_key).unwrap());
    assert!(!line.included);
    assert!(line.note.contains("OpenRouter key"), "{}", line.note);
    let mut no_url = settings.clone();
    no_url.dictation_processing.server.url.clear();
    let line = runs_on(plan::plan(Vec::new(), &answers("custom"), &no_url, &mac()).unwrap());
    assert!(!line.included && line.note.contains("address"));
    let signed_out = State {
        codex_signed_in: Some(false),
        ..mac()
    };
    let line = runs_on(plan::plan(Vec::new(), &answers("codex"), &settings, &signed_out).unwrap());
    assert!(!line.included);
    // Ready: the person's answer, checked.
    let line = runs_on(plan::plan(Vec::new(), &answers("codex"), &settings, &mac()).unwrap());
    assert!(line.included && line.note.is_empty() && !line.yours);
}

/// A model's line over a value the person chose starts unchecked with
/// "You chose"; a key round one decides stays the rules', though the
/// model's reason for the same value is taken.
#[test]
fn a_models_lines_join_under_the_rules() {
    let mut settings = owners();
    settings.shortcut_mode = crate::dictation::ShortcutMode::Toggle;
    let shortcuts = crate::rune_ui::shortcut_rules();
    let line = |key: &str, value: Value, why: &str| Proposal {
        key: key.into(),
        value: Some(value),
        why: why.into(),
        plan: true,
        line: key.into(),
        ..Proposal::default()
    };
    let written = vec![
        line("shortcut_mode", json!("hold"), "держать удобнее"),
        line(
            "transcription.model",
            json!("gigaam_v3"),
            "лучше для русского",
        ),
        line(
            "dictation_processing.server.runs_on",
            json!("codex"),
            "качество",
        ),
        line("labs.filler_words", json!(true), "без эээ"),
    ];
    let (cards, _) = changes::resolve(&written, &settings, &shortcuts);
    let card = plan::plan(cards, &owners_answers(), &settings, &mac()).unwrap();
    let keys: Vec<&str> = card.items.iter().map(|item| item.key.as_str()).collect();
    assert!(!keys.contains(&"transcription.model"), "{keys:?}");
    assert!(!keys.contains(&"dictation_processing.server.runs_on"));
    let mode = card
        .items
        .iter()
        .find(|item| item.key == "shortcut_mode")
        .unwrap();
    assert!(mode.yours && !mode.included);
    if here("labs.filler_words") {
        let filler = card
            .items
            .iter()
            .find(|item| item.key == "labs.filler_words")
            .unwrap();
        assert!(filler.included && !filler.yours);
    }
    // Grouped: every area once, in order.
    let areas: Vec<Area> = card.items.iter().map(|item| area(&item.key)).collect();
    assert!(areas.windows(2).all(|pair| pair[0] <= pair[1]), "{areas:?}");
}

/// A follow-up's "Yes" starts checked; a value it already has is kept.
#[test]
fn a_yes_starts_checked() {
    let settings = owners();
    let mut answers = owners_answers();
    answers.push(Answer {
        id: "labs.spoken_symbols".into(),
        chosen: vec!["yes".into()],
        changes: vec![("labs.spoken_symbols".into(), Value::Bool(true))],
    });
    answers.push(Answer {
        id: "labs.vocabulary_quick_add".into(),
        chosen: vec!["yes".into()],
        changes: vec![("labs.vocabulary_quick_add".into(), Value::Bool(true))],
    });
    let card = plan::plan(Vec::new(), &answers, &settings, &mac()).unwrap();
    if here("labs.spoken_symbols") {
        let yes = card
            .items
            .iter()
            .find(|item| item.key == "labs.spoken_symbols")
            .unwrap();
        assert!(yes.included);
    }
    assert!(
        card.items
            .iter()
            .all(|item| item.key != "labs.vocabulary_quick_add")
    );
}

/// The follow-ups for the owner's answers: Labs features that fit, none
/// already on, none round one decides, best first, at most twelve.
#[test]
fn follow_ups_fit_the_answers_and_skip_what_is_on() {
    let settings = owners();
    let asks = follow::candidates(&owners_answers(), &settings, &mac());
    assert!(asks.len() <= follow::ROUND * follow::ROUNDS);
    for ask in &asks {
        assert_eq!(ask.options.len(), 2, "{}", ask.id);
        assert!(ask.own && ask.chosen.is_empty());
        assert_ne!(
            current(&settings, &ask.id),
            Some(Value::Bool(true)),
            "{} is on",
            ask.id
        );
        for key in [
            "labs.russian_vibe_coding",
            "meetings.enabled",
            "labs.files",
            "labs.ai_cleanup_server",
        ] {
            assert_ne!(ask.id, key);
        }
        assert!(here(&ask.id) || ask.id.starts_with("vocabulary."));
    }
    // Dictation alone in English: nothing about calls or Russian.
    let english = vec![
        answer("use", &["dictation_any"]),
        answer("language", &["en"]),
        answer("cleanup", &["none"]),
    ];
    let asks = follow::candidates(&english, &AppSettings::default(), &mac());
    for ask in &asks {
        let article = guide::articles()
            .iter()
            .find(|article| article.keys.first() == Some(&ask.id))
            .unwrap();
        assert!(
            article.uses.iter().any(|tag| tag == "dictation"),
            "{}",
            ask.id
        );
        assert!(
            !article
                .uses
                .iter()
                .any(|tag| tag == "ru" || tag == "cleanup"),
            "{}",
            ask.id
        );
    }
    // Code in Russian without vibe coding: the tech terms first.
    let terms = vec![
        answer("use", &["dictation_code"]),
        answer("language", &["ru"]),
        answer("cleanup", &["none"]),
    ];
    let asks = follow::candidates(&terms, &AppSettings::default(), &mac());
    assert_eq!(asks[0].id, follow::OWN[0].id);
    assert!(
        asks[0].options[0]
            .changes
            .iter()
            .all(|(key, _)| key == changes::vocab::KEY)
    );
}

use crate::assistant::guide;

/// What a model writes for the follow-ups is checked: a candidate keeps
/// its changes in the model's words, a catalog feature gets the app's Yes,
/// any other key needs values a save takes, and an unusable reply is the
/// candidates as they were.
#[test]
fn a_models_follow_ups_are_checked() {
    let settings = AppSettings::default();
    let candidates = vec![{
        let mut ask = Ask {
            id: "labs.filler_words".into(),
            q: "Remove ums and ahs?".into(),
            own: true,
            ..Ask::default()
        };
        ask.options = vec![
            Choice {
                id: "yes".into(),
                label: follow::YES.into(),
                changes: vec![("labs.filler_words".into(), Value::Bool(true))],
                ..Choice::default()
            },
            Choice {
                id: "no".into(),
                label: follow::NO.into(),
                ..Choice::default()
            },
        ];
        ask
    }];
    let written: Vec<Value> = [
        r#"{"id":"labs.filler_words","q":"Убирать «эээ»?","options":[{"id":"yes","label":"Да"},{"id":"no","label":"Нет"}]}"#,
        r#"{"id":"labs.teleport","q":"Телепорт?","options":[{"id":"yes","label":"Да"},{"id":"no","label":"Нет"}]}"#,
        r#"{"id":"history_retention","q":"Хранить историю дольше?","options":[{"id":"long","label":"Всегда","value":"forever"},{"id":"no","label":"Нет"}]}"#,
        r#"{"id":"recording_duck_volume","q":"Громкость?","options":[{"id":"loud","label":"140%","value":1.4},{"id":"no","label":"Нет"}]}"#,
        r#"{"id":"assistant.engine","q":"Модель?","options":[{"id":"x","label":"x","value":"codex"},{"id":"no","label":"Нет"}]}"#,
    ]
    .iter()
    .map(|line| serde_json::from_str(line).unwrap())
    .collect();
    let asks = follow::follow_ups(&written, &candidates, &settings, &[], &Default::default());
    let ids: Vec<&str> = asks.iter().map(|ask| ask.id.as_str()).collect();
    assert_eq!(ids, ["labs.filler_words", "history_retention"]);
    assert_eq!(asks[0].q, "Убирать «эээ»?");
    assert!(!asks[0].own);
    assert_eq!(asks[0].options[0].label, "Да");
    assert_eq!(
        asks[0].options[0].changes,
        [("labs.filler_words".to_owned(), Value::Bool(true))]
    );
    assert_eq!(
        asks[1].options[0].changes,
        [("history_retention".to_owned(), json!("forever"))]
    );
    assert_eq!(
        follow::follow_ups(&[], &candidates, &settings, &[], &Default::default()),
        candidates
    );
    // A setting answered or declined before is never asked again, unless
    // the app picked it itself.
    let settled = [
        "history_retention".to_owned(),
        "labs.filler_words".to_owned(),
    ];
    let asks = follow::follow_ups(
        &written,
        &candidates,
        &settings,
        &settled,
        &Default::default(),
    );
    let ids: Vec<&str> = asks.iter().map(|ask| ask.id.as_str()).collect();
    assert_eq!(ids, ["labs.filler_words"]);
}

/// A follow-up answered in an earlier setup is left out of the app's
/// candidates, out of what a model may add, and named to it as answered.
#[test]
fn a_follow_up_answered_before_is_not_a_candidate() {
    let settings = owners();
    let answers = owners_answers();
    let candidates = follow::candidates(&answers, &settings, &mac());
    let first = candidates[0].clone();
    let state = State {
        answered: vec![crate::assistant::setup::Ask {
            chosen: vec!["no".into()],
            before: true,
            ..first.clone()
        }],
        ..mac()
    };
    let again = follow::candidates(&answers, &settings, &state);
    assert!(again.iter().all(|ask| ask.id != first.id));
    let before = follow::before(&answers, &state);
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].chosen, ["no"]);
    let (_, others, answered) = follow::prompt_lines(&again, &settings, &state);
    assert!(!others.contains(&first.id));
    assert!(answered.contains(&first.id), "{answered}");
}

#[test]
fn a_models_questions_keep_known_ids_or_fall_back() {
    let written: Vec<Value> = [
        r#"{"id":"use","q":"Для чего вам VibeHex?","hint":"можно несколько","multi":false,"options":[{"id":"dictation_any","label":"Диктовка"},{"id":"meetings","label":"Созвоны"},{"id":"teleport","label":"Телепорт"}]}"#,
        r#"{"id":"language","q":"На каком языке?","options":[{"id":"ru","label":"Русский"},{"id":"en","label":"English"}]}"#,
        r#"{"id":"cleanup","q":"Где чистить диктовку?","options":[{"id":"local","label":"На этом Mac","note":"Быстрее всего"},{"id":"codex","label":"Codex"}]}"#,
        r#"{"id":"notes","q":"Где писать конспекты?","options":[{"id":"codex","label":"Codex"},{"id":"apple","label":"Apple Intelligence"}]}"#,
        r#"{"id":"mood","q":"Как дела?","options":[]}"#,
    ]
    .iter()
    .map(|line| serde_json::from_str(line).unwrap())
    .collect();
    let asks = asks(&written, "macos");
    assert_eq!(asks.len(), 4, "an unknown question is dropped");
    assert!(!asks[0].own && asks[0].multi, "multi is the app's to say");
    assert_eq!(asks[0].options.len(), 2, "an unknown option is dropped");
    assert_eq!(asks[2].options[0].note, "Быстрее всего");
    assert!(!asks[2].options[1].note.is_empty(), "the app's caption");
    // One question missing: the app's own instead.
    let fallback = super::asks(&written[..3], "macos");
    assert!(fallback.iter().all(|ask| ask.own));
    assert_eq!(super::asks(&[], "macos"), questions("macos"));
}

/// Every line the rules make, on every answer, is a change a save takes.
#[test]
fn every_rule_resolves_to_a_valid_change() {
    let shortcuts = crate::rune_ui::shortcut_rules();
    for settings in [AppSettings::default(), owners()] {
        for language in ["ru", "en", "ru_tech", "mixed"] {
            for cleanup in ["local", "codex", "opencode", "openrouter", "custom", "none"] {
                for notes in ["codex", "local", "apple", "opencode"] {
                    // Apple Intelligence is offered on a Mac only.
                    if notes == "apple" && !cfg!(target_os = "macos") {
                        continue;
                    }
                    let answers = vec![
                        answer(
                            "use",
                            &["dictation_any", "dictation_code", "meetings", "files"],
                        ),
                        answer("language", &[language]),
                        answer("cleanup", &[cleanup]),
                        answer("notes", &[notes]),
                    ];
                    let rules = plan::rules(&answers, &settings, &mac());
                    let lines = plan::proposals(&rules);
                    let (cards, refused) = changes::resolve(&lines, &settings, &shortcuts);
                    assert!(
                        refused.is_empty(),
                        "{language} {cleanup} {notes}: {refused:?}"
                    );
                    let items: usize = cards.iter().map(|card| card.items.len()).sum();
                    assert_eq!(items, lines.len());
                    let card = plan::plan(Vec::new(), &answers, &settings, &mac());
                    assert!(card.is_none_or(|card| card.items.len() <= plan::PLAN_LINES));
                }
            }
        }
    }
}

#[test]
fn the_answers_read_as_ids_and_the_labels_seen() {
    let mut asks = questions("macos");
    asks[0].chosen = vec!["dictation_code".into(), "meetings".into()];
    asks[1].chosen = vec!["ru".into()];
    asks[2].chosen.clear();
    asks[3].chosen.clear();
    let line = answers_line(&asks, |_, option| option.label.clone());
    assert_eq!(
        line,
        "use: dictation_code (Dictation into code and the terminal), meetings (Recording calls); language: ru (Russian)"
    );
}

/// What is set now, in one paragraph: the owner's model, engines, Labs,
/// and terms.
#[test]
fn now_says_what_is_set() {
    let now = now::now(&owners());
    for words in [
        "Orukeet",
        "Russian",
        "Gemma 4 E4B",
        "Codex",
        "Russian vibe coding",
        "94 terms",
    ] {
        assert!(now.contains(words), "{words}: {now}");
    }
}

/// What an earlier setup offered and the person declined (a follow-up's
/// "No", a row left unticked) isn't offered again while the setting is
/// as it was then; once it changed, it may be.
#[test]
fn a_declined_line_is_not_offered_again() {
    let settings = AppSettings::default();
    let answers = owners_answers();
    let asks = follow::candidates(&answers, &settings, &mac());
    let first = asks
        .iter()
        .find(|ask| ask.id.starts_with("labs."))
        .expect("a Labs follow-up")
        .id
        .clone();
    let card = plan::plan(Vec::new(), &answers, &settings, &mac()).unwrap();
    let row = card
        .items
        .iter()
        .find(|item| item.key != changes::vocab::KEY)
        .unwrap()
        .clone();
    let line = |key: &str, to: Value| past::Line {
        key: key.into(),
        from: current(&settings, key).unwrap_or(Value::Null),
        to,
    };
    let state = State {
        declined: vec![
            line(&first, Value::Bool(true)),
            line(&row.key, row.to.clone()),
        ],
        ..mac()
    };
    let again = follow::candidates(&answers, &settings, &state);
    assert!(
        again.iter().all(|ask| ask.id != first),
        "{first} asked again"
    );
    let card = plan::plan(Vec::new(), &answers, &settings, &state).unwrap();
    assert!(
        card.items.iter().all(|item| item.key != row.key),
        "{} offered again",
        row.key
    );
    // Changed since: offered again.
    let state = State {
        declined: vec![past::Line {
            from: Value::from("something else"),
            ..line(&row.key, row.to.clone())
        }],
        ..mac()
    };
    let card = plan::plan(Vec::new(), &answers, &settings, &state).unwrap();
    assert!(card.items.iter().any(|item| item.key == row.key));
}

/// A returning setup's terms join the plan as a row each: the
/// sure ones (cleanup fixed one heard word into them three times or more)
/// checked, the rest unchecked; none Vocabulary has, and none the person
/// declined in an earlier setup. Applied with the second left unticked,
/// its terms are declined for next time.
#[test]
fn the_uses_terms_join_the_plan_sure_ones_checked() {
    use crate::assistant::changes::vocab;
    use since::Candidate;
    let settings = owners();
    let candidate = |term: &str, heard: &str, sure: bool| Candidate {
        term: term.into(),
        sounds_like: vec![heard.into()],
        sure,
    };
    let candidates = [
        candidate("Granola", "гранола", true),
        candidate("Outkit", "ауткит", false),
        candidate("Whisper", "виспер", false),
        candidate("Term7", "терм", true),
    ];
    let rows = |state: &State| -> Vec<(Vec<String>, bool)> {
        plan::with_terms(Vec::new(), &owners_answers(), &candidates, &settings, state)
            .unwrap()
            .items
            .iter()
            .filter(|item| item.key == vocab::KEY)
            .map(|item| {
                let terms = vocab::terms(&item.to);
                (
                    terms.into_iter().map(|term| term.term).collect(),
                    item.included,
                )
            })
            .collect()
    };
    // `Term7` is in the owner's Vocabulary already, and `Outkit` is a
    // near miss of the speech model Orukeet: offered as Orukeet, heard
    // as "ауткит". Each term is a row of its own.
    assert_eq!(
        rows(&mac()),
        [
            (vec!["Granola".to_owned()], true),
            (vec!["Orukeet".to_owned()], false),
            (vec!["Whisper".to_owned()], false),
        ]
    );
    let card = plan::with_terms(
        Vec::new(),
        &owners_answers(),
        &candidates,
        &settings,
        &mac(),
    )
    .unwrap();
    let orukeet = card
        .items
        .iter()
        .flat_map(|item| vocab::terms(&item.to))
        .find(|term| term.term == "Orukeet")
        .unwrap();
    assert_eq!(orukeet.sounds_like, ["ауткит"]);
    // A near miss of a Vocabulary term is that term misheard: left out.
    let near = [
        candidate("Grafana", "графана", true),
        candidate("Tirm7", "тирм", true),
    ];
    let left = plan::with_terms(Vec::new(), &owners_answers(), &near, &settings, &mac())
        .unwrap()
        .items
        .iter()
        .flat_map(|item| vocab::terms(&item.to))
        .map(|term| term.term)
        .collect::<Vec<_>>();
    assert_eq!(left, ["Grafana"]);
    let card = plan::with_terms(
        Vec::new(),
        &owners_answers(),
        &candidates,
        &settings,
        &mac(),
    )
    .unwrap();
    let mut card = card;
    for item in &mut card.items {
        if item.key == vocab::KEY && !item.included {
            assert_eq!(item.why, "You say these, and Vocabulary doesn't have them");
        }
    }
    card.decision = changes::Decision::Applied {
        before: Default::default(),
        after: Default::default(),
        at_ms: 1,
    };
    let finished = past::finished(&[], &owners_answers(), Some(&card), &settings, 1);
    let state = State {
        declined: past::declined(&[finished]),
        ..mac()
    };
    assert_eq!(rows(&state), [(vec!["Granola".to_owned()], true)]);
}

/// A Mac known to have no notch and only a Magic Mouse, one with a notch
/// and an MX mouse, and one whose hardware couldn't be read.
fn macs() -> [State; 3] {
    use crate::desktop::hardware::{Hardware, Pointer, Screens};
    let with = |notch: bool, name: &str, extra_buttons: bool| State {
        hardware: Hardware {
            screens: Some(Screens {
                built_in: notch,
                external: usize::from(!notch),
                notch,
            }),
            pointers: Some(vec![Pointer {
                name: name.into(),
                extra_buttons,
            }]),
            model: None,
        },
        ..mac()
    };
    [
        with(false, "Magic Mouse", false),
        with(true, "MX Master 3S", true),
        mac(),
    ]
}

/// A Labs feature this Mac can't use is never asked or planned; one it
/// can is asked straight out; unknown hardware asks "if your … has …".
#[test]
fn a_feature_this_mac_cant_use_is_left_out_and_unknown_hardware_asks_if() {
    let [mini, notched, unknown] = macs();
    let answers = vec![
        answer("use", &["dictation_any"]),
        answer("language", &["en"]),
        answer("cleanup", &["none"]),
    ];
    let settings = AppSettings::default();
    for (key, ask, ask_if) in [
        (
            "labs.notch_indicator",
            "Show the recording indicator in your screen's notch?",
            "If your screen has a notch, show the recording indicator in it?",
        ),
        (
            "labs.mouse_shortcuts",
            "Start dictation with a mouse button, such as a side button?",
            "If your mouse has extra buttons, start dictation with one?",
        ),
    ] {
        if !here(key) {
            continue;
        }
        let relevant = |state: &State| follow::relevant(&answers, &state.hardware);
        assert!(!relevant(&mini).contains(&key.to_owned()), "{key}");
        assert!(relevant(&notched).contains(&key.to_owned()), "{key}");
        assert!(relevant(&unknown).contains(&key.to_owned()), "{key}");
        let candidates = follow::candidates(&answers, &settings, &mini);
        assert!(candidates.iter().all(|ask| ask.id != key), "{key}");
        // A model asking about it anyway: dropped where it can't be used.
        let written = [json!({"id": key})];
        let asked = |state: &State| {
            follow::follow_ups(&written, &[], &settings, &[], &state.hardware)
                .into_iter()
                .find(|ask| ask.id == key)
                .map(|ask| ask.q)
        };
        assert_eq!(asked(&mini), None, "{key}");
        assert_eq!(asked(&notched).as_deref(), Some(ask));
        assert_eq!(asked(&unknown).as_deref(), Some(ask_if));
        let (_, others, _) = follow::prompt_lines(&[], &settings, &mini);
        assert!(!others.contains(key), "{key}");
        // A Yes from an earlier round, or a model's plan line, doesn't
        // make the plan where it can't be used.
        let mut yes = answers.clone();
        yes.push(Answer {
            id: key.into(),
            chosen: vec!["yes".into()],
            changes: vec![(key.into(), Value::Bool(true))],
        });
        let model = vec![Card {
            items: vec![changes::Item {
                key: key.into(),
                from: Value::Bool(false),
                to: Value::Bool(true),
                included: true,
                ..changes::Item::default()
            }],
            ..Card::default()
        }];
        let planned = |state: &State, cards: Vec<Card>| {
            plan::plan(cards, &yes, &settings, state)
                .is_some_and(|card| card.items.iter().any(|item| item.key == key))
        };
        assert!(!planned(&mini, Vec::new()), "{key}");
        assert!(!planned(&mini, model.clone()), "{key}");
        assert!(planned(&notched, Vec::new()), "{key}");
        assert!(planned(&unknown, Vec::new()), "{key}");
    }
}
