//! Files' own list: one owner-only JSON record per file in
//! `<support>/files/`, holding where the file is (never a copy of it), its
//! timed transcript, and every version of its text, plus `recent.json`, the
//! last requests typed. Nothing here expires; deleting a record never
//! touches the file it came from.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde::{Deserialize, Serialize};

/// The most recent requests kept for the Recent menu.
pub const RECENT_LIMIT: usize = 10;
const RECENT_FILE: &str = "recent.json";

/// Every read-modify-write of a record holds this, so the worker adding a
/// version and the window editing one never undo each other.
static LOCK: Mutex<()> = Mutex::new(());

/// One timed stretch of the transcript, about a sentence.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Segment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    /// When each of `text`'s words starts, when the model timed them (see
    /// `meeting::TranscriptEntry::words`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    /// What a transcription model heard.
    #[default]
    Original,
    /// AI cleanup of another version.
    Cleanup,
    /// The person's own request applied to another version.
    Request,
}

/// One tab of a file's text.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Version {
    pub id: u32,
    pub kind: VersionKind,
    /// The tab's name: Original, Cleanup, Summary, Cleanup (Sonnet 4.6)…
    pub name: String,
    pub text: String,
    /// An Original's timed transcript, which `text` joins.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<Segment>,
    /// The model that made it.
    pub model: String,
    /// The version it was made from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<u32>,
    /// What a request asked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// The built-in engine's context window, in thousands of tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_k: Option<u32>,
    /// A text longer than the window was worked in three steps over this
    /// many parts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<usize>,
    /// How long making it took.
    pub seconds: u64,
    pub created_ms: u64,
    /// Changed by hand since it was made.
    pub edited: bool,
    /// The three-steps note was closed.
    pub note_closed: bool,
    /// Why some of it kept its input text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// A request stopped before it finished: `text` is what came.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
    /// A request's session: the questions asked after the first, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub follow_ups: Vec<FollowUp>,
    /// Where the session's next question runs, in `files.request_engine`'s
    /// form; `None` follows Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// How hard a Codex model thinks for the session's next question;
    /// `None` follows Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// One question asked later in a request's session, and its answer.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct FollowUp {
    pub question: String,
    pub answer: String,
    /// The model that answered.
    pub model: String,
    /// How long the answer took.
    pub seconds: u64,
    pub created_ms: u64,
    /// Stopped before it finished: `answer` is what came.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
}

/// One file added to Files.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct FileRecord {
    pub id: String,
    /// The file's name when it was added.
    pub name: String,
    /// Where the file is; Transcribe again reads it there.
    pub path: PathBuf,
    pub added_ms: u64,
    pub duration_ms: u64,
    pub versions: Vec<Version>,
    /// Why it could not be transcribed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileRecord {
    pub fn new(path: &Path, now_ms: u64) -> Self {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        Self {
            id: format!("{now_ms}-{:08x}", random_suffix()),
            name,
            path: path.to_path_buf(),
            added_ms: now_ms,
            ..Self::default()
        }
    }

    pub fn version(&self, id: u32) -> Option<&Version> {
        self.versions.iter().find(|version| version.id == id)
    }

    pub fn version_mut(&mut self, id: u32) -> Option<&mut Version> {
        self.versions.iter_mut().find(|version| version.id == id)
    }

    /// Adds `version` with the next id and returns it.
    pub fn push(&mut self, mut version: Version) -> u32 {
        version.id = self
            .versions
            .iter()
            .map(|v| v.id)
            .max()
            .map_or(1, |id| id + 1);
        let id = version.id;
        self.versions.push(version);
        id
    }

    /// A name for a new version that no tab has yet: `name`, else
    /// `name 2`, `name 3`…
    pub fn unique_name(&self, name: &str) -> String {
        let taken = |candidate: &str| self.versions.iter().any(|v| v.name == candidate);
        if !taken(name) {
            return name.into();
        }
        (2..)
            .map(|n| format!("{name} {n}"))
            .find(|candidate| !taken(candidate))
            .unwrap_or_else(|| name.into())
    }
}

fn random_suffix() -> u32 {
    let mut bytes = [0; 4];
    let _ = getrandom::fill(&mut bytes);
    u32::from_le_bytes(bytes)
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

/// `<support>/files`, created owner-only.
pub fn dir() -> Result<PathBuf> {
    #[cfg(test)]
    if let Some(dir) = tests::DIR.with(|dir| dir.borrow().clone()) {
        fs::create_dir_all(&dir)?;
        return Ok(dir);
    }
    let dir = crate::app_paths::support_dir()?.join("files");
    fs::create_dir_all(&dir).wrap_err("couldn't create the Files folder")?;
    restrict(&dir)?;
    Ok(dir)
}

fn record_path(dir: &Path, id: &str) -> Result<PathBuf> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(eyre!("not a Files record: {id}"));
    }
    Ok(dir.join(format!("{id}.json")))
}

/// Every record, newest first. Unreadable records are skipped and logged.
pub fn list() -> Vec<FileRecord> {
    match dir() {
        Ok(dir) => list_in(&dir),
        Err(_) => Vec::new(),
    }
}

/// Every record in `dir`, newest first, read without creating anything.
pub fn list_in(dir: &Path) -> Vec<FileRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut records: Vec<FileRecord> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "json")
                && path.file_name().is_some_and(|name| name != RECENT_FILE)
        })
        .filter_map(|path| match read(&path) {
            Ok(record) => Some(record),
            Err(error) => {
                tracing::warn!(path = %path.display(), "skipped a Files record: {error:#}");
                None
            }
        })
        .collect();
    records.sort_by(|a, b| b.added_ms.cmp(&a.added_ms).then(b.id.cmp(&a.id)));
    records
}

fn read(path: &Path) -> Result<FileRecord> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn load(id: &str) -> Result<FileRecord> {
    read(&record_path(&dir()?, id)?)
}

pub fn save(record: &FileRecord) -> Result<()> {
    let _held = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    write(record)
}

fn write(record: &FileRecord) -> Result<()> {
    let path = record_path(&dir()?, &record.id)?;
    let json = serde_json::to_vec_pretty(record)?;
    crate::durable::write(&path, &json).wrap_err("couldn't save the Files record")?;
    restrict(&path)?;
    Ok(())
}

/// Reads record `id`, changes it, and writes it back, all under the lock.
/// `None` when the record is gone (deleted meanwhile).
pub fn update<T>(id: &str, change: impl FnOnce(&mut FileRecord) -> T) -> Result<Option<T>> {
    let _held = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = record_path(&dir()?, id)?;
    let mut record = match read(&path) {
        Ok(record) => record,
        Err(error)
            if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::NotFound) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let result = change(&mut record);
    write(&record)?;
    Ok(Some(result))
}

/// Forgets record `id`; the file itself stays where it is.
pub fn delete(id: &str) -> Result<()> {
    let _held = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match fs::remove_file(record_path(&dir()?, id)?) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// The last requests typed, newest first.
pub fn recent() -> Vec<String> {
    dir()
        .ok()
        .and_then(|dir| fs::read(dir.join(RECENT_FILE)).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Puts `prompt` first among the recent requests, once.
pub fn remember(prompt: &str) -> Result<()> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Ok(());
    }
    let mut recent = recent();
    recent.retain(|kept| kept != prompt);
    recent.insert(0, prompt.into());
    recent.truncate(RECENT_LIMIT);
    write_recent(&recent)
}

pub fn clear_recent() -> Result<()> {
    write_recent(&[])
}

fn write_recent(recent: &[String]) -> Result<()> {
    let path = dir()?.join(RECENT_FILE);
    crate::durable::write(&path, &serde_json::to_vec_pretty(recent)?)?;
    restrict(&path)
}

#[cfg(unix)]
fn restrict(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if path.is_dir() { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        pub(crate) static DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    }

    /// Points this thread's store at a fresh temporary folder.
    pub(crate) fn temporary_store(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hex-files-{name}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        let _ = fs::remove_dir_all(&dir);
        DIR.with(|cell| *cell.borrow_mut() = Some(dir.clone()));
        dir
    }

    #[test]
    fn records_round_trip_newest_first_and_delete_leaves_the_file() {
        let dir = temporary_store("round-trip");
        let source = dir.join("talk.m4a");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&source, b"audio").unwrap();
        let mut older = FileRecord::new(&source, 1_000);
        older.push(Version {
            name: "Original".into(),
            text: "hello".into(),
            ..Version::default()
        });
        let newer = FileRecord::new(&source, 2_000);
        save(&older).unwrap();
        save(&newer).unwrap();
        let listed = list();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, newer.id);
        assert_eq!(listed[1].versions[0].id, 1);
        assert_eq!(listed[1].name, "talk.m4a");
        delete(&older.id).unwrap();
        assert_eq!(list().len(), 1);
        assert!(source.exists(), "deleting a record keeps the file");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn update_changes_a_record_and_skips_a_deleted_one() {
        let dir = temporary_store("update");
        let record = FileRecord::new(Path::new("/tmp/a.mp3"), 5);
        save(&record).unwrap();
        let added = update(&record.id, |record| {
            record.push(Version {
                name: "Cleanup".into(),
                ..Version::default()
            })
        })
        .unwrap();
        assert_eq!(added, Some(1));
        assert_eq!(load(&record.id).unwrap().versions.len(), 1);
        delete(&record.id).unwrap();
        assert_eq!(update(&record.id, |_| ()).unwrap(), None);
        assert!(record_path(&dir, "../x").is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recent_keeps_ten_unique_requests_newest_first() {
        let dir = temporary_store("recent");
        for n in 0..12 {
            remember(&format!("request {n}")).unwrap();
        }
        remember("request 5").unwrap();
        let recent = recent();
        assert_eq!(recent.len(), RECENT_LIMIT);
        assert_eq!(recent[0], "request 5");
        assert_eq!(recent.iter().filter(|r| *r == "request 5").count(), 1);
        clear_recent().unwrap();
        assert!(super::recent().is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unique_names_count_up() {
        let mut record = FileRecord::default();
        record.push(Version {
            name: "Summary".into(),
            ..Version::default()
        });
        assert_eq!(record.unique_name("Summary"), "Summary 2");
        assert_eq!(record.unique_name("Cleanup"), "Cleanup");
    }
}
