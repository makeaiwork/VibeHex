use serde_json::json;

use super::parse::{Proposal, parse};
use super::*;

fn rules() -> ShortcutRules {
    ShortcutRules {
        key_named: |name| Ok(crate::app_settings::HotkeyKey::named(name)),
        rule: |_, _| Ok(()),
    }
}

fn proposal(key: &str, value: Value) -> Proposal {
    Proposal {
        key: key.into(),
        value: Some(value),
        why: "because".into(),
        line: key.into(),
        ..Proposal::default()
    }
}

fn settings() -> AppSettings {
    let mut settings = AppSettings::default();
    settings.labs.smart_spacing = false;
    settings.labs.ai_cleanup_server = false;
    settings.meetings.enabled = false;
    settings
}

#[test]
fn a_good_line_is_a_card_with_its_current_value() {
    let (cards, refused) = resolve(
        &[proposal("labs.smart_spacing", json!("on"))],
        &settings(),
        &rules(),
    );
    assert!(refused.is_empty());
    let item = &cards[0].items[0];
    assert_eq!((&item.from, &item.to), (&json!(false), &json!(true)));
    assert_eq!(item.why, "because");
    assert_eq!(item.class, Class::Card);
    assert_eq!(cards[0].kind, CardKind::Single);
    assert_eq!(cards[0].decision, Decision::Proposed);
}

#[test]
fn every_bad_line_is_refused_for_its_own_reason() {
    let (cards, refused) = resolve(
        &[
            proposal("labs.smart_spacing", json!(true)),
            proposal("labs.teleport", json!(true)),
            proposal("recording_duck_volume", json!(1.4)),
            proposal("updates.channel", json!("dev")),
            proposal("assistant.read_recordings", json!(true)),
            proposal("meetings.output_folder", json!("/tmp")),
        ],
        &settings(),
        &rules(),
    );
    assert_eq!(cards.len(), 1);
    let why: Vec<&Why> = refused.iter().map(|refused| &refused.why).collect();
    assert_eq!(why[0], &Why::Unknown);
    assert!(matches!(why[1], Why::Invalid(_)), "{:?}", why[1]);
    assert_eq!(why[2..], [&Why::Never, &Why::Never, &Why::Never]);
    assert_eq!(refused[0].key.as_deref(), Some("labs.teleport"));
}

#[test]
fn a_change_names_what_it_needs_first() {
    let (cards, _) = resolve(
        &[proposal(
            "dictation_processing.server.url",
            json!("http://localhost:1234/v1"),
        )],
        &settings(),
        &rules(),
    );
    assert_eq!(
        cards[0].items[0].requires,
        [("labs.ai_cleanup_server".to_owned(), json!(true))]
    );
    let values = values(&cards[0], &settings()).unwrap();
    let mut keys: Vec<&str> = values.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["dictation_processing.server.url", "labs.ai_cleanup_server"]
    );
    let change = settings::check(&settings(), &values, &rules()).unwrap();
    assert!(change.settings.labs.ai_cleanup_server);
}

#[test]
fn options_become_a_choice_starting_at_the_suggestion() {
    let mut options = proposal("dictation_processing.server.runs_on", json!(null));
    options.value = None;
    options.options = vec![json!("OpenCode"), json!("codex"), json!("teleport")];
    options.suggest = Some(json!("Codex"));
    let (cards, _) = resolve(&[options], &settings(), &rules());
    assert_eq!(
        cards[0].kind,
        CardKind::Choice {
            options: vec![json!("opencode"), json!("codex")],
            chosen: 1,
            suggested: 1
        }
    );
}

#[test]
fn a_part_and_the_instructions_make_cards() {
    let (cards, refused) = resolve(
        &[
            proposal("meetings.providers.zoom.policy", json!("Always record")),
            proposal(parts::PROMPT, json!("Fix punctuation.")),
        ],
        &settings(),
        &rules(),
    );
    assert!(refused.is_empty(), "{refused:?}");
    let zoom = &cards[0].items[0];
    assert_eq!(
        (&zoom.from, &zoom.to),
        (&json!("ask"), &json!("always_record"))
    );
    assert_eq!(
        zoom.requires,
        [("meetings.enabled".to_owned(), json!(true))]
    );
    assert_eq!(cards[1].kind, CardKind::Prompt);
    let values = values(&cards[0], &settings()).unwrap();
    assert_eq!(values["meetings.providers"][0]["policy"], "always_record");
}

#[test]
fn a_whole_reply_becomes_its_body_cards_and_refusals() {
    let parsed = parse(
        "Turn on Smart spacing.\n```hex-change\n{\"key\":\"labs.smart_spacing\",\"value\":true,\"why\":\"spaces\"}\n{\"key\":\"labs.nope\",\"value\":true}\nnonsense\n```\nSources: labs/smart-spacing",
    );
    let (cards, refused) = resolve(&parsed.proposals, &settings(), &rules());
    assert_eq!(cards.len(), 1);
    assert_eq!(refused.len(), 1);
    assert_eq!(parsed.malformed, ["nonsense"]);
}

/// Old conversation files, without cards, still read; new ones keep them.
#[test]
fn cards_round_trip_and_old_turns_still_load() {
    let (cards, refused) = resolve(
        &[
            proposal("labs.smart_spacing", json!(true)),
            proposal("labs.nope", json!(true)),
        ],
        &settings(),
        &rules(),
    );
    let mut card = cards[0].clone();
    card.decision = applied(
        vec![("labs.smart_spacing".into(), json!(false), json!(true))],
        5,
    );
    let json = serde_json::to_string(&(vec![card.clone()], refused.clone())).unwrap();
    let back: (Vec<Card>, Vec<Refused>) = serde_json::from_str(&json).unwrap();
    assert_eq!(back, (vec![card], refused));
}

/// A `hex-vocab` block, or the older `vocabulary_add` shape, makes one
/// terms card; the block never shows, even while it streams.
#[test]
fn vocabulary_terms_make_one_card() {
    use super::vocab;
    let reply = "Добавлю их в словарь.\n\n```hex-vocab\n{\"term\":\"Kubernetes\",\"sounds_like\":[\"кубернетис\",\"кубер\"],\"why\":\"часто путается\"}\n{\"term\":\"kubectl\",\"sounds_like\":\"куб контрол, кубкатл\"}\n{\"term\":\"Kubernetes\"}\n```\n\n```hex-change\n{\"vocabulary_add\":[{\"term\":\"Grafana\",\"sounds_like\":[\"графана\"]}]}\n```\n\nSources: none";
    let parsed = parse(reply);
    assert_eq!(parsed.body, "Добавлю их в словарь.\n\nSources: none");
    assert_eq!(parsed.proposals.len(), 4);
    assert!(parsed.malformed.is_empty());
    let settings = AppSettings::default();
    let (cards, refused) = resolve(&parsed.proposals, &settings, &rules());
    assert!(refused.is_empty());
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0].kind, CardKind::Vocabulary);
    let names: Vec<String> = cards[0]
        .items
        .iter()
        .map(|item| vocab::terms(&item.to)[0].term.clone())
        .collect();
    assert_eq!(names, ["Kubernetes", "kubectl", "Grafana"]);
    let streaming = "Добавлю.\n\n```hex-vo";
    assert_eq!(crate::assistant::prompt::draft(streaming), "Добавлю.");
}

/// A change to the value there is now is no card: the same value, or the
/// same engine written another way (`codex` over `codex:<AI cleanup's
/// model>`). A real change stays, and its from side names the model.
#[test]
fn a_value_already_set_makes_no_card() {
    let mut settings = settings();
    settings.labs.ai_cleanup_server = true;
    settings.meetings.enabled = true;
    settings.meetings.notes.writer = Some("codex:gpt-6.1-terra".into());
    settings.dictation_processing.server.codex_model = "gpt-6.1-terra".into();
    let lines = [
        proposal("meetings.notes.writer", json!("codex")),
        proposal("meetings.notes.writer", json!("codex:gpt-6.1-terra")),
        proposal("labs.smart_spacing", json!("off")),
        proposal("files.cleanup_engine", json!("opencode")),
        proposal("meetings.notes.writer", json!("opencode")),
    ];
    let (mut cards, refused) = resolve(&lines, &settings, &rules());
    assert!(refused.is_empty(), "{refused:?}");
    let already = already_set(&mut cards, &settings);
    assert_eq!(already.len(), 3, "{already:?}");
    assert!(already[0].starts_with("meetings.notes.writer = \"codex\" : already set to"));
    let keys: Vec<&str> = cards
        .iter()
        .map(|card| card.items[0].key.as_str())
        .collect();
    assert_eq!(keys, ["files.cleanup_engine", "meetings.notes.writer"]);
    let writer = &cards[1].items[0];
    assert_eq!(writer.from, json!("codex:gpt-6.1-terra"));
    assert!(
        names::label(&writer.key, &writer.from).starts_with("Codex ("),
        "{}",
        names::label(&writer.key, &writer.from)
    );
    assert!(names::same(
        &settings,
        "meetings.notes.writer",
        &json!(""),
        &Value::Null
    ));
    assert!(!names::same(
        &settings,
        "labs.smart_spacing",
        &json!(true),
        &json!(false)
    ));
}
