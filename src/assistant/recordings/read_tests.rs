use std::path::{Path, PathBuf};

use super::super::Shelf;
use super::*;
use crate::app_settings::AssistantSettings;

fn fixtures() -> Sources {
    Sources::at(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/assistant/recordings"),
    )
}

/// Friday 27 November 2026, noon local.
fn now() -> u64 {
    parse_time("2026-11-27", 0, false).unwrap() + 12 * 3_600_000
}

fn allowed() -> Allowed {
    Allowed::from_settings(&AssistantSettings {
        read_recordings: Some(true),
        ..AssistantSettings::default()
    })
    .unwrap()
}

const STANDUP: &str = "1795680000000-standup";

/// A folder of recordings made for one test, removed when it ends.
struct Made(PathBuf);

impl Made {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("hex-assistant-read-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("meetings")).unwrap();
        Self(root)
    }

    /// A meeting `id` whose manifest is the standup's with `change` made,
    /// and whose transcript is `entries`.
    fn meeting(&self, id: &str, change: impl Fn(&mut serde_json::Value), entries: &[String]) {
        let from = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/assistant/recordings/meetings")
            .join(STANDUP);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(from.join("manifest.json")).unwrap())
                .unwrap();
        manifest["id"] = id.into();
        change(&mut manifest);
        let folder = self.0.join("meetings").join(id);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("manifest.json"), manifest.to_string()).unwrap();
        // Every line ended, as a finished recording's are.
        let transcript: String = entries.iter().map(|entry| format!("{entry}\n")).collect();
        std::fs::write(folder.join("transcript.ndjson"), transcript).unwrap();
    }

    fn sources(&self) -> Sources {
        Sources::at(&self.0)
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn entry(source: &str, start_ms: u64, text: &str) -> String {
    serde_json::json!({
        "source": source,
        "start_ms": start_ms,
        "end_ms": start_ms + 4_000,
        "text": text,
    })
    .to_string()
}

#[test]
fn the_meetings_are_listed_with_their_ids_newest_first() {
    let listed = meetings_list(&allowed(), &fixtures(), (None, None, None), 20, now()).unwrap();
    assert_eq!((listed.searched, listed.found), (10, 10), "{}", listed.text);
    let standup = listed.text.find(STANDUP).unwrap();
    let acme = listed.text.find("1792508400000-acme").unwrap();
    assert!(standup < acme, "newest first:\n{}", listed.text);
    assert!(
        listed.text.contains("| Standup | 2026-11-2"),
        "{}",
        listed.text
    );
    assert!(listed.text.contains("| 00:45 | notes"), "{}", listed.text);
}

#[test]
fn a_list_keeps_to_its_time_its_words_and_its_limit() {
    let november = meetings_list(
        &allowed(),
        &fixtures(),
        (Some("2026-11-20"), None, None),
        20,
        now(),
    )
    .unwrap();
    assert_eq!(november.found, 3, "{}", november.text);
    assert_eq!(november.searched, 10);

    let anna = meetings_list(
        &allowed(),
        &fixtures(),
        (None, None, Some("anna")),
        20,
        now(),
    )
    .unwrap();
    assert_eq!(anna.found, 2, "{}", anna.text);

    let two = meetings_list(&allowed(), &fixtures(), (None, None, None), 2, now()).unwrap();
    assert_eq!(two.found, 2);
    assert!(two.text.starts_with("2 of 10 meetings"), "{}", two.text);

    let none = meetings_list(
        &allowed(),
        &fixtures(),
        (None, None, Some("zebra")),
        20,
        now(),
    )
    .unwrap();
    assert_eq!(none.found, 0);
    assert!(none.text.starts_with("No meetings match"), "{}", none.text);

    assert!(
        meetings_list(
            &allowed(),
            &fixtures(),
            (Some("someday"), None, None),
            20,
            now()
        )
        .is_err(),
        "a time it cannot read is said, not guessed"
    );
}

#[test]
fn notes_are_one_excerpt_cited_whole() {
    let notes = meeting_notes(&allowed(), &fixtures(), STANDUP).unwrap();
    assert!(notes.is_notes() && notes.lines.is_empty());
    assert!(notes.text.contains("Release blockers"), "{}", notes.text);
    assert_eq!(notes.id, STANDUP);

    // The start of an id is enough when one meeting starts so.
    let short = meeting_notes(&allowed(), &fixtures(), "17956800").unwrap();
    assert_eq!(short.id, STANDUP);

    let unknown = meeting_notes(&allowed(), &fixtures(), "nope-nope").unwrap_err();
    assert!(unknown.contains("meetings_list"), "{unknown}");
}

#[test]
fn a_meeting_without_notes_says_to_read_the_transcript() {
    let made = Made::new("no-notes");
    made.meeting(
        "1795700000000-plain",
        |manifest| manifest["notes"] = serde_json::Value::Null,
        &[entry("system", 0, "Hello there.")],
    );
    let error = meeting_notes(&allowed(), &made.sources(), "1795700000000-plain").unwrap_err();
    assert!(error.contains("meeting_transcript"), "{error}");
}

#[test]
fn a_transcript_reads_in_order_with_who_spoke_and_lines_to_cite() {
    let read = meeting_transcript(&allowed(), &fixtures(), STANDUP, (None, None), now()).unwrap();
    let excerpt = &read.excerpt;
    assert_eq!(read.more, None);
    assert_eq!(excerpt.kind, Kind::Meeting);
    assert!(excerpt.lines.len() >= 2, "{excerpt:?}");
    assert!(
        excerpt.lines[0].text.starts_with("Them: Payment flow"),
        "{:?}",
        excerpt.lines[0]
    );
    assert!(
        excerpt.label.starts_with("transcript 00:00 to "),
        "{}",
        excerpt.label
    );
    // The lines stand where the meeting's pane shows them, in order.
    for pair in excerpt.lines.windows(2) {
        assert!(pair[0].at.end <= pair[1].at.start, "{pair:?}");
        assert!(pair[0].start_ms <= pair[1].start_ms, "{pair:?}");
    }

    // A stretch of it: only what was said from then on.
    let later = meeting_transcript(
        &allowed(),
        &fixtures(),
        STANDUP,
        (Some("0:20"), None),
        now(),
    )
    .unwrap();
    assert!(later.excerpt.lines.len() < excerpt.lines.len());
    assert!(
        later.excerpt.lines[0].text.starts_with("You: Релиз"),
        "{:?}",
        later.excerpt.lines[0]
    );
    assert_eq!(later.excerpt.offset_ms, Some(22_900));

    let past = meeting_transcript(
        &allowed(),
        &fixtures(),
        STANDUP,
        (Some("9:00:00"), None),
        now(),
    );
    assert!(past.is_err(), "{past:?}");
}

#[test]
fn a_long_transcript_stops_and_says_where_to_read_on() {
    let made = Made::new("long");
    let said = "word ".repeat(40);
    let entries: Vec<String> = (0..60)
        .map(|index| {
            entry(
                if index % 2 == 0 {
                    "system"
                } else {
                    "microphone"
                },
                index * 10_000,
                &format!("Part {index}. {said}"),
            )
        })
        .collect();
    made.meeting(
        "1795700000000-long",
        |manifest| manifest["duration_ms"] = 600_000.into(),
        &entries,
    );
    let sources = made.sources();
    let mut shelf = Shelf::default();
    let mut from: Option<String> = None;
    let mut parts: Vec<String> = Vec::new();
    let mut reads = 0;
    loop {
        let read = meeting_transcript(
            &allowed(),
            &sources,
            "1795700000000-long",
            (from.as_deref(), None),
            now(),
        )
        .unwrap();
        reads += 1;
        assert!(
            read.excerpt.text.chars().count() <= TRANSCRIPT_CHARS + 400,
            "{}",
            read.excerpt.text.chars().count()
        );
        parts.extend(read.excerpt.lines.iter().map(|line| line.text.clone()));
        let placed = shelf.put(vec![read.excerpt]);
        assert_eq!(placed[0].n as usize, reads, "numbered on from the last");
        assert!(placed[0].new);
        match read.more {
            Some(more) => from = Some(more),
            None => break,
        }
        assert!(reads < 20, "it never ends");
    }
    assert!(reads >= 3, "{reads}");
    // Every part once, in order: nothing skipped at a seam, nothing twice.
    let numbers: Vec<usize> = parts
        .iter()
        .map(|part| {
            let part = part.split_once("Part ").unwrap().1;
            part.split_once('.').unwrap().0.parse().unwrap()
        })
        .collect();
    assert_eq!(numbers, (0..60).collect::<Vec<_>>());
}

#[test]
fn files_are_listed_and_read_by_their_lines() {
    let listed = files_list(&allowed(), &fixtures(), None);
    assert_eq!((listed.searched, listed.found), (3, 3), "{}", listed.text);
    assert!(
        listed
            .text
            .contains("1795168800000-budget | budget-draft.m4a")
    );
    let one = files_list(&allowed(), &fixtures(), Some("lecture"));
    assert_eq!((one.searched, one.found), (3, 1), "{}", one.text);

    let read = file_read(
        &allowed(),
        &fixtures(),
        "1795168800000-budget",
        (None, None),
    )
    .unwrap();
    assert_eq!(read.excerpt.kind, Kind::File);
    assert_eq!(read.excerpt.lines.len(), 3, "{:?}", read.excerpt);
    assert_eq!(read.more, None);
    let record = crate::files::store::list_in(&fixtures().files)
        .into_iter()
        .find(|record| record.id == "1795168800000-budget")
        .unwrap();
    let plain = crate::files::text::plain(&record.versions[0].segments);
    for line in &read.excerpt.lines {
        assert_eq!(plain[line.at.clone()].trim(), line.text, "{line:?}");
    }

    let unknown = file_read(&allowed(), &fixtures(), "nope", (None, None)).unwrap_err();
    assert!(unknown.contains("files_list"), "{unknown}");
}

#[test]
fn a_dictation_reads_whole_by_its_id() {
    let read = dictation_read(&allowed(), &fixtures(), "1").unwrap();
    assert_eq!(read.kind, Kind::Dictation);
    assert_eq!(read.title, "Slack");
    assert!(read.text.starts_with("Release notes: payment flow fixed"));
    assert!(read.lines.is_empty());
    assert!(dictation_read(&allowed(), &fixtures(), "99999").is_err());
    assert!(dictation_read(&allowed(), &fixtures(), "one").is_err());
}

#[test]
fn a_search_asked_for_finds_what_a_typed_question_would() {
    let asked = Asked {
        query: "pricing".into(),
        kinds: Kinds {
            meetings: true,
            ..Kinds::default()
        },
        ..Asked::default()
    };
    let mut steps = Vec::new();
    let found = search(
        &allowed(),
        &fixtures(),
        &asked,
        &[],
        (Budget::LARGE, now()),
        &mut |step| steps.push(step.clone()),
    )
    .unwrap();
    assert!(
        found
            .excerpts
            .iter()
            .any(|excerpt| excerpt.id.ends_with("pricing-anna")),
        "{:?}",
        found.excerpts
    );
    assert!(
        found
            .excerpts
            .iter()
            .all(|excerpt| excerpt.kind == Kind::Meeting)
    );
    assert!(matches!(steps.first(), Some(Step::Meetings { .. })));

    let wrong = search(
        &allowed(),
        &fixtures(),
        &Asked {
            since: Some("2026-11-20".into()),
            until: Some("2026-11-10".into()),
            ..Asked::default()
        },
        &[],
        (Budget::LARGE, now()),
        &mut |_| {},
    );
    assert!(wrong.is_err());
}

#[test]
fn the_recording_a_question_is_about_is_not_read_again() {
    let sources = fixtures().without(Kind::Meeting, STANDUP);
    let listed = meetings_list(&allowed(), &sources, (None, None, None), 20, now()).unwrap();
    assert!(!listed.text.contains(STANDUP), "{}", listed.text);
    assert!(meeting_notes(&allowed(), &sources, STANDUP).is_err());
    assert!(meeting_transcript(&allowed(), &sources, STANDUP, (None, None), now()).is_err());
}

/// What the use shows is counts and single words: no line of it holds a
/// dictation's text, or a stretch of one.
#[test]
fn usage_never_carries_a_dictations_text() {
    let lines = usage_lines(&allowed(), &fixtures(), &[], now()).join("\n");
    assert!(!lines.is_empty());
    let entries = crate::history::read_snapshot(Path::new(&fixtures().history)).unwrap();
    assert!(entries.len() >= 10);
    for entry in &entries {
        for text in [&entry.raw_text, &entry.final_text] {
            let words: Vec<&str> = text.split_whitespace().collect();
            for run in words.windows(4) {
                let run = run.join(" ");
                assert!(!lines.contains(&run), "usage quotes {run:?}:\n{lines}");
            }
        }
    }
}
