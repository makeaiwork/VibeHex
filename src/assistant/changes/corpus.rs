//! The reply corpus (`tests/fixtures/assistant/replies`): replies as models
//! write them, strong and weak, each with the parse the app must make of
//! it. Its README says what a file's header checks.

use std::path::PathBuf;

use serde_json::Value;

use super::parse::parse;
use super::{Why, resolve, vocab};
use crate::app_settings::AppSettings;
use crate::assistant::offer::{Gate, Origin};
use crate::assistant::setup;

fn corpus() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant/replies");
    let mut files: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("the reply corpus")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_name()?.to_str()?.to_owned();
            (name.ends_with(".md") && name != "README.md")
                .then(|| (name, std::fs::read_to_string(&path).unwrap()))
        })
        .collect();
    files.sort();
    files
}

/// The header's fields and the reply under it.
fn split(text: &str) -> (Vec<(String, String)>, String) {
    let rest = text.strip_prefix("---\n").expect("a header");
    let (header, reply) = rest.split_once("\n---\n").expect("a closed header");
    let fields = header
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    (fields, reply.to_owned())
}

fn list(value: &str, separator: char) -> Vec<String> {
    value
        .split(separator)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

fn why(why: &Why) -> &'static str {
    match why {
        Why::Malformed => "malformed",
        Why::Unknown => "unknown",
        Why::Invalid(_) => "invalid",
        Why::Never => "never",
    }
}

#[test]
fn every_reply_in_the_corpus_parses_as_its_header_says() {
    let files = corpus();
    assert!(files.len() >= 15, "only {} replies", files.len());
    let state = crate::assistant::facts::State::default();
    let settings = AppSettings::default();
    for (name, text) in files {
        let (fields, reply) = split(&text);
        let parsed = parse(&reply);
        for fence in ["```hex", "~~~hex", "{\"key\"", "{\"term\"", "CHANGE "] {
            assert!(
                !parsed.body.contains(fence),
                "{name}: the body shows {fence}"
            );
        }
        let (settings_lines, terms): (Vec<_>, Vec<_>) = parsed
            .proposals
            .iter()
            .partition(|proposal| proposal.key != vocab::KEY);
        let read = resolve(
            &parsed.proposals,
            &Gate::new(&settings, &state),
            Origin::Answer,
        );
        // The corpus holds the parse, whatever the OS: a reply with a line
        // for a Labs feature this one lacks holds its cards where that
        // feature is. Every feature it names is in VibeHex on a Mac.
        let elsewhere = read
            .dropped
            .iter()
            .filter(|dropped| matches!(dropped.why, crate::assistant::offer::Why::Elsewhere(_)))
            .count();
        let (cards, mut refused) = (read.cards, read.refused);
        refused.extend(parsed.malformed.iter().map(|line| super::Refused {
            line: line.clone(),
            key: None,
            why: Why::Malformed,
        }));
        for (field, want) in fields {
            match field.as_str() {
                "body" => assert_eq!(
                    parsed.body.lines().next().unwrap_or_default(),
                    want,
                    "{name}: body"
                ),
                "proposals" => {
                    let got: Vec<String> = settings_lines
                        .iter()
                        .map(|proposal| {
                            let value = match &proposal.value {
                                Some(value) => value.clone(),
                                None => Value::Array(proposal.options.clone()),
                            };
                            format!("{}={value}", proposal.key)
                        })
                        .collect();
                    let want: Vec<String> = list(&want, ';')
                        .iter()
                        .map(|line| {
                            let (key, value) = line.split_once('=').unwrap();
                            let value: Value = serde_json::from_str(value).unwrap();
                            format!("{key}={value}")
                        })
                        .collect();
                    assert_eq!(got, want, "{name}: proposals");
                }
                "plan" => assert_eq!(
                    parsed.proposals.iter().filter(|line| line.plan).count(),
                    want.parse::<usize>().unwrap(),
                    "{name}: plan lines"
                ),
                "terms" => {
                    let got: Vec<String> = terms
                        .iter()
                        .filter_map(|proposal| proposal.value.as_ref())
                        .flat_map(vocab::terms)
                        .map(|term| term.term)
                        .collect();
                    assert_eq!(got, list(&want, ','), "{name}: terms");
                }
                "asks" => {
                    let got: Vec<&str> = parsed
                        .asks
                        .iter()
                        .filter_map(|ask| ask.get("id")?.as_str())
                        .collect();
                    assert_eq!(got, list(&want, ','), "{name}: asks");
                }
                "own_asks" => {
                    let asks = setup::asks(&parsed.asks, "macos");
                    let all = setup::questions("macos").len();
                    assert_eq!(asks.len(), all, "{name}: setup always asks all four");
                    assert_eq!(asks[0].own.to_string(), want, "{name}: own_asks");
                }
                "follow_ups" => {
                    let (asks, _) =
                        setup::follow::follow_ups(&parsed.asks, &[], &Gate::new(&settings, &state));
                    let got: Vec<&str> = asks.iter().map(|ask| ask.id.as_str()).collect();
                    assert_eq!(got, list(&want, ','), "{name}: follow_ups");
                }
                "malformed" => assert_eq!(
                    parsed.malformed.len(),
                    want.parse::<usize>().unwrap(),
                    "{name}: malformed"
                ),
                // A line for a feature this OS lacks made no card: the count
                // is the Mac's, where every played line is at home.
                "cards" if elsewhere > 0 => {
                    if cfg!(target_os = "macos") {
                        panic!("{name}: a played line isn't available on a Mac");
                    }
                }
                "cards" => assert_eq!(cards.len(), want.parse::<usize>().unwrap(), "{name}: cards"),
                "refused" => {
                    let got: Vec<String> = refused
                        .iter()
                        .map(|refused| {
                            format!(
                                "{}:{}",
                                refused.key.as_deref().unwrap_or_default(),
                                why(&refused.why)
                            )
                        })
                        .collect();
                    let want: Vec<String> = list(&want, ',')
                        .iter()
                        .map(|item| item.replace(' ', ""))
                        .collect();
                    assert_eq!(got, want, "{name}: refused");
                }
                other => panic!("{name}: no field {other}"),
            }
        }
    }
}
