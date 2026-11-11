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
use crate::app_settings::AppSettings;
use crate::assistant::changes::parse::Proposal;
use crate::assistant::changes::{Card, vocab};
use crate::assistant::recordings::usage::{self, Language, Usage};
use crate::desktop::i18n::{tr, tr_fill};

/// This many dictations kept makes an install one used a while.
pub const USED_DICTATIONS: usize = 50;
/// A dictation share into code editors that leads to the code questions,
/// and one of long dictations that names the Codex trade-off, in percent.
const CODE_SHARE: u32 = 30;
const LONG_SHARE: u32 = 20;
/// The most fixes the opening names.
const FIXES_SHOWN: usize = 3;
/// The most releases the opening names.
const RELEASES_SHOWN: usize = 5;

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

/// The releases after `after_ms`, oldest first.
// TODO(feat/assistant-releases): the releases branch adds
// `docs/releases/<version>.md`; read them here (version, date, a title,
// and the `keys:` they bring) once it lands. Until then nothing is new.
pub fn releases_after(_after_ms: u64) -> Vec<Release> {
    Vec::new()
}

/// What a returning setup opens with.
#[derive(Clone, Debug, Default)]
pub struct Since {
    /// The last setup, when there was one.
    pub last: Option<Setup>,
    pub releases: Vec<Release>,
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

/// The opening's paragraph, in the window's language: since when, what is
/// new, what the use shows (or that it could, if allowed).
pub fn paragraph(since: &Since) -> String {
    let heading = match &since.last {
        Some(last) => tr_fill("Since your last setup ({}):", &date(last.at_ms)),
        None => tr("What your use shows:").to_owned(),
    };
    let mut parts = Vec::new();
    for release in since.releases.iter().rev().take(RELEASES_SHOWN) {
        parts.push(tr_named_release(release));
    }
    match &since.usage {
        Some(usage) => parts.extend(use_parts(usage)),
        None => parts.push(
            tr("Your dictations and meetings could show more here, if Assistant may read them (Assistant › Settings)")
                .to_owned(),
        ),
    }
    if parts.is_empty() {
        parts.push(tr("Nothing new since then").to_owned());
    }
    format!("**{heading}** {}.", parts.join(". "))
}

fn tr_named_release(release: &Release) -> String {
    let what = if release.title.is_empty() {
        release.version.clone()
    } else {
        format!("{} ({})", release.title, release.version)
    };
    tr_fill("New in VibeHex: {}", &what)
}

fn use_parts(usage: &Usage) -> Vec<String> {
    let mut parts = Vec::new();
    parts.push(format!(
        "{}, {}",
        tr_fill("{} dictations kept", &usage.dictations.to_string()),
        tr_fill("{} in the last 30 days", &usage.last_30_days.to_string())
    ));
    if !usage.apps.is_empty() {
        let apps: Vec<&str> = usage
            .apps
            .iter()
            .take(3)
            .map(|(app, _)| app.as_str())
            .collect();
        parts.push(tr_fill("Dictated into most: {}", &apps.join(", ")));
    }
    if usage.code_share >= CODE_SHARE {
        parts.push(tr_fill(
            "{}% go into a terminal or a code editor",
            &usage.code_share.to_string(),
        ));
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
    match usage.terms.len() {
        0 => {}
        1 => parts.push(tr("1 term you say isn't in Vocabulary yet").to_owned()),
        terms => parts.push(tr_fill(
            "{} terms you say aren't in Vocabulary yet",
            &terms.to_string(),
        )),
    }
    if usage.meetings > 0 {
        let mut meetings = tr_fill("{} meetings recorded", &usage.meetings.to_string());
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

/// The terms the use shows Vocabulary lacks, as one card to add them,
/// each with how it was heard.
pub fn terms_card(usage: &Usage, settings: &AppSettings) -> Option<Card> {
    let mut card: Option<Card> = None;
    for term in &usage.terms {
        let proposal = Proposal {
            key: vocab::KEY.into(),
            value: Some(
                vocab::Term {
                    term: term.word.clone(),
                    sounds_like: term.heard.clone(),
                }
                .value(),
            ),
            why: tr("You say it often").into(),
            ..Proposal::default()
        };
        let Ok(Some(one)) = vocab::card(&proposal, settings) else {
            continue;
        };
        match &mut card {
            Some(card) => vocab::merge(card, one),
            None => card = Some(one),
        }
    }
    card
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
        "Since your last setup ({}):",
        "What your use shows:",
        "Your dictations and meetings could show more here, if Assistant may read them (Assistant › Settings)",
        "Nothing new since then",
        "New in VibeHex: {}",
        "{} dictations kept",
        "{} in the last 30 days",
        "Dictated into most: {}",
        "{}% go into a terminal or a code editor",
        "{}% are long: Codex cleans long ones up better, this computer is faster",
        "Cleanup keeps fixing {}",
        "1 term you say isn't in Vocabulary yet",
        "{} terms you say aren't in Vocabulary yet",
        "{} meetings recorded",
        "mostly in Russian",
        "mostly in English or another language",
        "You say it often",
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
        assert!(returning(&[], 49, 0) == false && returning(&[], 50, 0) && returning(&[], 0, 1));
    }

    #[test]
    fn the_terms_become_one_card() {
        let card = terms_card(&usage(), &AppSettings::default()).unwrap();
        assert_eq!(card.items.len(), 1);
        let terms = vocab::terms(&card.items[0].to);
        assert_eq!(terms[0].term, "GitHub");
        assert_eq!(terms[0].sounds_like, ["гитхаб"]);
    }
}
