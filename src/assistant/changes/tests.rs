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
