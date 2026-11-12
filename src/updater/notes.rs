//! What a staged update changed: the channel's notes
//! (`<channel>-notes.json` beside its manifest, which
//! `scripts/publish-vibehex.sh` keeps) from the offered release back to the
//! running one, so releases skipped in between are listed too. They show in
//! the update sheet and under Restart to update. Notes are a nicety: any
//! failure leaves the update with only its version.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use super::transport::ManifestCache;
use crate::desktop::host::{ReleaseNotes, UpdateNotes};

/// Where What's new goes when a release names no page of its own.
const PAGE: &str = "https://github.com/makeaiwork/hex/releases";
/// A release's own notes, as `scripts/publish-vibehex.sh` tags it.
const RELEASES: &str = "https://github.com/makeaiwork/hex/releases/tag";
const TIMEOUT: Duration = Duration::from_secs(30);
/// Releases listed, newest first; an install further behind sees the latest.
const MAX_RELEASES: usize = 20;
/// Changes listed per release.
const MAX_ITEMS: usize = 40;
const MAX_TITLE: usize = 200;

/// The notes beside `manifest_url`: `stable.json` has `stable-notes.json`.
pub fn url(manifest_url: &str) -> String {
    match manifest_url.strip_suffix(".json") {
        Some(base) => format!("{base}-notes.json"),
        None => format!("{manifest_url}-notes.json"),
    }
}

/// Fetches the notes for `offered`, the staged release, since `running`.
/// Without notes it still names `offered`.
pub fn fetch(
    url: &str,
    user_agent: &str,
    scratch: &Path,
    running: &str,
    offered: &str,
) -> UpdateNotes {
    let _ = std::fs::create_dir_all(scratch);
    ManifestCache::default()
        .fetch(url, user_agent, scratch, TIMEOUT)
        .inspect_err(|error| tracing::info!(%error, "no notes for the update"))
        .ok()
        .and_then(|text| parse(&text, running, offered))
        .unwrap_or_else(|| bare(offered))
}

/// An update whose notes don't say what changed.
pub fn bare(offered: &str) -> UpdateNotes {
    UpdateNotes {
        items: Vec::new(),
        url: PAGE.into(),
        version: offered.into(),
        releases: Vec::new(),
    }
}

/// Where Download update goes when the release names neither a file for
/// this install nor a page of its own: the fork's releases, not Hex's site.
pub fn download_page() -> String {
    PAGE.into()
}

fn version(build: &Value) -> Option<&str> {
    build.get("version").and_then(Value::as_str)
}

fn clean(title: &str) -> String {
    title
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_TITLE)
        .collect::<String>()
        .trim()
        .to_string()
}

/// The release's `YYYY-MM-DD` from its `published_at`.
fn date(build: &Value) -> Option<String> {
    let published = build.get("published_at").and_then(Value::as_str)?;
    let day = published.get(..10)?;
    let shaped = day.bytes().enumerate().all(|(index, byte)| match index {
        4 | 7 => byte == b'-',
        _ => byte.is_ascii_digit(),
    });
    shaped.then(|| day.to_string())
}

/// A plain `major.minor.patch`, with no suffix.
fn is_stable_version(version: &str) -> bool {
    fn digits(part: Option<&str>) -> bool {
        part.is_some_and(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    }
    let mut parts = version.split('.');
    digits(parts.next()) && digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// A VibeHex release's version: `major.minor.patch-vibe.N`.
fn is_vibe_version(version: &str) -> bool {
    version.split_once("-vibe.").is_some_and(|(stable, build)| {
        is_stable_version(stable)
            && !build.is_empty()
            && build.bytes().all(|byte| byte.is_ascii_digit())
    })
}

/// Where the release notes for a running version are: the fork's GitHub
/// release for a VibeHex or plain release, and nothing for any other
/// version. The sidebar asks only on a release build, so a local or test
/// build with a plain version still shows no link.
pub fn release_notes_url(running: &str) -> Option<String> {
    (is_vibe_version(running) || is_stable_version(running))
        .then(|| format!("{RELEASES}/v{running}"))
}

/// The releases from `offered` (newest first) back to, not including,
/// `running`; every listed one up to the cap when `running` is older than
/// the notes go. `None` when the notes don't list `offered`.
pub fn parse(text: &str, running: &str, offered: &str) -> Option<UpdateNotes> {
    let value: Value = serde_json::from_str(text).ok()?;
    let builds = value.get("builds")?.as_array()?;
    let start = builds
        .iter()
        .position(|build| version(build) == Some(offered))?;
    let page = builds[start]
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with("https://"))
        .map_or_else(|| PAGE.to_string(), str::to_string);
    let mut items: Vec<String> = Vec::new();
    let mut releases = Vec::new();
    for build in &builds[start..] {
        let Some(name) = version(build) else {
            continue;
        };
        if name == running || releases.len() == MAX_RELEASES {
            break;
        }
        let mut changes: Vec<String> = Vec::new();
        let titles = build
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("title").and_then(Value::as_str));
        for title in titles.map(clean) {
            if title.is_empty() || changes.contains(&title) || changes.len() == MAX_ITEMS {
                continue;
            }
            if !items.contains(&title) && items.len() < MAX_ITEMS {
                items.push(title.clone());
            }
            changes.push(title);
        }
        releases.push(ReleaseNotes {
            version: clean(name),
            date: date(build),
            items: changes,
        });
    }
    Some(UpdateNotes {
        items,
        url: page,
        version: offered.into(),
        releases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOTES: &str = r#"{"schema": 1, "builds": [
      {"version": "3.3.0-vibe.5", "published_at": "2026-11-06T10:00:00Z", "items": [{"title": "Newer"}]},
      {"version": "3.3.0-vibe.4", "published_at": "2026-11-05T09:30:00Z", "url": "https://github.com/makeaiwork/hex/releases/tag/v3.3.0-vibe.4", "items": [{"title": "Fix A"}, {"title": "Fix B"}]},
      {"version": "3.3.0-vibe.3", "published_at": "soon", "items": []},
      {"version": "3.3.0-vibe.2", "items": [{"title": "Fix A"}, {"title": "Fix C\u0007"}, {"title": "Fix C"}]},
      {"version": "3.3.0-vibe.1", "published_at": "2026-11-03T16:29:00Z", "items": [{"title": "Already running"}]}
    ]}"#;

    #[test]
    fn lists_every_release_from_the_offered_one_back_to_the_running_one() {
        let notes = parse(NOTES, "3.3.0-vibe.1", "3.3.0-vibe.4").unwrap();
        assert_eq!(notes.version, "3.3.0-vibe.4");
        assert_eq!(notes.items, ["Fix A", "Fix B", "Fix C"]);
        let versions: Vec<_> = notes.releases.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(versions, ["3.3.0-vibe.4", "3.3.0-vibe.3", "3.3.0-vibe.2"]);
        assert_eq!(notes.releases[0].date.as_deref(), Some("2026-11-05"));
        assert_eq!(notes.releases[0].items, ["Fix A", "Fix B"]);
        assert_eq!(notes.releases[1].date, None, "a malformed date is left out");
        assert!(notes.releases[1].items.is_empty());
        assert_eq!(notes.releases[2].items, ["Fix A", "Fix C"]);
        assert_eq!(
            notes.url,
            "https://github.com/makeaiwork/hex/releases/tag/v3.3.0-vibe.4"
        );
    }

    #[test]
    fn an_install_older_than_the_notes_gets_every_listed_release() {
        let notes = parse(NOTES, "3.2.0", "3.3.0-vibe.2").unwrap();
        let versions: Vec<_> = notes.releases.iter().map(|r| r.version.as_str()).collect();
        assert_eq!(versions, ["3.3.0-vibe.2", "3.3.0-vibe.1"]);
        assert_eq!(notes.url, PAGE, "a release without its own page");
    }

    #[test]
    fn no_notes_without_the_offered_release() {
        assert!(parse(NOTES, "3.3.0-vibe.1", "3.3.0-vibe.99").is_none());
        assert!(parse("not json", "3.3.0-vibe.1", "3.3.0-vibe.4").is_none());
        let bare = bare("3.3.0-vibe.99");
        assert_eq!(bare.version, "3.3.0-vibe.99");
        assert!(bare.releases.is_empty() && bare.items.is_empty());
    }

    #[test]
    fn a_download_without_a_page_opens_the_forks_releases() {
        assert_eq!(download_page(), PAGE);
        assert!(download_page().starts_with("https://github.com/makeaiwork/"));
    }

    #[test]
    fn the_notes_sit_beside_the_manifest() {
        assert_eq!(
            url("https://github.com/makeaiwork/hex/releases/download/updates/stable.json"),
            "https://github.com/makeaiwork/hex/releases/download/updates/stable-notes.json"
        );
    }

    #[test]
    fn a_vibehex_release_opens_its_github_release() {
        assert_eq!(
            release_notes_url("3.3.0-vibe.6").as_deref(),
            Some("https://github.com/makeaiwork/hex/releases/tag/v3.3.0-vibe.6")
        );
        assert_eq!(
            release_notes_url("3.3.0").as_deref(),
            Some("https://github.com/makeaiwork/hex/releases/tag/v3.3.0")
        );
    }

    #[test]
    fn an_odd_version_has_no_release_notes() {
        for version in [
            "0.0.0-dev-local-abc1234",
            "0.0.0-local-abc1234-dirty",
            "3.3",
            "3.3.0.1",
            "3.3.0-beta.1",
            "3.3.0-dev-4",
            "0.0.0-dev-12",
            "3.3.0-vibe.",
            "3.3.0-vibe.x",
            "v3.3.0",
            "",
        ] {
            assert_eq!(release_notes_url(version), None, "{version}");
        }
    }
}
