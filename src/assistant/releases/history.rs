//! The VibeHex versions this install has run, each with when it first ran,
//! so "what's new" means what came since the one before:
//! `<support>/assistant/versions.json`, the newest 20. Only a release
//! records itself; a development build leaves the list as it was.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The most versions kept.
const MAX: usize = 20;

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
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<File>(&bytes).ok())
        .map(|file| file.versions)
        .unwrap_or_default()
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
    let mut versions = read_at(path);
    if versions.last().is_some_and(|last| last.version == version) {
        return Ok(());
    }
    versions.push(Seen {
        version: version.to_owned(),
        first_ms: now_ms,
    });
    let excess = versions.len().saturating_sub(MAX);
    versions.drain(..excess);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(&File { versions }).map_err(std::io::Error::other)?;
    crate::durable::write(path, &json)
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
