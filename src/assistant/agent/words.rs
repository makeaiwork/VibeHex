//! A turn's steps in words, in the window's language: each thing it read
//! as a line ("Searched 12 meetings: 4 match", "Read your cleanup
//! instructions"), what it is doing now, and what the folded line over an
//! answer counts. The pane draws these and nothing of its own, so an
//! agent turn's trace, a router turn's search and a Requests question
//! read the same.
//!
//! The lines say only what happened: the articles the app laid out are
//! listed, never counted as read by a tool; a call that was refused says
//! so; a step this build doesn't know says nothing.

use super::trace::{Did, Doing, Why};
use crate::assistant::guide;
use crate::assistant::recordings::Step;
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// The setting keys one line names at most.
const KEYS: usize = 4;

pub fn counted(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        tr(one).to_owned()
    } else {
        tr_fill(many, &count.to_string())
    }
}

fn meetings(count: usize) -> String {
    counted(count, "1 meeting", "{} meetings")
}

fn files(count: usize) -> String {
    counted(count, "1 file", "{} files")
}

fn dictations(count: usize) -> String {
    counted(count, "1 dictation", "{} dictations")
}

/// What a returning setup read: "your settings, 972 dictations and 5
/// meetings".
fn looked(dictations: usize, meetings: usize) -> String {
    let dictations = self::dictations(dictations);
    if meetings == 0 {
        tr_fill("your settings and {}", &dictations)
    } else {
        tr_named(
            "your settings, {dictations} and {meetings}",
            &[
                ("dictations", &dictations),
                ("meetings", &self::meetings(meetings)),
            ],
        )
    }
}

/// Guide articles read, by their titles as the guide's sources show them;
/// nothing when none of `ids` is an article.
fn articles(ids: &[String]) -> String {
    let names: Vec<&str> = ids
        .iter()
        .filter_map(|id| guide::article(id))
        .map(|article| article.title.as_str())
        .collect();
    match names.len() {
        0 => String::new(),
        1 => tr_fill("Read 1 guide article: {}", names[0]),
        count => tr_named(
            "Read {count} guide articles: {names}",
            &[("count", &count.to_string()), ("names", &names.join(", "))],
        ),
    }
}

/// A search's step in words.
pub fn step(step: &Step) -> String {
    let searched = |what: String, found: usize| {
        if found == 0 {
            tr_fill("Searched {}: none match", &what)
        } else {
            tr_named(
                "Searched {what}: {found} match",
                &[("what", &what), ("found", &found.to_string())],
            )
        }
    };
    match step {
        Step::Guide { articles } => self::articles(articles),
        Step::Meetings {
            searched: count,
            found,
        } => searched(meetings(*count), *found),
        Step::Files {
            searched: count,
            found,
        } => searched(files(*count), *found),
        Step::Dictations {
            searched: count,
            found,
        } => searched(dictations(*count), *found),
        Step::Read { notes, passages } if *notes > 0 => {
            tr_fill("Read their notes and {} passages", &passages.to_string())
        }
        Step::Read { passages, .. } => tr_fill("Read {} passages", &passages.to_string()),
        Step::Looked {
            dictations,
            meetings,
        } => tr_fill("Read {}", &looked(*dictations, *meetings)),
    }
}

/// One thing a turn did, as its lines; none for what shows nothing.
fn did(did: &Did) -> Vec<String> {
    let one = |line: String| vec![line];
    match did {
        Did::Articles { ids, .. } => one(articles(ids)),
        Did::GuideSearch { query, .. } => one(tr_fill("Searched the guide for “{}”", query)),
        Did::Settings { keys } if keys.is_empty() => {
            one(tr("Checked how VibeHex is set up").to_owned())
        }
        Did::Settings { keys } => {
            let mut named: Vec<&str> = keys.iter().take(KEYS).map(String::as_str).collect();
            if keys.len() > KEYS {
                named.push("…");
            }
            one(tr_fill("Read settings: {}", &named.join(", ")))
        }
        Did::Instructions => one(tr("Read your cleanup instructions").to_owned()),
        Did::Vocabulary { named, total } => one(tr_named(
            "Looked up Vocabulary: {named} of {total} entries",
            &[("named", &named.to_string()), ("total", &total.to_string())],
        )),
        Did::Recordings { steps } => steps.iter().map(step).collect(),
        Did::Notes { title } => one(tr_fill("Read the notes of “{}”", title)),
        Did::Transcript { title } => one(tr_fill("Read the transcript of “{}”", title)),
        Did::File { title } => one(tr_fill("Read the file “{}”", title)),
        Did::Dictation => one(tr("Read one dictation whole").to_owned()),
        Did::Usage => one(tr("Looked at how you use VibeHex: counts, never your words").to_owned()),
        Did::Releases { versions } if versions.is_empty() => {
            one(tr("Read the release notes").to_owned())
        }
        Did::Releases { versions } => {
            one(tr_fill("Read the release notes: {}", &versions.join(", ")))
        }
        Did::Consent { allowed: true } => one(tr("You allowed reading your recordings").to_owned()),
        Did::Consent { allowed: false } => {
            one(tr("You didn't allow reading your recordings").to_owned())
        }
        Did::Refused { why, .. } => refused(why),
        Did::Unknown => Vec::new(),
    }
}

/// A refusal the person should know of; a call the model got wrong (no
/// such tool, a bad id) says nothing.
fn refused(why: &str) -> Vec<String> {
    let line = if why == Why::NotAllowed.name() {
        tr("Didn't read your recordings: not allowed")
    } else if why == Why::Unanswered.name() {
        tr("Didn't read your recordings: the question went unanswered")
    } else if why == Why::NotSent.name() {
        tr("Didn't read your settings: sending them is off")
    } else if why == Why::Budget.name() {
        tr("Stopped reading: this answer has read as much as it may")
    } else {
        return Vec::new();
    };
    vec![line.to_owned()]
}

/// One line of a turn's steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line {
    pub words: String,
    /// It says what was not read (a no, a refusal): no check beside it.
    pub not: bool,
}

/// A turn's steps as its lines, in order, each said once in a row. A no
/// on the card is said by its own line, not by the refusal after it too.
pub fn lines(trace: &[Did]) -> Vec<Line> {
    let declined = trace
        .iter()
        .any(|did| matches!(did, Did::Consent { allowed: false }));
    let mut lines: Vec<Line> = Vec::new();
    for done in trace {
        if declined && done.refused_for(Why::NotAllowed).is_some() {
            continue;
        }
        let not = matches!(done, Did::Consent { allowed: false } | Did::Refused { .. });
        for words in did(done) {
            if !words.is_empty() && lines.last().is_none_or(|last| last.words != words) {
                lines.push(Line { words, not });
            }
        }
    }
    lines
}

/// A search's steps as a trace: a router turn's, or a Requests
/// question's, so every list of steps is drawn one way.
pub fn of(steps: &[Step], trace: &[Did]) -> Vec<Did> {
    let mut all = trace.to_vec();
    if !steps.is_empty() {
        all.push(Did::Recordings {
            steps: steps.to_vec(),
        });
    }
    all
}

/// What a turn is doing now. `read`: it has read something since the
/// articles the app laid out, so it is no longer only in the guide.
pub fn doing(doing: Doing, read: bool) -> &'static str {
    match doing {
        Doing::Reading("guide_search" | "guide_read") => tr("Looking in the guide"),
        Doing::Reading("recordings_search" | "meetings_list" | "files_list") => {
            tr("Searching your recordings")
        }
        Doing::Reading(
            "meeting_notes" | "meeting_transcript" | "file_read" | "dictation_read"
            | "usage_insights",
        ) => tr("Reading your recordings"),
        Doing::Reading("release_notes") => tr("Reading the release notes"),
        Doing::Reading(_) => tr("Reading your settings"),
        Doing::Thinking if read => tr("Thinking it over"),
        Doing::Thinking => tr("Looking in the guide"),
    }
}

/// It read something a tool gave: more than the articles the app laid
/// out before the model said anything.
pub fn read_more(trace: &[Did]) -> bool {
    trace
        .iter()
        .any(|did| !matches!(did, Did::Articles { app: true, .. }))
}

/// It searched the recordings, whatever that found.
pub fn searched(trace: &[Did]) -> bool {
    trace
        .iter()
        .any(|did| matches!(did, Did::Recordings { .. }))
}

fn add<'a>(known: &mut Vec<&'a str>, name: &'a str) {
    if !known.contains(&name) {
        known.push(name);
    }
}

/// What the folded line names as read, in its order: recordings, guide
/// articles a tool read, the person's settings, how they use VibeHex, the
/// release notes. Only what a tool read counts: never the articles the
/// app laid out. `other`: the recordings are other than the one a
/// Requests question was asked about.
pub fn read(trace: &[Did], other: bool) -> Vec<String> {
    let (mut meetings, mut files, mut dictations) = (0, 0, 0);
    let mut looked = None;
    let (mut guide, mut whole, mut texts): (Vec<&str>, Vec<&str>, Vec<&str>) = Default::default();
    let (mut settings, mut usage, mut releases) = (false, false, false);
    for did in trace {
        match did {
            Did::Articles { ids, app: false } => {
                ids.iter().for_each(|id| add(&mut guide, id));
            }
            Did::Recordings { steps } => {
                for step in steps {
                    match step {
                        Step::Meetings { found, .. } => meetings = meetings.max(*found),
                        Step::Files { found, .. } => files = files.max(*found),
                        Step::Dictations { found, .. } => dictations = dictations.max(*found),
                        Step::Looked {
                            dictations,
                            meetings,
                        } => looked = Some((*dictations, *meetings)),
                        Step::Guide { articles } => {
                            articles.iter().for_each(|id| add(&mut guide, id));
                        }
                        Step::Read { .. } => {}
                    }
                }
            }
            Did::Notes { title } | Did::Transcript { title } => add(&mut whole, title),
            Did::File { title } => add(&mut texts, title),
            Did::Dictation => dictations = dictations.max(1),
            Did::Settings { keys } => settings |= !keys.is_empty(),
            Did::Instructions => settings = true,
            Did::Vocabulary { named, .. } => settings |= *named > 0,
            Did::Usage => usage = true,
            Did::Releases { .. } => releases = true,
            _ => {}
        }
    }
    let meetings = meetings.max(whole.len());
    let files = files.max(texts.len());
    let mut read = Vec::new();
    match (meetings, other) {
        (0, _) => {}
        (found, true) => read.push(counted(found, "1 other meeting", "{} other meetings")),
        (found, false) => read.push(self::meetings(found)),
    }
    match (files, other) {
        (0, _) => {}
        (found, true) => read.push(counted(found, "1 other file", "{} other files")),
        (found, false) => read.push(self::files(found)),
    }
    if dictations > 0 {
        read.push(self::dictations(dictations));
    }
    if let Some((dictations, meetings)) = looked {
        read.push(self::looked(dictations, meetings));
    }
    if !guide.is_empty() {
        read.push(counted(guide.len(), "1 guide article", "{} guide articles"));
    }
    if settings {
        read.push(tr("your settings").to_owned());
    }
    if usage {
        read.push(tr("how you use VibeHex").to_owned());
    }
    if releases {
        read.push(tr("the release notes").to_owned());
    }
    read
}

/// The folded line of what was `read`, in `seconds`: its first three
/// things. None when nothing was.
pub fn folded(read: &[String], seconds: &str) -> Option<String> {
    Some(match read {
        [] => return None,
        [one] => tr_named(
            "Read {what} in {seconds} s",
            &[("what", one), ("seconds", seconds)],
        ),
        [first, second] => tr_named(
            "Read {first} and {second} in {seconds} s",
            &[("first", first), ("second", second), ("seconds", seconds)],
        ),
        [first, second, third, ..] => tr_named(
            "Read {first}, {second}, and {third} in {seconds} s",
            &[
                ("first", first),
                ("second", second),
                ("third", third),
                ("seconds", seconds),
            ],
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::i18n::speaking;

    fn titles(ids: &[&str]) -> Vec<String> {
        ids.iter()
            .map(|id| guide::article(id).unwrap().title.clone())
            .collect()
    }

    /// A recordings answer that drew on the guide too names both.
    #[test]
    fn the_steps_name_the_guide_articles_read() {
        let steps = [
            Step::Dictations {
                searched: 40,
                found: 8,
            },
            Step::Guide {
                articles: vec![
                    "topics/vocabulary".into(),
                    "topics/output-and-instructions".into(),
                ],
            },
        ];
        let trace = of(&steps, &[]);
        assert_eq!(
            folded(&read(&trace, false), "29").unwrap(),
            "Read 8 dictations and 2 guide articles in 29 s"
        );
        let titles = titles(&["topics/vocabulary", "topics/output-and-instructions"]);
        let guide = step(&steps[1]);
        assert!(guide.starts_with("Read 2 guide articles: "), "{guide}");
        assert!(guide.ends_with(&titles.join(", ")), "{guide}");
        assert_eq!(
            step(&Step::Guide {
                articles: vec!["topics/vocabulary".into()]
            }),
            format!("Read 1 guide article: {}", titles[0])
        );
        assert_eq!(lines(&trace).len(), 2);
    }

    fn said(trace: &[Did]) -> Vec<String> {
        lines(trace).into_iter().map(|line| line.words).collect()
    }

    fn agent() -> Vec<Did> {
        vec![
            Did::Articles {
                ids: vec!["topics/meetings".into(), "topics/vocabulary".into()],
                app: true,
            },
            Did::Recordings {
                steps: vec![
                    Step::Meetings {
                        searched: 12,
                        found: 3,
                    },
                    Step::Read {
                        notes: 0,
                        passages: 5,
                    },
                ],
            },
            Did::Notes {
                title: "Budget".into(),
            },
            Did::Transcript {
                title: "Budget".into(),
            },
            Did::Settings {
                keys: vec!["meetings.enabled".into()],
            },
            Did::refused("teleport", Why::Unknown),
            Did::Unknown,
        ]
    }

    /// The fold counts what tools read, never the articles the app laid
    /// out; the list names both, and nothing for a call the model got
    /// wrong or a step a later build wrote.
    #[test]
    fn the_fold_counts_only_what_tools_read() {
        let trace = agent();
        assert_eq!(read(&trace, false), ["3 meetings", "your settings"]);
        assert_eq!(
            folded(&read(&trace, false), "9").unwrap(),
            "Read 3 meetings and your settings in 9 s"
        );
        assert!(lines(&trace).iter().all(|line| !line.not));
        let lines = said(&trace);
        assert_eq!(lines.len(), 6, "{lines:?}");
        assert!(lines[0].starts_with("Read 2 guide articles: "));
        assert_eq!(lines[1], "Searched 12 meetings: 3 match");
        assert_eq!(lines[3], "Read the notes of “Budget”");
        assert_eq!(lines[5], "Read settings: meetings.enabled");

        // Only the articles laid out: nothing a tool read.
        let guide = &trace[..1];
        assert!(read(guide, false).is_empty() && !read_more(guide));
        assert!(folded(&[], "3").is_none());
        assert!(read_more(&trace) && searched(&trace));
        // A meeting read whole counts, found by a search or not.
        assert_eq!(read(&trace[2..4], true), ["1 other meeting"]);
    }

    /// A no on the card is one line, however often the model asked again.
    #[test]
    fn a_no_is_said_once() {
        let trace = [
            Did::Consent { allowed: false },
            Did::refused("recordings_search", Why::NotAllowed),
            Did::refused("meetings_list", Why::NotAllowed),
        ];
        assert_eq!(said(&trace), ["You didn't allow reading your recordings"]);
        assert!(lines(&trace)[0].not, "no check beside a no");
        // Closed before the turn: the refusal says it, once.
        assert_eq!(
            said(&trace[1..]),
            ["Didn't read your recordings: not allowed"]
        );
        assert_eq!(
            said(&[Did::refused("settings_get", Why::NotSent)]),
            ["Didn't read your settings: sending them is off"]
        );
    }

    #[test]
    fn what_goes_on_is_in_the_guide_only_while_it_is() {
        assert_eq!(doing(Doing::Thinking, false), "Looking in the guide");
        assert_eq!(doing(Doing::Thinking, true), "Thinking it over");
        assert_eq!(
            doing(Doing::Reading("guide_read"), true),
            "Looking in the guide"
        );
        assert_eq!(
            doing(Doing::Reading("recordings_search"), true),
            "Searching your recordings"
        );
        assert_eq!(
            doing(Doing::Reading("settings_get"), true),
            "Reading your settings"
        );
        // Every tool has words of its own kind.
        for tool in super::super::table::TOOLS {
            assert!(!doing(Doing::Reading(tool.name), true).is_empty());
        }
    }

    /// Every line an agent turn can show speaks all nine languages, and
    /// none joins words with a middle dot or ends in typed dots.
    #[test]
    fn the_lines_speak_every_language() {
        const WORDS: &[&str] = &[
            "Searched the guide for “{}”",
            "Checked how VibeHex is set up",
            "Read settings: {}",
            "Read your cleanup instructions",
            "Looked up Vocabulary: {named} of {total} entries",
            "Read the notes of “{}”",
            "Read the transcript of “{}”",
            "Read the file “{}”",
            "Read one dictation whole",
            "Looked at how you use VibeHex: counts, never your words",
            "Read the release notes",
            "Read the release notes: {}",
            "You allowed reading your recordings",
            "You didn't allow reading your recordings",
            "Didn't read your recordings: not allowed",
            "Didn't read your settings: sending them is off",
            "Stopped reading: this answer has read as much as it may",
            "Thinking it over",
            "Reading your recordings",
            "Reading your settings",
            "Reading the release notes",
            "your settings",
            "how you use VibeHex",
            "the release notes",
            "1 web page",
            "{} web pages",
        ];
        for code in ["pl", "zh", "ja", "de", "es", "uk", "fr", "pt", "ru"] {
            for words in WORDS {
                let shown = speaking(Some(code), || tr(words).to_owned());
                assert_ne!(shown, *words, "{code} has no row for {words}");
                assert!(!shown.contains('·'), "{code}: {shown}");
                assert!(
                    !shown.ends_with('…') && !shown.ends_with("..."),
                    "{code}: {shown}"
                );
                for hole in ["{}", "{named}", "{total}"] {
                    assert_eq!(
                        shown.matches(hole).count(),
                        words.matches(hole).count(),
                        "{code}: {shown}"
                    );
                }
            }
        }
    }
}
