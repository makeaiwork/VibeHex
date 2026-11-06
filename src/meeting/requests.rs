//! Questions about a meeting, kept in sessions as Files keeps them: each
//! session is a conversation about the meeting's transcript, its first
//! question named it, and every answer keeps the model that wrote it. They
//! live beside the meeting as `requests.json`, so trashing the meeting takes
//! them too. Files' worker answers them (`files::requests::meeting`).

use std::path::PathBuf;
use std::sync::Mutex;

use color_eyre::eyre::{Result, WrapErr, eyre};
use serde::{Deserialize, Serialize};

use crate::files::store::FollowUp;

/// Every read-modify-write of a meeting's sessions holds this.
static LOCK: Mutex<()> = Mutex::new(());

/// One conversation about a meeting.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Session {
    pub id: u32,
    /// From its first question, as Files names a typed request.
    pub name: String,
    /// Every question and its answer, the first one first.
    pub turns: Vec<FollowUp>,
    /// Where its next question runs, in `files.request_engine`'s form;
    /// `None` follows Files › Settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// How hard a Codex model thinks for its next question.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub created_ms: u64,
}

impl Session {
    /// When it was last asked or answered.
    pub fn updated_ms(&self) -> u64 {
        self.turns
            .last()
            .map_or(self.created_ms, |turn| turn.created_ms)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
struct Sessions {
    sessions: Vec<Session>,
}

/// The worker's record name for meeting `id`'s questions.
pub fn record(id: &str) -> String {
    format!("meeting:{id}")
}

/// The meeting a worker record names, if it names one.
pub fn meeting_of(record: &str) -> Option<&str> {
    record.strip_prefix("meeting:")
}

fn path(id: &str) -> Result<PathBuf> {
    let valid = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(eyre!("That meeting id isn't valid."));
    }
    Ok(super::root()?.join(id).join("requests.json"))
}

fn read(id: &str) -> Result<Vec<Session>> {
    let path = path(id)?;
    match std::fs::read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice::<Sessions>(&bytes)
            .wrap_err("couldn't read the meeting's requests")?
            .sessions),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// Meeting `id`'s sessions, oldest first; none when it has none or they
/// can't be read.
pub fn load(id: &str) -> Vec<Session> {
    let _held = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    read(id).unwrap_or_else(|error| {
        tracing::warn!(
            meeting = id,
            "couldn't read the meeting's requests: {error:#}"
        );
        Vec::new()
    })
}

/// Reads meeting `id`'s sessions, changes them, and writes them back.
pub fn update<T>(id: &str, change: impl FnOnce(&mut Vec<Session>) -> T) -> Result<T> {
    let _held = LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let path = path(id)?;
    if !path.parent().is_some_and(std::path::Path::is_dir) {
        return Err(eyre!("That meeting is gone."));
    }
    let mut sessions = read(id)?;
    let made = change(&mut sessions);
    let json = serde_json::to_vec_pretty(&Sessions { sessions })?;
    crate::durable::write(&path, &json).wrap_err("couldn't save the meeting's requests")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(made)
}

/// Starts a session named `name` and returns its id.
pub fn start(id: &str, name: &str, engine: Option<String>, effort: Option<String>) -> Result<u32> {
    update(id, |sessions| {
        let next = sessions.iter().map(|session| session.id).max().unwrap_or(0) + 1;
        sessions.push(Session {
            id: next,
            name: name.trim().to_owned(),
            turns: Vec::new(),
            engine,
            effort,
            created_ms: crate::files::store::now_ms(),
        });
        next
    })
}

/// Removes session `session` when nothing was answered in it.
pub fn drop_if_empty(id: &str, session: u32) -> Result<()> {
    update(id, |sessions| {
        sessions.retain(|kept| kept.id != session || !kept.turns.is_empty());
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_names_its_meeting_and_ids_stay_in_the_meeting_folder() {
        assert_eq!(record("1700-42"), "meeting:1700-42");
        assert_eq!(meeting_of("meeting:1700-42"), Some("1700-42"));
        assert_eq!(meeting_of("file-1"), None);
        assert!(path("../x").is_err());
        assert!(path("").is_err());
    }

    #[test]
    fn sessions_read_back_as_written() {
        let json = r#"{"sessions":[{"id":2,"name":"Who waits?","turns":[{"question":"Who waits?","answer":"Design.","model":"GPT","seconds":3,"created_ms":20}],"created_ms":10}]}"#;
        let sessions: Sessions = serde_json::from_str(json).unwrap();
        let session = &sessions.sessions[0];
        assert_eq!(session.turns[0].answer, "Design.");
        assert_eq!(session.updated_ms(), 20);
        assert_eq!(session.engine, None);
    }
}
