//! A returning setup: someone who set VibeHex up before, or has used it a
//! while (50 dictations, or any meeting), skips round one's questions. The
//! opening says what is set now and what came since: what is new in
//! VibeHex after the last setup, and what their own use shows, which the
//! app works out only while Assistant may read recordings (counts, shares
//! and single words, `recordings::usage`). Round one stays as one row,
//! "Has anything changed?", its answers as they were, and the follow-ups
//! and the plan come straight after.

use super::past::Setup;
use super::{Answer, Ask};
use serde::{Deserialize, Serialize};

use crate::assistant::recordings::usage::{self, Language, Usage};
use crate::assistant::releases;
use crate::desktop::i18n::{tr, tr_fill, tr_named, tr_window};

/// This many dictations kept makes an install one used a while.
pub const USED_DICTATIONS: usize = 50;
/// A dictation share into code editors that leads to the code questions,
/// and one of long dictations that names the Codex trade-off, in percent.
const CODE_SHARE: u32 = 30;
const LONG_SHARE: u32 = 20;
/// The most fixes the opening names.
const FIXES_SHOWN: usize = 3;
/// The most release headlines the opening names, and use lines it lists.
const HEADLINES: usize = 5;
const BULLETS: usize = 5;

/// One VibeHex release, as its notes describe it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Release {
    pub version: String,
    pub date_ms: u64,
    /// One line of what it brings, in English.
    pub title: String,
    /// The settings and Labs features it brings or changes.
    pub keys: Vec<String>,
}

/// Whether a release dated after `after_ms` brings or changes `key`: what
/// was answered about it before may be asked again. Work not released yet
/// (a development build's "next") never counts: it isn't a release since.
pub fn changed_since(key: &str, after_ms: u64) -> bool {
    let after = date(after_ms);
    releases::notes()
        .iter()
        .any(|note| !note.is_next() && note.date > after && note.settings_keys().contains(&key))
}

/// The releases after `after_ms`, oldest first: VibeHex's own notes
/// (`releases::notes`) dated after that day, up to the version running
/// (a newer one isn't here yet). Next, the work not released yet, only on
/// a development build, which runs it.
pub fn releases_after(after_ms: u64) -> Vec<Release> {
    releases_from(
        releases::notes(),
        after_ms,
        &releases::Running::here(),
        now_ms(),
    )
}

fn releases_from(
    notes: &[releases::Note],
    after_ms: u64,
    running: &releases::Running,
    now_ms: u64,
) -> Vec<Release> {
    let after = (after_ms > 0).then(|| date(after_ms));
    let up_to = releases::order(&running.version);
    notes
        .iter()
        .filter(|note| {
            if note.is_next() {
                return running.dev;
            }
            let Some(at) = releases::order(&note.version) else {
                return false;
            };
            let here = running.dev || up_to.is_some_and(|up_to| at <= up_to);
            // A release the same day as the setup was most likely in it.
            here && after.as_ref().is_none_or(|after| note.date > *after)
        })
        .map(|note| Release {
            version: note.version.clone(),
            date_ms: day_ms(&note.date).unwrap_or(now_ms),
            title: note.headline(),
            keys: note
                .settings_keys()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        })
        .collect()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

/// `2026-11-06` as its first millisecond, in UTC; `None` for anything else.
fn day_ms(day: &str) -> Option<u64> {
    let mut parts = day.splitn(3, '-').map(|part| part.parse::<i64>().ok());
    let (year, month, day) = (parts.next()??, parts.next()??, parts.next()??);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Howard Hinnant's days_from_civil.
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days).ok().map(|days| days * 86_400_000)
}

/// What a returning setup opens with.
#[derive(Clone, Debug, Default)]
pub struct Since {
    /// The last setup, when there was one.
    pub last: Option<Setup>,
    pub releases: Vec<Release>,
    /// Without a setup before, the first version this install ran, which
    /// the releases are since.
    pub from: Option<String>,
    /// The person's use; `None` while Assistant may not read recordings.
    pub usage: Option<Usage>,
}

/// Whether setup returns: a setup was finished before, or this install
/// has been used a while.
pub fn returning(setups: &[Setup], dictations: usize, meetings: usize) -> bool {
    !setups.is_empty() || dictations >= USED_DICTATIONS || meetings > 0
}

/// The newest setup, if any.
pub fn last(setups: &[Setup]) -> Option<&Setup> {
    setups.iter().max_by_key(|setup| setup.at_ms)
}

/// Round one as it was answered last time, the rest as the settings have
/// it; a use the person's dictations show (code editors) joins.
pub fn preselect(asks: &mut [Ask], since: &Since) {
    let answers = since
        .last
        .as_ref()
        .map(|last| last.answers.as_slice())
        .unwrap_or_default();
    for ask in asks.iter_mut() {
        let Some(answer) = answers.iter().find(|answer| answer.id == ask.id) else {
            continue;
        };
        let chosen: Vec<String> = answer
            .chosen
            .iter()
            .filter(|id| ask.options.iter().any(|option| option.id == **id))
            .cloned()
            .collect();
        if !chosen.is_empty() {
            ask.chosen = chosen;
            ask.touched = true;
        }
    }
    let code = since
        .usage
        .as_ref()
        .is_some_and(|usage| usage.code_share >= CODE_SHARE);
    if let Some(ask) = asks.iter_mut().find(|ask| ask.id == "use")
        && code
        && !ask.chosen.iter().any(|id| id == "dictation_code")
    {
        ask.chosen.push("dictation_code".into());
        ask.touched = true;
    }
}

/// The opening's paragraph, in the window's language, as Markdown: one
/// line of the updates since (a few of their headlines and where to hear
/// more), then what the use shows as a short list, or that it could, if
/// allowed.
pub fn paragraph(since: &Since) -> String {
    let mut blocks = Vec::new();
    blocks.extend(releases_line(since));
    match &since.usage {
        Some(usage) => {
            let bullets: Vec<String> = use_parts(usage)
                .into_iter()
                .take(BULLETS)
                .map(|part| format!("- {part}"))
                .collect();
            if !bullets.is_empty() {
                blocks.push(format!("{}\n\n{}", tr("What your use shows:"), bullets.join("\n")));
            }
        }
        // The place keeps the window's words, as the window names it.
        None => blocks.push(format!(
            "{}.",
            tr_fill(
                "Your dictations and meetings could show more here, if Assistant may read them ({})",
                &format!("{} › {}", tr_window("Assistant"), tr_window("Settings"))
            )
        )),
    }
    blocks.join("\n\n")
}

/// "5 updates since your last setup (2026-11-06): Assistant, Files, …
/// Ask “What's new?” to hear more." `None` with nothing to name and no
/// setup before.
fn releases_line(since: &Since) -> Option<String> {
    let count = since.releases.len();
    if count == 0 {
        return since.last.as_ref().map(|last| {
            format!(
                "{}.",
                tr_fill(
                    "Nothing new in VibeHex since your last setup ({})",
                    &date(last.at_ms)
                )
            )
        });
    }
    let named = headlines(&since.releases);
    let list = named
        .iter()
        .take(HEADLINES)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    // A list cut short trails off; a whole one ends.
    let end = if named.len() > HEADLINES {
        ", …"
    } else {
        "."
    };
    let number = count.to_string();
    let day = since
        .last
        .as_ref()
        .map(|last| date(last.at_ms))
        .unwrap_or_default();
    let version = since.from.clone().unwrap_or_default();
    let words = match (since.last.is_some(), since.from.is_some(), count) {
        (true, _, 1) => "1 update since your last setup ({date}): {list}",
        (true, ..) => "{count} updates since your last setup ({date}): {list}",
        (false, true, 1) => "1 update since {version}: {list}",
        (false, true, _) => "{count} updates since {version}: {list}",
        (false, false, 1) => "1 VibeHex update: {list}",
        (false, false, _) => "{count} VibeHex updates: {list}",
    };
    let line = tr_named(
        words,
        &[
            ("count", &number),
            ("date", &day),
            ("version", &version),
            ("list", &list),
        ],
    );
    Some(format!(
        "{line}{end} {}.",
        tr("Ask “What's new?” to hear more")
    ))
}

/// The releases' headlines, newest first, each feature once: a title
/// "Files, Stop shortcut, and AI cleanup on Codex" names three.
fn headlines(releases: &[Release]) -> Vec<String> {
    let mut named: Vec<String> = Vec::new();
    for release in releases.iter().rev() {
        for part in release.title.split(", ") {
            let part = part.trim();
            let part = part
                .strip_prefix("and ")
                .or_else(|| part.strip_prefix("with "))
                .unwrap_or(part)
                .trim_end_matches('.');
            if !part.is_empty() && !named.iter().any(|known| known.eq_ignore_ascii_case(part)) {
                named.push(part.to_owned());
            }
        }
        if release.title.trim().is_empty() && !named.contains(&release.version) {
            named.push(release.version.clone());
        }
    }
    named
}

/// What the use shows, one short line each: the dictations of the last
/// 30 days, where most go and the code share, the long ones, what
/// cleanup keeps fixing, the meetings.
fn use_parts(usage: &Usage) -> Vec<String> {
    let mut parts = Vec::new();
    parts.push(match (usage.last_30_days, usage.dictations) {
        (1, _) => tr("1 dictation in the last 30 days").to_owned(),
        (0, 1) => tr("1 dictation kept").to_owned(),
        (0, kept) => tr_fill("{} dictations kept", &kept.to_string()),
        (recent, _) => tr_fill("{} dictations in the last 30 days", &recent.to_string()),
    });
    if !usage.apps.is_empty() {
        let apps: Vec<&str> = usage
            .apps
            .iter()
            .take(3)
            .map(|(app, _)| app.as_str())
            .collect();
        let apps = apps.join(", ");
        parts.push(if usage.code_share >= CODE_SHARE {
            tr_named(
                "Mostly into {apps}, {share}% into a terminal or a code editor",
                &[("apps", &apps), ("share", &usage.code_share.to_string())],
            )
        } else {
            tr_fill("Dictated into most: {}", &apps)
        });
    }
    if usage.long_share >= LONG_SHARE {
        parts.push(tr_fill(
            "{}% are long: Codex cleans long ones up better, this computer is faster",
            &usage.long_share.to_string(),
        ));
    }
    if !usage.fixes.is_empty() {
        let fixes: Vec<String> = usage
            .fixes
            .iter()
            .take(FIXES_SHOWN)
            .map(|fix| format!("{} → {}", fix.heard, fix.written))
            .collect();
        parts.push(tr_fill("Cleanup keeps fixing {}", &fixes.join(", ")));
    }
    if usage.meetings > 0 {
        let mut meetings = if usage.meetings == 1 {
            tr("1 meeting recorded").to_owned()
        } else {
            tr_fill("{} meetings recorded", &usage.meetings.to_string())
        };
        if let Some((language, _)) = usage.meeting_languages.first() {
            let mostly = match language {
                Language::Russian => "mostly in Russian",
                Language::Latin => "mostly in English or another language",
            };
            meetings = format!("{meetings}, {}", tr(mostly));
        }
        parts.push(meetings);
    }
    parts
}

/// What the model reads of it, in English: the last setup's date,
/// answers, and what it applied and declined; the releases since; the
/// use's aggregates, or that they weren't read. Never a recording's text.
pub fn prompt(since: &Since) -> String {
    let mut lines = Vec::new();
    match &since.last {
        Some(last) => {
            lines.push(format!("last setup: {}", date(last.at_ms)));
            lines.push(format!("answers then: {}", answers_line(&last.answers)));
            for (name, done) in [
                ("applied then", &last.applied),
                ("declined then", &last.declined),
            ] {
                if !done.is_empty() {
                    let keys: Vec<String> = done
                        .iter()
                        .map(|line| format!("{}={}", line.key, line.to))
                        .collect();
                    lines.push(format!("{name}: {}", keys.join(", ")));
                }
            }
        }
        None => lines.push("no setup before; this install has been used a while".into()),
    }
    if let Some(from) = since.from.as_ref().filter(|_| since.last.is_none()) {
        lines.push(format!("first version run here: {from}"));
    }
    for release in &since.releases {
        lines.push(format!(
            "new in VibeHex {}: {} ({})",
            release.version,
            release.title,
            release.keys.join(", ")
        ));
    }
    match &since.usage {
        Some(usage) => lines.extend(usage::lines(usage)),
        None => lines.push("their use: not read (Assistant may not read recordings)".into()),
    }
    lines.join("\n")
}

fn answers_line(answers: &[Answer]) -> String {
    answers
        .iter()
        .map(|answer| format!("{}: {}", answer.id, answer.chosen.join(", ")))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A term the use shows Vocabulary lacks, for setup's plan: `sure` when
/// cleanup turned the same heard word into exactly it often
/// (`usage::Term::sure`), and then offered checked.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Candidate {
    pub term: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sounds_like: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub sure: bool,
}

/// The terms the use shows, for the plan.
pub fn candidates(usage: &Usage) -> Vec<Candidate> {
    usage
        .terms
        .iter()
        .map(|term| Candidate {
            term: term.word.clone(),
            // How cleanup heard it, else how History has it heard.
            sounds_like: match term.heard.is_empty() {
                true => term.sounds.clone(),
                false => term.heard.clone(),
            },
            sure: term.sure(),
        })
        .collect()
}

/// `2026-10-12`, in UTC.
pub fn date(ms: u64) -> String {
    let days = (ms / 86_400_000) as i64;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Every English phrase the opening shows, for the translation tables'
/// test.
#[cfg(test)]
pub fn words() -> Vec<&'static str> {
    vec![
        "What your use shows:",
        "Your dictations and meetings could show more here, if Assistant may read them ({})",
        "Nothing new in VibeHex since your last setup ({})",
        "1 update since your last setup ({date}): {list}",
        "{count} updates since your last setup ({date}): {list}",
        "1 update since {version}: {list}",
        "{count} updates since {version}: {list}",
        "1 VibeHex update: {list}",
        "{count} VibeHex updates: {list}",
        "Ask “What's new?” to hear more",
        "1 dictation in the last 30 days",
        "{} dictations in the last 30 days",
        "1 dictation kept",
        "{} dictations kept",
        "Mostly into {apps}, {share}% into a terminal or a code editor",
        "Dictated into most: {}",
        "{}% are long: Codex cleans long ones up better, this computer is faster",
        "Cleanup keeps fixing {}",
        "1 meeting recorded",
        "{} meetings recorded",
        "mostly in Russian",
        "mostly in English or another language",
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::assistant::recordings::usage::{Fix, Term};
    use crate::assistant::setup::questions;

    fn usage() -> Usage {
        Usage {
            dictations: 120,
            last_30_days: 40,
            apps: vec![("Ghostty".into(), 50), ("Slack".into(), 30)],
            code_share: 45,
            long_share: 25,
            median_words: 30,
            terms: vec![Term {
                word: "GitHub".into(),
                count: 9,
                heard: vec!["гитхаб".into()],
                fixed: 9,
                sounds: Vec::new(),
            }],
            fixes: vec![Fix {
                heard: "гитхаб".into(),
                written: "GitHub".into(),
                count: 9,
            }],
            meetings: 3,
            ..Usage::default()
        }
    }

    #[test]
    fn a_date_reads_as_its_day() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_795_714_800_000), "2026-11-26");
    }

    /// The last answers come back, and code editors in the use add code.
    #[test]
    fn round_one_comes_back_as_last_answered() {
        let last = Setup {
            at_ms: 1,
            answers: vec![
                Answer {
                    id: "use".into(),
                    chosen: vec!["dictation_any".into()],
                    ..Answer::default()
                },
                Answer {
                    id: "cleanup".into(),
                    chosen: vec!["codex".into()],
                    ..Answer::default()
                },
            ],
            ..Setup::default()
        };
        let mut asks = questions("macos");
        let since = Since {
            last: Some(last),
            usage: Some(usage()),
            ..Since::default()
        };
        preselect(&mut asks, &since);
        assert_eq!(asks[0].chosen, ["dictation_any", "dictation_code"]);
        assert_eq!(asks[2].chosen, ["codex"]);
        assert!(asks[2].touched);
    }

    /// The opening names the use; without leave to read, says it could.
    /// The model reads the aggregates and the last setup, never text.
    #[test]
    fn the_opening_and_the_prompt_say_what_came_since() {
        let since = Since {
            last: Some(Setup {
                at_ms: 1_795_714_800_000,
                declined: vec![super::super::past::Line {
                    key: "labs.playback".into(),
                    to: Value::Bool(true),
                    from: Value::Bool(false),
                }],
                ..Setup::default()
            }),
            usage: Some(usage()),
            ..Since::default()
        };
        let shown = paragraph(&since);
        assert!(shown.contains("2026-11-26"), "{shown}");
        assert!(shown.contains("45%") && shown.contains("25%"), "{shown}");
        assert!(shown.contains("гитхаб → GitHub"), "{shown}");
        let read = prompt(&since);
        assert!(read.contains("declined then: labs.playback=true"), "{read}");
        assert!(read.contains("GitHub x9"), "{read}");
        let unread = Since {
            usage: None,
            ..since
        };
        assert!(paragraph(&unread).contains("Assistant › Settings"));
        assert!(prompt(&unread).contains("not read"));
        assert!(!returning(&[], 49, 0) && returning(&[], 50, 0) && returning(&[], 0, 1));
    }

    fn release(version: &str, title: &str) -> Release {
        Release {
            version: version.into(),
            title: title.into(),
            ..Release::default()
        }
    }

    /// The owner's opening, short: the updates in one line, newest
    /// headlines first, no version and no repeated "New in VibeHex"; the
    /// use as at most five lines, the last 30 days' dictations once, and
    /// no count of terms (they go to the plan).
    #[test]
    fn the_opening_is_one_line_of_updates_and_a_short_list() {
        let since = Since {
            releases: vec![
                release(
                    "3.3.0-vibe.4",
                    "What an update brings, Add to vocabulary from a selection, and Cleanup progress",
                ),
                release(
                    "3.3.0-vibe.5",
                    "Add to vocabulary hears what the speech model heard, and built-in models' meters",
                ),
                release(
                    "3.3.0-vibe.6",
                    "Files, Stop shortcut, Listen to the recording, and AI cleanup on Codex",
                ),
                release(
                    "next",
                    "Assistant, VibeHex in Russian, Screenshots, and more speech models",
                ),
            ],
            from: Some("3.3.0-vibe.3".into()),
            usage: Some(Usage {
                dictations: 972,
                last_30_days: 972,
                apps: vec![
                    ("Code".into(), 700),
                    ("Anty".into(), 180),
                    ("Sublime Text".into(), 35),
                ],
                code_share: 76,
                long_share: 17,
                fixes: vec![Fix {
                    heard: "кодекс".into(),
                    written: "Codex".into(),
                    count: 26,
                }],
                terms: usage().terms,
                meetings: 5,
                meeting_languages: vec![(Language::Russian, 5)],
                ..Usage::default()
            }),
            ..Since::default()
        };
        let shown = paragraph(&since);
        let blocks: Vec<&str> = shown.split("\n\n").collect();
        assert_eq!(
            blocks[0],
            "4 updates since 3.3.0-vibe.3: Assistant, VibeHex in Russian, Screenshots, \
             more speech models, Files, … Ask “What's new?” to hear more."
        );
        assert!(
            !shown.contains("New in VibeHex") && !shown.contains("vibe.6"),
            "{shown}"
        );
        let bullets: Vec<&str> = shown
            .lines()
            .filter(|line| line.starts_with("- "))
            .collect();
        assert_eq!(
            bullets,
            [
                "- 972 dictations in the last 30 days",
                "- Mostly into Code, Anty, Sublime Text, 76% into a terminal or a code editor",
                "- Cleanup keeps fixing кодекс → Codex",
                "- 5 meetings recorded, mostly in Russian",
            ]
        );
        assert!(
            !shown.contains("kept") && !shown.contains("Vocabulary"),
            "{shown}"
        );
        // One release, since the last setup; nothing since, said so.
        let one = Since {
            last: Some(Setup {
                at_ms: 1_795_714_800_000,
                ..Setup::default()
            }),
            releases: vec![release("3.3.0-vibe.6", "Files, and Stop shortcut")],
            ..Since::default()
        };
        assert!(
            paragraph(&one).starts_with(
                "1 update since your last setup (2026-11-26): Files, Stop shortcut. Ask"
            ),
            "{}",
            paragraph(&one)
        );
        let none = Since {
            releases: Vec::new(),
            ..one
        };
        assert!(paragraph(&none).starts_with("Nothing new in VibeHex since your last setup"));
    }

    fn note(version: &str, date: &str) -> releases::Note {
        releases::Note {
            version: version.into(),
            date: date.into(),
            title: format!("What {version} brings"),
            labs: vec!["files".into()],
            keys: vec!["ui_language".into()],
            body: String::new(),
        }
    }

    fn running(version: &str, dev: bool) -> releases::Running {
        releases::Running {
            version: version.into(),
            dev,
            previous: None,
        }
    }

    /// The releases dated after the setup's day, oldest first, up to the
    /// one running; Next only on a development build.
    #[test]
    fn the_releases_since_are_those_after_its_day_and_here() {
        let notes = [
            note("3.3.0-vibe.4", "2026-11-04"),
            note("3.3.0-vibe.5", "2026-11-05"),
            note("3.3.0-vibe.6", "2026-11-06"),
            note("next", "unreleased"),
        ];
        let setup = day_ms("2026-11-04").unwrap() + 15 * 3_600_000;
        let versions = |releases: Vec<Release>| -> Vec<String> {
            releases
                .into_iter()
                .map(|release| release.version)
                .collect()
        };
        let running5 = running("3.3.0-vibe.5", false);
        assert_eq!(
            versions(releases_from(&notes, setup, &running5, 9)),
            ["3.3.0-vibe.5"]
        );
        let released = releases_from(&notes, 0, &running("3.3.0-vibe.6", false), 9);
        assert_eq!(released.len(), 3, "every release, never Next");
        assert_eq!(released[2].title, "What 3.3.0-vibe.6 brings");
        assert_eq!(released[2].keys, ["labs.files", "ui_language"]);
        assert_eq!(released[0].date_ms, day_ms("2026-11-04").unwrap());
        assert_eq!(
            versions(releases_from(&notes, setup, &running("3.3.0", true), 9)),
            ["3.3.0-vibe.5", "3.3.0-vibe.6", "next"]
        );
        assert_eq!(date(day_ms("2026-11-26").unwrap()), "2026-11-26");
        assert_eq!(day_ms("unreleased"), None);
    }

    /// The real notes give each release a title.
    #[test]
    fn the_shipped_releases_have_titles() {
        let all = releases_from(releases::notes(), 0, &running("3.3.0", true), 9);
        assert!(all.len() >= 2);
        for release in &all {
            assert!(
                !release.title.is_empty(),
                "{} has no title",
                release.version
            );
            assert!(!release.title.contains('\n'), "{}", release.version);
        }
        assert_eq!(all.last().unwrap().version, releases::NEXT);
    }

    /// The terms go to the plan, sure when one heard word was fixed into
    /// exactly them three times or more.
    #[test]
    fn the_terms_become_candidates_for_the_plan() {
        let mut usage = usage();
        usage.terms.push(Term {
            word: "Supabase".into(),
            count: 4,
            heard: vec!["супабейс".into()],
            fixed: 2,
            sounds: Vec::new(),
        });
        // Never fixed: how History has it heard is how it sounds.
        usage.terms.push(Term {
            word: "LEPS".into(),
            count: 5,
            heard: Vec::new(),
            fixed: 0,
            sounds: vec!["лепс".into()],
        });
        let candidates = candidates(&usage);
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].term, "GitHub");
        assert_eq!(candidates[0].sounds_like, ["гитхаб"]);
        assert!(candidates[0].sure && !candidates[1].sure);
        assert_eq!(candidates[2].sounds_like, ["лепс"]);
        assert!(!candidates[2].sure);
    }
}
