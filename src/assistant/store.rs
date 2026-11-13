//! Assistant's conversations. While Assistant › Settings keeps them they
//! live in `<support>/assistant/conversations.json`, readable only by this
//! account; otherwise only in memory until VibeHex quits. They hold
//! questions about VibeHex and their answers, and the guided setups
//! finished (`setup::past`), nothing else.

use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

use super::agent::trace::Did;
use super::changes::{Card, Refused};
use super::recordings::{Ref, Step};
use super::setup::past::{self, Setup};
use super::setup::since::Candidate;
use super::setup::{Answer, Ask};
use super::web::Web;

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
    /// How it searched the person's recordings; empty for a guide answer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
    /// What an agent turn read before it answered, in order
    /// (`agent::trace`); empty for a turn the router read for.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trace: Vec<Did>,
    /// The places in the recordings its `[n]` cite.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<Ref>,
    /// The setting changes it suggested, and what the person did.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<Card>,
    /// Change lines the app refused; never offered as actions.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub refused: Vec<Refused>,
    /// Setup's questions it asks, with option chips.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<Ask>,
    /// The answers to the turn before's questions: this turn has no
    /// question of its own, and shows no bubble.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub answers: Vec<Answer>,
    /// Setup's paragraph of what was set when it began, above its
    /// questions, in the window's language.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub now: String,
    /// Setup's follow-up questions for later rounds.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub more: Vec<Ask>,
    /// A follow-up round a model wrote: asking again asks it again.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub follow: bool,
    /// A returning setup's paragraph of what came since the last one, in
    /// the window's language (`setup::since`).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub since: String,
    /// The same, as the model reads it: the last setup, the releases
    /// since, and the use's aggregates, never a recording's text.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub facts: String,
    /// A returning setup's round one, folded to one row until the person
    /// says something changed.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub folded: bool,
    /// Round one's own words, as written in the field under its questions
    /// (`setup::about`): what they do and where they dictate most.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub about: String,
    /// The terms a returning setup's use shows Vocabulary lacks, which
    /// its plan offers (`setup::since::candidates`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub terms: Vec<Candidate>,
    /// A preset's step ("Set me up like Denis"): `settings` its plan,
    /// `shortcuts` the question about its shortcuts, `shortcuts-plan` them
    /// as a plan, `done` once the person kept their own.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub preset: String,
    /// What it searched on the web and the pages it cites; empty when it
    /// never went online.
    #[serde(skip_serializing_if = "Web::is_empty")]
    pub web: Web,
    /// What its model was told of this install (`facts::text::knew`):
    /// settings' effective values and this computer, never a recording.
    /// Kept in memory for the session only, for "What Assistant knew".
    #[serde(skip)]
    pub knew: String,
}

impl Turn {
    /// Its answer was written from recordings it found, so it may quote
    /// them; a search that found nothing gave it nothing of theirs.
    pub fn read_recordings(&self) -> bool {
        !self.refs.is_empty()
            || self.steps.iter().any(|step| match *step {
                Step::Meetings { found, .. }
                | Step::Files { found, .. }
                | Step::Dictations { found, .. } => found > 0,
                Step::Read { passages, .. } => passages > 0,
                Step::Looked {
                    dictations,
                    meetings,
                } => dictations + meetings > 0,
                Step::Guide { .. } => false,
            })
            || self.trace.iter().any(Did::personal)
    }
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
    /// Its Web toggle, when it isn't Assistant › Settings' one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web: Option<bool>,
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    setups: Vec<Setup>,
}

/// The conversations, and where they are saved while they are kept.
#[derive(Debug, Default)]
pub struct Store {
    path: Option<PathBuf>,
    conversations: Vec<Conversation>,
    /// The guided setups finished, oldest first.
    setups: Vec<Setup>,
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
        let file = if keep { read(path) } else { File::default() };
        Self {
            path: keep.then(|| path.to_owned()),
            conversations: file.conversations,
            setups: file.setups,
        }
    }

    /// Every conversation, the most recently used last.
    pub fn conversations(&self) -> &[Conversation] {
        &self.conversations
    }

    /// The guided setups finished, oldest first.
    pub fn setups(&self) -> &[Setup] {
        &self.setups
    }

    /// Keeps `setup`: in place of the same setup kept before (a later
    /// round, or its plan decided), else as the newest.
    pub fn finish(&mut self, setup: Setup) {
        past::keep(&mut self.setups, setup);
        self.save();
    }

    /// A scene's hand on every conversation, to lengthen what it shows.
    /// Nothing is saved.
    pub fn each(&mut self, edit: impl FnMut(&mut Conversation)) {
        self.conversations.iter_mut().for_each(edit);
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
            web: None,
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
    /// Dates conversation `id` at `created_ms`, as a scene's fixed clock.
    pub fn date(&mut self, id: u32, created_ms: u64) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.created_ms = created_ms;
        }
    }

    pub fn push(&mut self, id: u32, turn: Turn) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.turns.push(turn);
            self.save();
        }
    }

    /// Replaces card `card` of turn `turn` in conversation `id`, as the
    /// person decided it.
    pub fn set_card(&mut self, id: u32, turn: usize, card: usize, value: Card) {
        let slot = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|conversation| conversation.turns.get_mut(turn))
            .and_then(|turn| turn.cards.get_mut(card));
        if let Some(slot) = slot {
            *slot = value;
            self.save();
        }
    }

    /// Unfolds a returning setup's round one, turn `turn` of conversation
    /// `id`.
    pub fn unfold(&mut self, id: u32, turn: usize) {
        let slot = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|conversation| conversation.turns.get_mut(turn));
        if let Some(slot) = slot {
            slot.folded = false;
            self.save();
        }
    }

    /// Round one's own words in turn `turn` of conversation `id`, as typed:
    /// a draft, held in memory and saved with the round's next change (its
    /// Continue), so typing writes no file.
    pub fn write_about(&mut self, id: u32, turn: usize, text: &str) {
        let slot = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|conversation| conversation.turns.get_mut(turn));
        if let Some(slot) = slot {
            slot.about = crate::assistant::setup::about::cap(text).to_owned();
        }
    }

    /// Replaces question `ask` of turn `turn` in conversation `id`, as the
    /// person answered it.
    pub fn set_ask(&mut self, id: u32, turn: usize, ask: usize, value: Ask) {
        let slot = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|conversation| conversation.turns.get_mut(turn))
            .and_then(|turn| turn.asks.get_mut(ask));
        if let Some(slot) = slot {
            *slot = value;
            self.save();
        }
    }

    /// Adds `asks` to turn `turn` of conversation `id`.
    pub fn push_asks(&mut self, id: u32, turn: usize, asks: Vec<Ask>) {
        let slot = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|conversation| conversation.turns.get_mut(turn));
        if let Some(slot) = slot {
            slot.asks.extend(asks);
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

    /// Whether conversation `id` may search the web next; `None` follows
    /// Assistant › Settings.
    pub fn set_web(&mut self, id: u32, web: Option<bool>) {
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.web = web;
            self.save();
        }
    }

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
        if let Err(error) = write(path, &self.conversations, &self.setups) {
            tracing::warn!("couldn't save Assistant's conversations: {error:#}");
        }
    }
}

fn read(path: &Path) -> File {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<File>(&bytes).unwrap_or_else(|error| {
            tracing::warn!("couldn't read Assistant's conversations: {error}");
            File::default()
        }),
        Err(_) => File::default(),
    }
}

fn write(path: &Path, conversations: &[Conversation], setups: &[Setup]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_vec_pretty(&File {
        conversations: conversations.to_vec(),
        setups: setups.to_vec(),
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
        store.finish(Setup {
            at_ms: 7,
            ..Setup::default()
        });
        let again = Store::at(&path, true);
        assert_eq!(again.conversations(), store.conversations());
        assert_eq!(again.setups(), store.setups());
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

    /// A file written before cards existed still reads, and a card's
    /// decision is kept.
    #[test]
    fn old_files_load_and_a_decided_card_is_kept() {
        let old = r#"{"conversations":[{"id":1,"name":"q","turns":[{"question":"q","answer":"a","model":"m","seconds":1.0,"created_ms":5}],"created_ms":5}]}"#;
        let file: File = serde_json::from_str(old).unwrap();
        assert!(file.conversations[0].turns[0].cards.is_empty());
        let mut store = Store::default();
        let id = store.start("q", None, None);
        store.push(
            id,
            Turn {
                cards: vec![Card::default()],
                ..turn("q")
            },
        );
        let declined = Card {
            decision: crate::assistant::changes::Decision::Declined,
            ..Card::default()
        };
        store.set_card(id, 0, 0, declined.clone());
        assert_eq!(store.get(id).unwrap().turns[0].cards, [declined]);
    }

    /// A web answer's searches and sources, and a conversation's Web
    /// toggle, come back; a file from before them still reads.
    #[test]
    fn a_web_answer_and_the_web_toggle_are_kept() {
        let dir = scratch("web");
        let path = dir.join("assistant/conversations.json");
        let mut store = Store::at(&path, true);
        let id = store.start("LM Studio?", Some("codex".into()), None);
        store.set_web(id, Some(false));
        let web = Web {
            guide: vec!["topics/ai-cleanup".into()],
            searches: vec!["LM Studio start local server".into()],
            pages: vec!["https://lmstudio.ai/docs/app".into()],
            sources: vec!["https://lmstudio.ai/docs/developer/core/server".into()],
            going: Vec::new(),
            held_back: false,
        };
        store.push(
            id,
            Turn {
                web: web.clone(),
                ..turn("LM Studio?")
            },
        );
        store.push(id, turn("And Ollama?"));
        let again = Store::at(&path, true);
        let conversation = again.get(id).unwrap();
        assert_eq!(conversation.web, Some(false));
        assert_eq!(conversation.turns[0].web, web);
        assert!(conversation.turns[1].web.is_empty());
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            saved.matches("\"web\"").count(),
            2,
            "an empty one is left out"
        );
        let old = r#"{"conversations":[{"id":1,"name":"q","turns":[{"question":"q","answer":"a"}],"created_ms":5}]}"#;
        let file: File = serde_json::from_str(old).unwrap();
        assert_eq!(file.conversations[0].web, None);
        assert!(file.conversations[0].turns[0].web.is_empty());
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
