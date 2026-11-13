//! The VibeHex versions this install has run, each with when it first ran,
//! so "what's new" means what came since the one before:
//! `<support>/assistant/versions.json`, the newest 20. Only a release
//! records itself; a development build leaves the list as it was. Beside
//! them, `opened`: the last version Assistant was opened on after an
//! update, so the sidebar's mark for an update shows until then and never
//! again for it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The most versions kept.
const MAX: usize = 20;

/// How long after an update Assistant offers to tell what's new.
const FRESH_MS: u64 = 14 * 24 * 3_600_000;

/// A version and when this install first ran it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Seen {
    pub version: String,
    pub first_ms: u64,
}

#[derive(Default, Deserialize, Serialize)]
struct File {
    #[serde(default)]
    versions: Vec<Seen>,
    /// The last version Assistant was opened on after an update.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opened: Option<String>,
}

fn path() -> Option<PathBuf> {
    crate::app_paths::support_dir()
        .ok()
        .map(|dir| dir.join("assistant/versions.json"))
}

/// The versions run, oldest first.
pub fn read() -> Vec<Seen> {
    path().map(|path| read_at(&path)).unwrap_or_default()
}

fn read_at(path: &Path) -> Vec<Seen> {
    file_at(path).versions
}

fn file_at(path: &Path) -> File {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<File>(&bytes).ok())
        .unwrap_or_default()
}

fn write_at(path: &Path, file: &File) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(file).map_err(std::io::Error::other)?;
    crate::durable::write(path, &json)
}

/// Notes that this build runs, once per new version: a release only.
pub fn record_running() {
    if crate::dev_build::is_dev_build() || super::order(crate::VERSION).is_none() {
        return;
    }
    let Some(path) = path() else {
        return;
    };
    if let Err(error) = record_at(&path, crate::VERSION, crate::desktop::events::now_ms()) {
        tracing::info!(%error, "couldn't note the version run");
    }
}

/// Adds `version` at `now_ms` unless it is the last one run.
fn record_at(path: &Path, version: &str, now_ms: u64) -> std::io::Result<()> {
    let mut file = file_at(path);
    if file
        .versions
        .last()
        .is_some_and(|last| last.version == version)
    {
        return Ok(());
    }
    file.versions.push(Seen {
        version: version.to_owned(),
        first_ms: now_ms,
    });
    let excess = file.versions.len().saturating_sub(MAX);
    file.versions.drain(..excess);
    write_at(path, &file)
}

/// The version run before the last run of `running`.
pub fn previous(seen: &[Seen], running: &str) -> Option<String> {
    let at = seen.iter().rposition(|entry| entry.version == running)?;
    seen[..at]
        .iter()
        .rev()
        .find(|entry| entry.version != running)
        .map(|entry| entry.version.clone())
}

/// When this install updated to `running`, if it ran another version
/// before: the first time `running` ran.
pub fn updated_at(seen: &[Seen], running: &str) -> Option<u64> {
    previous(seen, running)?;
    seen.iter()
        .rev()
        .find(|entry| entry.version == running)
        .map(|entry| entry.first_ms)
}

/// The version this install updated to in the last two weeks and when,
/// if it ran another one before: a release only.
pub fn fresh(now_ms: u64) -> Option<(String, u64)> {
    if crate::dev_build::is_dev_build() || super::order(crate::VERSION).is_none() {
        return None;
    }
    fresh_in(&read(), crate::VERSION, now_ms)
}

fn fresh_in(seen: &[Seen], running: &str, now_ms: u64) -> Option<(String, u64)> {
    let at = updated_at(seen, running)?;
    (now_ms.saturating_sub(at) < FRESH_MS).then(|| (running.to_owned(), at))
}

/// The version the sidebar marks Assistant for: the update [`fresh`] names,
/// while Assistant hasn't been opened on it. A release only; a first
/// install has none.
pub fn unopened(now_ms: u64) -> Option<String> {
    if crate::dev_build::is_dev_build() || super::order(crate::VERSION).is_none() {
        return None;
    }
    let path = path()?;
    unopened_in(&file_at(&path), crate::VERSION, now_ms)
}

fn unopened_in(file: &File, running: &str, now_ms: u64) -> Option<String> {
    let (version, _) = fresh_in(&file.versions, running, now_ms)?;
    (file.opened.as_deref() != Some(version.as_str())).then_some(version)
}

/// Assistant was opened on `version` after its update: its mark is gone
/// for good, across relaunches.
pub fn note_opened(version: &str) {
    let Some(path) = path() else {
        return;
    };
    if let Err(error) = note_opened_at(&path, version) {
        tracing::info!(%error, "couldn't note Assistant opened after the update");
    }
}

fn note_opened_at(path: &Path, version: &str) -> std::io::Result<()> {
    let mut file = file_at(path);
    if file.opened.as_deref() == Some(version) {
        return Ok(());
    }
    file.opened = Some(version.to_owned());
    write_at(path, &file)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hex-versions-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("assistant/versions.json")
    }

    #[test]
    fn a_new_version_is_noted_once_with_when_it_first_ran() {
        let path = scratch("once");
        record_at(&path, "3.3.0-vibe.4", 1_000).unwrap();
        record_at(&path, "3.3.0-vibe.4", 2_000).unwrap();
        record_at(&path, "3.3.0-vibe.6", 3_000).unwrap();
        record_at(&path, "3.3.0-vibe.6", 4_000).unwrap();
        let seen = read_at(&path);
        assert_eq!(
            seen,
            [
                Seen {
                    version: "3.3.0-vibe.4".into(),
                    first_ms: 1_000
                },
                Seen {
                    version: "3.3.0-vibe.6".into(),
                    first_ms: 3_000
                },
            ]
        );
        assert_eq!(
            previous(&seen, "3.3.0-vibe.6").as_deref(),
            Some("3.3.0-vibe.4")
        );
        assert_eq!(updated_at(&seen, "3.3.0-vibe.6"), Some(3_000));
        assert_eq!(previous(&seen, "3.3.0-vibe.4"), None, "the first one run");
        assert_eq!(updated_at(&seen, "3.3.0-vibe.4"), None);
        assert_eq!(previous(&seen, "3.3.0-vibe.9"), None, "never run");
        let day = 24 * 3_600_000;
        assert_eq!(
            fresh_in(&seen, "3.3.0-vibe.6", 3_000 + 13 * day),
            Some(("3.3.0-vibe.6".into(), 3_000))
        );
        assert_eq!(fresh_in(&seen, "3.3.0-vibe.6", 3_000 + 15 * day), None);
        assert_eq!(
            fresh_in(&seen, "3.3.0-vibe.4", 1_500),
            None,
            "a first install"
        );
    }

    /// The sidebar's mark: none on a first install, the update's until
    /// Assistant opens on it, then never again for it, after a relaunch
    /// too; the next update brings it back.
    #[test]
    fn the_mark_lasts_until_assistant_opens_on_the_update() {
        let path = scratch("mark");
        let day = 24 * 3_600_000;
        record_at(&path, "3.3.0-vibe.6", 1_000).unwrap();
        assert_eq!(
            unopened_in(&file_at(&path), "3.3.0-vibe.6", 2_000),
            None,
            "a first install has no mark"
        );
        record_at(&path, "3.3.0-vibe.7", 10_000).unwrap();
        let now = 10_000 + day;
        assert_eq!(
            unopened_in(&file_at(&path), "3.3.0-vibe.7", now).as_deref(),
            Some("3.3.0-vibe.7")
        );
        note_opened_at(&path, "3.3.0-vibe.7").unwrap();
        // Relaunched: the same version runs again, and the file is read anew.
        record_at(&path, "3.3.0-vibe.7", now + 1).unwrap();
        assert_eq!(unopened_in(&file_at(&path), "3.3.0-vibe.7", now + 2), None);
        assert_eq!(read_at(&path).len(), 2, "the versions are kept beside it");
        // Two weeks on, the update isn't fresh: no mark either way.
        let file = File {
            versions: read_at(&path),
            opened: None,
        };
        assert_eq!(unopened_in(&file, "3.3.0-vibe.7", 10_000 + 15 * day), None);
        record_at(&path, "3.3.0-vibe.8", now + 3).unwrap();
        assert_eq!(
            unopened_in(&file_at(&path), "3.3.0-vibe.8", now + 4).as_deref(),
            Some("3.3.0-vibe.8"),
            "the next update marks again"
        );
    }

    #[test]
    fn the_list_keeps_the_newest_twenty() {
        let path = scratch("twenty");
        for n in 0..25u64 {
            record_at(&path, &format!("3.3.0-vibe.{n}"), n).unwrap();
        }
        let seen = read_at(&path);
        assert_eq!(seen.len(), MAX);
        assert_eq!(seen[0].version, "3.3.0-vibe.5");
        assert_eq!(seen[MAX - 1].version, "3.3.0-vibe.24");
    }

    #[test]
    fn a_missing_or_broken_file_is_an_empty_list() {
        let path = scratch("broken");
        assert!(read_at(&path).is_empty());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "not json").unwrap();
        assert!(read_at(&path).is_empty());
        record_at(&path, "3.3.0-vibe.6", 5).unwrap();
        assert_eq!(read_at(&path).len(), 1, "a broken file starts over");
    }

    #[test]
    fn going_back_and_forth_reads_the_one_before() {
        let seen = [
            ("3.3.0-vibe.5", 1),
            ("3.3.0-vibe.6", 2),
            ("3.3.0-vibe.5", 3),
        ]
        .map(|(version, first_ms)| Seen {
            version: version.into(),
            first_ms,
        });
        assert_eq!(
            previous(&seen, "3.3.0-vibe.5").as_deref(),
            Some("3.3.0-vibe.6")
        );
        assert_eq!(updated_at(&seen, "3.3.0-vibe.5"), Some(3));
    }
}
