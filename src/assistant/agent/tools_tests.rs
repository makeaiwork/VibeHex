use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use super::super::protocol;
use super::*;
use crate::app_settings::{AppSettings, AssistantSettings};
use crate::assistant::facts::State;

fn call(line: &str) -> Call {
    let reads = protocol::reads(&format!("```hex-read\n{line}\n```")).expect(line);
    assert!(reads.malformed.is_empty(), "{line}");
    reads.calls.into_iter().next().expect(line)
}

fn sources() -> Sources {
    Sources::at(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant/recordings"),
    )
}

/// Friday 27 November 2026, noon local.
fn now() -> u64 {
    crate::meeting::query::parse_time("2026-11-27", 0, false).unwrap() + 12 * 3_600_000
}

fn allowed() -> Allowed {
    Allowed::from_settings(&AssistantSettings {
        read_recordings: Some(true),
        ..AssistantSettings::default()
    })
    .unwrap()
}

fn open() -> Access {
    Access::Open {
        allowed: allowed(),
        sources: sources(),
        vocabulary: Vec::new(),
    }
}

fn ask() -> Access {
    Access::Ask {
        sources: sources(),
        vocabulary: Vec::new(),
    }
}

/// The person at the other end: what they say when asked, how often they
/// were, and the search steps that reached the pane.
#[derive(Default)]
struct Person {
    allows: bool,
    /// They never answer: the card's wait runs out.
    away: bool,
    asked: Cell<usize>,
    steps: RefCell<Vec<Step>>,
}

impl Live for Person {
    fn consent(&self) -> Consent {
        self.asked.set(self.asked.get() + 1);
        match (self.away, self.allows) {
            (true, _) => Consent::Unanswered,
            (false, true) => Consent::Allowed(allowed()),
            (false, false) => Consent::Declined,
        }
    }

    fn step(&self, step: &Step) {
        self.steps.borrow_mut().push(step.clone());
    }
}

fn context(send: bool) -> changes::Context {
    let mut settings = AppSettings::default();
    settings.assistant.send_settings = send;
    changes::Context {
        settings,
        state: State::default(),
    }
}

/// Settings holding what must never reach a model: paths, an address, an
/// app, a calendar, a microphone, keys, the instructions' text, a term.
fn with_secrets(send: bool) -> changes::Context {
    let mut context = context(send);
    let settings = &mut context.settings;
    settings.meetings.output_folder = Some("/Users/someone/Secret Meetings".into());
    settings.screenshots.folder = Some("/Users/someone/Shots".into());
    settings.files.last_folder = Some("/Users/someone/Interviews".into());
    settings.dictation_processing.server.url = "http://secret-host.example:1234/v1".into();
    settings
        .dictation_processing
        .default_mode
        .post_processing
        .prompt = "Never mention Project Nightjar.".into();
    settings.dictation_processing.default_mode.applications = vec!["com.secret.app".into()];
    settings.files.prompt = "Call the client Bluefinch.".into();
    settings.vocabulary.push(VocabularyEntry {
        term: "Zyxwvutsr".into(),
        sounds_like: vec!["зиксвутср".into()],
    });
    settings.meetings.calendar.excluded_calendars = vec!["CAL-SECRET-ID".into()];
    settings.microphone = Some("Someone's AirPods".into());
    context
}

const SECRETS: &[&str] = &[
    "/Users/someone",
    "secret-host",
    "Nightjar",
    "Bluefinch",
    "com.secret.app",
    "Zyxwvutsr",
    "зиксвутср",
    "CAL-SECRET-ID",
    "AirPods",
    "_api_key |",
];

fn desk(context: &changes::Context, access: Access) -> Desk<'_> {
    Desk::new(Some(context), access, Limits::REMOTE, now())
}

#[test]
fn every_tool_has_one_name_and_runs() {
    let context = context(true);
    let person = Person {
        allows: true,
        ..Person::default()
    };
    for (index, tool) in TOOLS.iter().enumerate() {
        assert!(
            TOOLS[..index].iter().all(|other| other.name != tool.name),
            "{} twice",
            tool.name
        );
        // Its own example is a call the protocol reads, for this tool.
        let example = call(tool.call);
        assert_eq!(example.tool, tool.name);
        let mut desk = desk(&context, open());
        let ran = desk.run(&call(&format!(r#"{{"tool":"{}"}}"#, tool.name)), &person);
        assert!(
            ran.did
                .iter()
                .all(|did| did.refused_for(Why::Unknown).is_none()),
            "{}: {}",
            tool.name,
            ran.text
        );
    }
    assert_eq!(person.asked.get(), 0, "allowed already: never asked");
}

#[test]
fn a_tool_nobody_has_is_said_so_with_the_ones_there_are() {
    let context = context(true);
    let mut desk = desk(&context, open());
    let ran = desk.run(
        &call(r#"{"tool":"settings_set","key":"labs.files","value":true}"#),
        &Person::default(),
    );
    assert!(
        ran.text.starts_with("No tool is called \"settings_set\""),
        "{}",
        ran.text
    );
    assert!(ran.text.contains("guide_read") && ran.text.contains("release_notes"));
    assert_eq!(ran.did, [Did::refused("settings_set", Why::Unknown)]);
    assert_eq!(desk.calls(), 0);
}

/// The never-list, over every key the registry has and the API keys by
/// name: `settings_get` gives a key's line exactly as the catalog writes
/// it, and for a key the catalog never shows, or this VibeHex hasn't got
/// here, no line at all. Nothing of the person's text, paths, addresses,
/// or keys is in any of it.
#[test]
fn settings_get_tells_only_what_the_catalog_tells() {
    let context = with_secrets(true);
    let facts = context.facts();
    let mut keys: Vec<String> = settings::kinds().map(|(key, ..)| key.to_owned()).collect();
    assert!(keys.len() > 80, "{}", keys.len());
    keys.extend(
        [
            parts::CLEANUP,
            parts::PROMPT,
            "dictation_processing.server.openrouter_api_key",
            "dictation_processing.server.custom_api_key",
            "meetings.providers.openai_api_key",
            "assistant.agent",
            "no.such.key",
        ]
        .map(str::to_owned),
    );
    let mut shown = 0;
    for key in &keys {
        let mut desk = desk(&context, Access::Closed);
        let ran = desk.run(
            &call(&format!(r#"{{"tool":"settings_get","keys":["{key}"]}}"#)),
            &Person::default(),
        );
        for secret in SECRETS {
            assert!(!ran.text.contains(secret), "{key} tells {secret}");
        }
        let line = catalog::line(&context.settings, &facts, key);
        let refused = catalog::never(key) || settings::unavailable(key).is_some();
        match line {
            Some(line) => {
                assert!(!refused, "{key} has a line and is never shown");
                assert!(
                    ran.text.contains(&format!("{line}\n")),
                    "{key}: {}",
                    ran.text
                );
                assert_eq!(
                    ran.did,
                    [Did::Settings {
                        keys: vec![key.clone()]
                    }]
                );
                shown += 1;
            }
            None => {
                assert!(
                    ran.text
                        .contains(&format!("{key}: not a setting you can read here.")),
                    "{key}: {}",
                    ran.text
                );
                assert!(!ran.text.contains(&format!("{key} |")), "{key}");
                assert_eq!(ran.did, [Did::Settings { keys: Vec::new() }]);
            }
        }
        assert!(
            !desk.personal(),
            "a setting's line keeps nothing off the web"
        );
    }
    assert!(shown > 40, "{shown}");
    // Keys the catalog never shows were among them.
    assert!(keys.iter().filter(|key| catalog::never(key)).count() > 10);
}

#[test]
fn a_prefix_reads_its_keys_lines_and_no_secret() {
    let context = with_secrets(true);
    let mut text = String::new();
    for prefix in [
        "dictation_processing.",
        "meetings.",
        "files.",
        "assistant.",
        "m",
    ] {
        let mut desk = desk(&context, Access::Closed);
        let ran = desk.run(
            &call(&format!(r#"{{"tool":"settings_get","prefix":"{prefix}"}}"#)),
            &Person::default(),
        );
        text.push_str(&ran.text);
        if prefix == "assistant." {
            assert_eq!(
                ran.did,
                [Did::Settings { keys: Vec::new() }],
                "{}",
                ran.text
            );
        }
    }
    assert!(text.contains("meetings.summaries | "), "{text}");
    assert!(text.contains(&format!("{} | ", parts::PROMPT)), "{text}");
    assert!(!text.contains("microphone |") && !text.contains("assistant."));
    for secret in SECRETS {
        assert!(!text.contains(secret), "a prefix tells {secret}");
    }
}

#[test]
fn with_the_settings_not_sent_no_value_instruction_or_term_is_read() {
    let context = with_secrets(false);
    let mut desk = desk(&context, Access::Closed);
    let person = Person::default();
    let settings = desk.run(
        &call(r#"{"tool":"settings_get","keys":["labs.filler_words","sound_effects"]}"#),
        &person,
    );
    assert!(
        settings
            .text
            .starts_with("key | values | where in the window\n"),
        "{}",
        settings.text
    );
    assert!(settings.text.contains(facts::text::NOT_SENT));
    assert!(!settings.text.contains(" | on\n") && !settings.text.contains(" | off\n"));

    let state = desk.run(&call(r#"{"tool":"facts"}"#), &person);
    assert!(state.text.contains(facts::text::NOT_SENT) && !state.text.contains("<state>"));

    for tool in ["cleanup_instructions", "vocabulary_list"] {
        let ran = desk.run(&call(&format!(r#"{{"tool":"{tool}"}}"#)), &person);
        assert_eq!(ran.text, facts::text::NOT_SENT, "{tool}");
        assert_eq!(ran.did, [Did::refused(tool, Why::NotSent)]);
    }
    let lookup = desk.run(
        &call(r#"{"tool":"vocabulary_lookup","words":["Zyxwvutsr"]}"#),
        &person,
    );
    assert_eq!(lookup.text, facts::text::NOT_SENT);
    assert!(!desk.personal());
    assert_eq!(desk.read().vocabulary, None);
}

#[test]
fn instructions_and_named_terms_are_read_when_sent_and_are_personal() {
    let context = with_secrets(true);
    let person = Person::default();

    let mut one = desk(&context, Access::Closed);
    let ran = one.run(&call(r#"{"tool":"cleanup_instructions"}"#), &person);
    assert!(
        ran.text.contains("Nightjar") && ran.text.contains("<cleanup_instructions>"),
        "{}",
        ran.text
    );
    assert_eq!(ran.did, [Did::Instructions]);
    assert!(one.personal());

    // A lookup that names no entry holds nothing of theirs.
    let mut two = desk(&context, Access::Closed);
    let none = two.run(
        &call(r#"{"tool":"vocabulary_lookup","words":["Kubernetes"]}"#),
        &person,
    );
    assert_eq!(none.did, [Did::Vocabulary { named: 0, total: 1 }]);
    assert!(!two.personal());
    let named = two.run(
        &call(r#"{"tool":"vocabulary_lookup","words":["zyxwvutsr"]}"#),
        &person,
    );
    assert!(named.text.contains("зиксвутср"), "{}", named.text);
    assert_eq!(named.did, [Did::Vocabulary { named: 1, total: 1 }]);
    assert!(two.personal());

    // The list is terms only.
    let mut three = desk(&context, Access::Closed);
    let list = three.run(&call(r#"{"tool":"vocabulary_list"}"#), &person);
    assert!(list.text.contains("Zyxwvutsr") && !list.text.contains("зиксвутср"));
    assert!(three.personal());
}

#[test]
fn the_guide_is_searched_and_read_with_its_settings_lines() {
    // A feature every OS has: its setting has a line everywhere.
    let context = context(true);
    let person = Person::default();
    let mut desk = desk(&context, Access::Closed);
    desk.laid_out(&["topics/meetings"]);

    let found = desk.run(
        &call(r#"{"tool":"guide_search","query":"filler words","limit":50}"#),
        &person,
    );
    assert!(found.text.contains("labs/filler-words: "), "{}", found.text);
    assert!(found.text.lines().count() <= 6);
    assert!(matches!(
        found.did[..],
        [Did::GuideSearch { found: 1..=6, .. }]
    ));

    let read = desk.run(
        &call(r#"{"tool":"guide_read","ids":["labs/filler-words","topics/meetings","no/such"]}"#),
        &person,
    );
    assert!(
        read.text
            .contains("<guide_article id=\"labs/filler-words\">"),
        "{}",
        read.text
    );
    assert!(read.text.contains("topics/meetings: already above."));
    assert!(read.text.contains("\"no/such\": no article has this id"));
    assert!(
        read.text.contains("labs.filler_words | on/off | "),
        "its settings' lines come with it: {}",
        read.text
    );
    assert_eq!(
        read.did,
        [Did::Articles {
            ids: vec!["labs/filler-words".into()],
            app: false
        }]
    );
    assert_eq!(desk.articles(), ["topics/meetings", "labs/filler-words"]);
    assert!(!desk.personal());

    let nothing = desk.run(&call(r#"{"tool":"guide_read","ids":["no/such"]}"#), &person);
    assert_eq!(nothing.did, [Did::refused("guide_read", Why::Failed)]);

    let notes = desk.run(&call(r#"{"tool":"release_notes"}"#), &person);
    assert!(notes.text.contains("<releases running="), "{}", notes.text);
    assert!(matches!(notes.did[..], [Did::Releases { .. }]));
    assert!(!desk.personal() && desk.read().releases);
}

/// One numbering for a turn: a search's excerpts, then a transcript, the
/// notes, and a file number on from one another; what was read before
/// keeps its number and isn't written again.
#[test]
fn excerpts_are_numbered_on_across_calls() {
    let context = context(true);
    let person = Person::default();
    let mut desk = desk(&context, open());

    let search = desk.run(
        &call(r#"{"tool":"recordings_search","query":"pricing","kinds":["meetings"]}"#),
        &person,
    );
    let first = desk.shelf().found().excerpts.len();
    assert!(first >= 1, "{}", search.text);
    assert!(search.text.contains("\n[1] Meeting \""), "{}", search.text);
    assert!(matches!(search.did[..], [Did::Recordings { .. }]));
    assert!(!person.steps.borrow().is_empty(), "its steps land live");
    assert!(desk.personal());
    let meeting = desk.shelf().excerpt(1).unwrap().id.clone();

    // By an excerpt's number: the meeting it is of.
    let transcript = desk.run(&call(r#"{"tool":"meeting_transcript","n":1}"#), &person);
    let next = first as u32 + 1;
    assert!(
        transcript.text.contains(&format!("\n[{next}] Meeting \"")),
        "{}",
        transcript.text
    );
    assert!(transcript.text.contains(&format!("[{next}.1] ")));
    assert_eq!(desk.shelf().excerpt(next).unwrap().id, meeting);
    assert!(matches!(transcript.did[..], [Did::Transcript { .. }]));

    let notes = desk.run(
        &call(&format!(r#"{{"tool":"meeting_notes","id":"{meeting}"}}"#)),
        &person,
    );
    // The search may have read the notes already; either way they have
    // one number.
    let numbered = desk.shelf().found().excerpts.len() as u32;
    assert!(
        notes.text.contains(&format!("[{numbered}] ")) || notes.text.contains("Already above: ["),
        "{}",
        notes.text
    );

    // The same transcript again: nothing new, its number named.
    let again = desk.run(&call(r#"{"tool":"meeting_transcript","n":1}"#), &person);
    assert_eq!(again.text, format!("Already above: [{next}].\n"));
    assert_eq!(desk.shelf().found().excerpts.len() as u32, numbered);

    let files = desk.run(
        &call(r#"{"tool":"file_read","id":"1795168800000-budget"}"#),
        &person,
    );
    assert!(
        files
            .text
            .contains(&format!("\n[{}] File \"budget-draft.m4a\"", numbered + 1)),
        "{}",
        files.text
    );
    // Every number once, in order.
    let numbers: Vec<u32> = desk
        .shelf()
        .found()
        .excerpts
        .iter()
        .map(|excerpt| excerpt.n)
        .collect();
    assert_eq!(numbers, (1..=numbered + 1).collect::<Vec<_>>());

    let missing = desk.run(&call(r#"{"tool":"dictation_read","n":99}"#), &person);
    assert_eq!(missing.did, [Did::refused("dictation_read", Why::Failed)]);
    let dictation = desk.run(&call(r#"{"tool":"dictation_read","id":1}"#), &person);
    assert!(
        dictation
            .text
            .contains(&format!("[{}] Dictation in Slack", numbered + 2)),
        "{}",
        dictation.text
    );
}

#[test]
fn lists_name_ids_and_count_as_the_persons_own() {
    let context = context(true);
    let person = Person::default();
    let mut desk = desk(&context, open());
    let none = desk.run(
        &call(r#"{"tool":"meetings_list","query":"zebra"}"#),
        &person,
    );
    assert!(none.text.starts_with("No meetings match"));
    assert!(!desk.personal(), "a list that names nothing tells nothing");

    let meetings = desk.run(&call(r#"{"tool":"meetings_list","limit":3}"#), &person);
    assert!(meetings.text.contains("1795680000000-standup | Standup"));
    assert_eq!(
        meetings.did,
        [Did::Recordings {
            steps: vec![Step::Meetings {
                searched: 10,
                found: 3
            }]
        }]
    );
    assert!(desk.personal());

    let files = desk.run(&call(r#"{"tool":"files_list"}"#), &person);
    assert_eq!(
        files.did,
        [Did::Recordings {
            steps: vec![Step::Files {
                searched: 3,
                found: 3
            }]
        }]
    );

    let usage = desk.run(&call(r#"{"tool":"usage_insights"}"#), &person);
    assert!(usage.text.starts_with("<usage>\n"), "{}", usage.text);
    assert_eq!(usage.did, [Did::Usage]);
    assert!(desk.read().usage);
}

/// Without the person's leave nothing of the recordings is read, by any
/// tool; and once they said no they aren't asked again.
#[test]
fn without_leave_no_recordings_tool_reads() {
    let context = context(true);
    let calls = [
        r#"{"tool":"recordings_search","query":"pricing"}"#,
        r#"{"tool":"meetings_list"}"#,
        r#"{"tool":"meeting_notes","id":"1795680000000-standup"}"#,
        r#"{"tool":"meeting_transcript","id":"1795680000000-standup"}"#,
        r#"{"tool":"files_list"}"#,
        r#"{"tool":"file_read","id":"1795168800000-budget"}"#,
        r#"{"tool":"dictation_read","id":"1"}"#,
        r#"{"tool":"usage_insights"}"#,
    ];
    assert_eq!(
        calls.len(),
        TOOLS.iter().filter(|tool| tool.recordings).count(),
        "every recordings tool is tried"
    );

    // Closed: refused outright, nobody asked.
    let person = Person::default();
    let mut closed = Desk::new(Some(&context), Access::Closed, Limits::REMOTE, now());
    for line in calls {
        let ran = closed.run(&call(line), &person);
        assert_eq!(ran.text, CLOSED, "{line}");
        assert_eq!(ran.did.len(), 1);
        assert!(ran.did[0].refused_for(Why::NotAllowed).is_some(), "{line}");
    }
    assert_eq!(person.asked.get(), 0);
    assert!(closed.shelf().is_empty() && !closed.personal() && closed.read().closed);

    // To be asked, and they say no: asked once, nothing read, then closed.
    let person = Person::default();
    let mut asking = Desk::new(Some(&context), ask(), Limits::REMOTE, now());
    for (index, line) in calls.iter().enumerate() {
        let ran = asking.run(&call(line), &person);
        assert_eq!(ran.text, CLOSED, "{line}");
        let consent = ran
            .did
            .iter()
            .filter(|did| **did == Did::Consent { allowed: false })
            .count();
        assert_eq!(consent, usize::from(index == 0), "{line}");
    }
    assert_eq!(person.asked.get(), 1, "the card comes once");
    assert!(asking.shelf().is_empty() && !asking.personal());
    assert!(person.steps.borrow().is_empty());

    // They say yes: asked once, and it reads from then on.
    let person = Person {
        allows: true,
        ..Person::default()
    };
    let mut allowed = Desk::new(Some(&context), ask(), Limits::REMOTE, now());
    let first = allowed.run(&call(calls[0]), &person);
    assert_eq!(first.did[0], Did::Consent { allowed: true });
    assert!(matches!(first.did[1], Did::Recordings { .. }));
    let second = allowed.run(&call(calls[2]), &person);
    assert!(matches!(second.did[..], [Did::Notes { .. }]), "{second:?}");
    assert_eq!(person.asked.get(), 1);
    assert!(allowed.personal());

    // Nobody answers: asked once, nothing read, nothing decided. The rest
    // of the turn reads none and asks no more; a new turn asks again.
    let person = Person {
        away: true,
        ..Person::default()
    };
    let mut away = Desk::new(Some(&context), ask(), Limits::REMOTE, now());
    for line in calls {
        let ran = away.run(&call(line), &person);
        assert_eq!(ran.text, UNANSWERED, "{line}");
        assert!(
            matches!(&ran.did[..], [refused] if refused.refused_for(Why::Unanswered).is_some())
        );
    }
    assert_eq!(person.asked.get(), 1, "the card comes once a turn");
    assert!(away.shelf().is_empty() && !away.personal());
    assert!(away.read().unanswered && !away.read().closed);
    assert!(person.steps.borrow().is_empty());
    let mut next = Desk::new(Some(&context), ask(), Limits::REMOTE, now());
    next.run(&call(calls[0]), &person);
    assert_eq!(person.asked.get(), 2, "a later turn asks again");
}

#[test]
fn a_turn_reads_within_its_budget() {
    let context = context(true);
    let person = Person::default();
    let small = Limits {
        result_chars: 500,
        turn_chars: 1_200,
        turn_calls: 50,
        ..Limits::REMOTE
    };
    let mut desk = Desk::new(Some(&context), Access::Closed, small, now());
    let mut read = 0;
    let mut calls = 0;
    for article in guide::articles() {
        if desk.spent() {
            break;
        }
        let ran = desk.run(
            &call(&format!(
                r#"{{"tool":"guide_read","ids":["{}"]}}"#,
                article.id
            )),
            &person,
        );
        calls += 1;
        // One result keeps to its own cap, and to what is left.
        assert!(ran.text.chars().count() <= 540, "{}", ran.text);
        assert!(ran.text.ends_with("… (cut here: the rest didn't fit)\n"));
        read += ran.text.chars().count();
    }
    assert!(desk.spent(), "it fills");
    assert!((3..=20).contains(&calls), "{calls}");
    assert!(read < 1_200 + 540, "{read}");
    let after = desk.run(&call(r#"{"tool":"facts"}"#), &person);
    assert_eq!(after.text, SPENT);
    assert_eq!(after.did, [Did::refused("facts", Why::Budget)]);
    assert_eq!(desk.calls(), calls);

    // And by calls alone.
    let few = Limits {
        turn_calls: 2,
        ..Limits::REMOTE
    };
    let mut desk = Desk::new(Some(&context), Access::Closed, few, now());
    for _ in 0..2 {
        let ran = desk.run(&call(r#"{"tool":"facts"}"#), &person);
        assert!(ran.text.contains("<state>"));
    }
    assert!(desk.spent());
    assert_eq!(desk.run(&call(r#"{"tool":"facts"}"#), &person).text, SPENT);
}

#[test]
fn a_call_that_names_nothing_readable_says_what_it_needs() {
    let context = context(true);
    let person = Person::default();
    let mut desk = desk(&context, open());
    for (line, needs) in [
        (r#"{"tool":"guide_search"}"#, "needs \"query\""),
        (r#"{"tool":"guide_read"}"#, "needs \"ids\""),
        (r#"{"tool":"settings_get"}"#, "needs \"keys\""),
        (r#"{"tool":"vocabulary_lookup"}"#, "needs \"words\""),
        (r#"{"tool":"meeting_notes"}"#, "needs \"id\""),
        (
            r#"{"tool":"meeting_notes","id":"nope"}"#,
            "meetings_list gives the ids",
        ),
        (
            r#"{"tool":"recordings_search","kinds":["emails"]}"#,
            "knows the kinds",
        ),
        (
            r#"{"tool":"recordings_search","since":"someday"}"#,
            "someday",
        ),
        (
            r#"{"tool":"meeting_transcript","id":"1795680000000-standup","from":"soon"}"#,
            "soon",
        ),
    ] {
        let ran = desk.run(&call(line), &person);
        assert!(ran.text.contains(needs), "{line}: {}", ran.text);
        assert!(
            ran.did.last().unwrap().refused_for(Why::Failed).is_some(),
            "{line}"
        );
    }
    assert!(desk.shelf().is_empty());
}
