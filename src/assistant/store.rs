//! Assistant's conversations. While Assistant › Settings keeps them they
//! live in `<support>/assistant/conversations.json`, readable only by this
//! account; otherwise only in memory until VibeHex quits. They hold
//! questions about VibeHex and their answers, nothing else.

use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

/// The most conversations kept; the least recently used go first.
const MAX_CONVERSATIONS: usize = 50;

/// The longest name a first question gives a conversation.
const NAME_CHARS: usize = 48;

/// One question and its answer.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Turn {
    pub question: String,
    /// What the pane shows, without the `Sources:` line.
    pub answer: String,
    /// The guide articles it named, as `labs/meetings`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// The model that wrote it, as the pane names it.
    pub model: String,
    pub seconds: f32,
    /// Stopped before it was done: `answer` is what came.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
    /// Why there is no answer, when the model failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_ms: u64,
}

/// One conversation.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Conversation {
    pub id: u32,
    /// From its first question.
    pub name: String,
    pub turns: Vec<Turn>,
    /// Its model, when it isn't Assistant › Settings' one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    pub created_ms: u64,
}

impl Conversation {
    /// When it was last asked or answered.
    pub fn updated_ms(&self) -> u64 {
        self.turns
            .last()
            .map_or(self.created_ms, |turn| turn.created_ms)
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
struct File {
    conversations: Vec<Conversation>,
}

/// The conversations, and where they are saved while they are kept.
#[derive(Debug, Default)]
pub struct Store {
    path: Option<PathBuf>,
    conversations: Vec<Conversation>,
}

impl Store {
    /// The saved conversations when `keep`, else none and nothing saved.
    pub fn open(keep: bool) -> Self {
        match crate::app_paths::support_dir() {
            Ok(dir) => Self::at(&dir.join("assistant/conversations.json"), keep),
            Err(error) => {
                tracing::warn!("Assistant can't keep conversations: {error:#}");
                Self::default()
            }
        }
    }

    fn at(path: &Path, keep: bool) -> Self {
        let conversations = if keep { read(path) } else { Vec::new() };
        Self {
            path: keep.then(|| path.to_owned()),
            conversations,
        }
    }

    /// Every conversation, the most recently used last.
    pub fn conversations(&self) -> &[Conversation] {
        &self.conversations
    }

    pub fn get(&self, id: u32) -> Option<&Conversation> {
        self.conversations
            .iter()
            .find(|conversation| conversation.id == id)
    }

    /// Starts a conversation named by its first question, and returns its id.
    pub fn start(&mut self, question: &str, engine: Option<String>, effort: Option<String>) -> u32 {
        let id = self.conversations.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        self.conversations.push(Conversation {
            id,
            name: name_of(question),
            turns: Vec::new(),
            engine,
            effort,
            created_ms: now_ms(),
        });
        if self.conversations.len() > MAX_CONVERSATIONS {
            self.conversations.sort_by_key(Conversation::updated_ms);
            let extra = self.conversations.len() - MAX_CONVERSATIONS;
            self.conversations.drain(..extra);
        }
        self.save();
        id
    }

    /// Adds a finished turn to conversation `id`.
    pub fn push(&mut self, id: u32, turn: Turn) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.turns.push(turn);
            self.save();
        }
    }

    /// The model and effort conversation `id` asks with next.
    pub fn choose(&mut self, id: u32, engine: Option<String>, effort: Option<String>) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.engine = engine;
            conversation.effort = effort;
            self.save();
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, id: u32) {
        self.conversations
            .retain(|conversation| conversation.id != id);
        self.save();
    }

    /// Starts or stops keeping conversations; stopping deletes the file and
    /// keeps them in memory.
    pub fn keep(&mut self, keep: bool) {
        match (keep, self.path.take()) {
            (true, Some(path)) => self.path = Some(path),
            (true, None) => {
                if let Ok(dir) = crate::app_paths::support_dir() {
                    self.path = Some(dir.join("assistant/conversations.json"));
                    self.save();
                }
            }
            (false, Some(path)) => {
                if let Err(error) = std::fs::remove_file(&path)
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!("couldn't delete Assistant's conversations: {error}");
                }
            }
            (false, None) => {}
        }
    }

    fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Err(error) = write(path, &self.conversations) {
            tracing::warn!("couldn't save Assistant's conversations: {error:#}");
        }
    }
}

fn read(path: &Path) -> Vec<Conversation> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<File>(&bytes)
            .map(|file| file.conversations)
            .unwrap_or_else(|error| {
                tracing::warn!("couldn't read Assistant's conversations: {error}");
                Vec::new()
            }),
        Err(_) => Vec::new(),
    }
}

fn write(path: &Path, conversations: &[Conversation]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(&File {
        conversations: conversations.to_vec(),
    })?;
    crate::durable::write(path, &json).wrap_err("couldn't write the file")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// A conversation's name: its first question, cut at a word.
fn name_of(question: &str) -> String {
    let question = question.split_whitespace().collect::<Vec<_>>().join(" ");
    if question.chars().count() <= NAME_CHARS {
        return question;
    }
    let cut: String = question.chars().take(NAME_CHARS).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', '.', ';', ':']))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hex-assistant-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn turn(question: &str) -> Turn {
        Turn {
            question: question.into(),
            answer: "Yes.".into(),
            sources: vec!["labs/meetings".into()],
            model: "GPT-5.5".into(),
            created_ms: now_ms(),
            ..Turn::default()
        }
    }

    #[test]
    fn kept_conversations_come_back_and_only_this_account_reads_them() {
        let dir = scratch("kept");
        let path = dir.join("assistant/conversations.json");
        let mut store = Store::at(&path, true);
        let id = store.start(
            "Can Zoom calls record by themselves?",
            Some("codex".into()),
            None,
        );
        store.push(id, turn("Can Zoom calls record by themselves?"));
        let again = Store::at(&path, true);
        assert_eq!(again.conversations(), store.conversations());
        assert_eq!(again.get(id).unwrap().turns[0].sources, ["labs/meetings"]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn not_keeping_writes_nothing_and_stopping_deletes_the_file() {
        let dir = scratch("memory");
        let path = dir.join("assistant/conversations.json");
        let mut memory = Store::at(&path, false);
        memory.start("question", None, None);
        assert!(!path.exists());
        assert_eq!(memory.conversations().len(), 1);
        let mut kept = Store::at(&path, true);
        kept.start("question", None, None);
        assert!(path.exists());
        kept.keep(false);
        assert!(!path.exists());
        assert_eq!(kept.conversations().len(), 1);
    }

    #[test]
    fn the_oldest_go_past_fifty() {
        let mut store = Store::default();
        for index in 0..55 {
            store.start(&format!("question {index}"), None, None);
        }
        assert_eq!(store.conversations().len(), MAX_CONVERSATIONS);
        assert!(store.conversations().iter().all(|c| c.name != "question 0"));
    }

    #[test]
    fn a_long_first_question_is_cut_at_a_word() {
        assert_eq!(
            name_of("  Can   Zoom calls record? "),
            "Can Zoom calls record?"
        );
        let name =
            name_of("Хочу, чтобы созвоны в Zoom записывались сами, без вопросов. Так можно?");
        assert_eq!(name, "Хочу, чтобы созвоны в Zoom записывались сами…");
    }
}
