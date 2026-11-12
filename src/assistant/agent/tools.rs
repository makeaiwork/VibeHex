//! What a model may ask to read, and the reading. Every tool only reads:
//! the guide, the catalog's lines, the state of this install, the cleanup
//! instructions and Vocabulary (while the person's settings are sent), the
//! release notes, and, with the person's leave, their recordings through
//! `recordings::read`. None changes anything; a change is still a card the
//! person presses.
//!
//! One [`Desk`] serves a turn. It holds what the turn may read
//! ([`Access`]), the excerpts read so far under one numbering
//! (`recordings::Shelf`), how much it has read against the turn's budget,
//! and what of the person's own it has read ([`Desk::personal`]), which
//! is what keeps the rest of the turn off the web.

use super::Limits;
use super::access::{Access, CLOSED, Consent, Live};
use super::protocol::Call;
use super::table::TOOLS;
use super::trace::{Did, Why};
use crate::assistant::changes::{self, catalog, parts};
use crate::assistant::intent::Kinds;
use crate::assistant::recordings::read::{self, Asked, Listed, Stretch};
use crate::assistant::recordings::{Allowed, Budget, Excerpt, HEARD, Shelf, Sources, Step};
use crate::assistant::{facts, guide, personal, prompt, releases, retrieve};
use crate::control::settings;
use crate::dictation::vocabulary::VocabularyEntry;

const SPENT: &str = "Not read: this answer has read as much as it may. Answer now from what \
is above.";

/// One call's result, and what the turn's trace keeps of it.
#[derive(Debug)]
pub struct Ran {
    pub text: String,
    pub did: Vec<Did>,
}

type Out = Result<(String, Did), (Why, String)>;

fn failed(text: impl Into<String>) -> (Why, String) {
    (Why::Failed, text.into())
}

/// What of the person's own a turn has read: any of it keeps the rest of
/// the turn, and of the conversation, off the web.
#[derive(Clone, Copy, Debug, Default)]
pub struct Read {
    /// A list of meetings or files that named any.
    pub listed: bool,
    /// The cleanup instructions.
    pub instructions: bool,
    /// Vocabulary entries, of how many.
    pub vocabulary: Option<(usize, usize)>,
    /// What the use shows.
    pub usage: bool,
    /// Release notes.
    pub releases: bool,
    /// The recordings were asked for and may not be read.
    pub closed: bool,
}

/// A turn's reading.
pub struct Desk<'a> {
    context: Option<&'a changes::Context>,
    access: Access,
    shelf: Shelf,
    now_ms: u64,
    limits: Limits,
    /// Characters of results so far.
    spent: usize,
    /// Calls run so far.
    calls: usize,
    /// The guide articles read so far, the app's first.
    articles: Vec<&'static str>,
    read: Read,
}

impl<'a> Desk<'a> {
    pub fn new(
        context: Option<&'a changes::Context>,
        access: Access,
        limits: Limits,
        now_ms: u64,
    ) -> Self {
        Self {
            context,
            access,
            shelf: Shelf::default(),
            now_ms,
            limits,
            spent: 0,
            calls: 0,
            articles: Vec::new(),
            read: Read::default(),
        }
    }

    /// The articles the app laid out before the first request.
    pub fn laid_out(&mut self, ids: &[&'static str]) {
        self.articles.extend(ids);
    }

    pub fn articles(&self) -> &[&'static str] {
        &self.articles
    }

    pub fn shelf(&self) -> &Shelf {
        &self.shelf
    }

    /// Everything read of the recordings, as the answer's citations
    /// resolve it.
    pub fn into_found(self) -> crate::assistant::recordings::Found {
        self.shelf.into_found()
    }

    pub fn read(&self) -> Read {
        self.read
    }

    pub fn calls(&self) -> usize {
        self.calls
    }

    /// The turn has read as much as it may.
    pub fn spent(&self) -> bool {
        self.spent >= self.limits.turn_chars || self.calls >= self.limits.turn_calls
    }

    /// The turn has read something of the person's own: recordings, what
    /// their use shows, their cleanup instructions, Vocabulary entries.
    pub fn personal(&self) -> bool {
        !self.shelf.is_empty()
            || self.read.listed
            || self.read.usage
            || self.read.instructions
            || self.read.vocabulary.is_some_and(|(named, _)| named > 0)
    }

    /// A dictation that cleanup changed is among the excerpts.
    pub fn heard(&self) -> bool {
        self.shelf
            .found()
            .excerpts
            .iter()
            .any(|excerpt| excerpt.text.starts_with(HEARD))
    }

    /// Runs `call`, within the turn's budget.
    pub fn run(&mut self, call: &Call, live: &dyn Live) -> Ran {
        let Some(tool) = TOOLS.iter().find(|tool| tool.name == call.tool) else {
            let names: Vec<&str> = TOOLS.iter().map(|tool| tool.name).collect();
            return Ran {
                text: format!(
                    "No tool is called {:?}. The tools: {}.",
                    shortened(&call.tool, 40),
                    names.join(", ")
                ),
                did: vec![Did::refused(&shortened(&call.tool, 40), Why::Unknown)],
            };
        };
        if self.spent() {
            return Ran {
                text: SPENT.to_owned(),
                did: vec![Did::refused(tool.name, Why::Budget)],
            };
        }
        self.calls += 1;
        let mut did = Vec::new();
        let most = match tool.name {
            // A search's own budget already bounds its excerpts.
            "recordings_search" => self.limits.search.chars + self.limits.search.chars / 4,
            _ => self.limits.result_chars,
        }
        .min(self.limits.turn_chars.saturating_sub(self.spent));
        let out = if tool.recordings {
            match self.open(live, &mut did) {
                Ok(open) => self.recordings(tool.name, call, &open, live),
                Err(refused) => Err(refused),
            }
        } else {
            self.plain(tool.name, call)
        };
        let text = match out {
            Ok((text, done)) => {
                did.push(done);
                capped(&text, most)
            }
            Err((why, text)) => {
                did.push(Did::refused(tool.name, why));
                text
            }
        };
        self.spent += text.chars().count();
        Ran { text, did }
    }

    fn plain(&mut self, tool: &str, call: &Call) -> Out {
        match tool {
            "guide_search" => self.guide_search(call),
            "guide_read" => self.guide_read(call),
            "facts" => self.facts(),
            "settings_get" => self.settings_get(call),
            "cleanup_instructions" => self.cleanup_instructions(),
            "vocabulary_lookup" => self.vocabulary_lookup(call),
            "vocabulary_list" => self.vocabulary_list(),
            _ => self.release_notes(call),
        }
    }

    fn guide_search(&mut self, call: &Call) -> Out {
        let query = call
            .text("query")
            .ok_or_else(|| failed("guide_search needs \"query\": the words to look for."))?;
        let limit = call.number("limit").unwrap_or(4).clamp(1, 6) as usize;
        let found = retrieve::search(query, limit);
        let mut text = match found.len() {
            0 => "No article matches; <guide_contents> lists every article.\n".to_owned(),
            _ => String::new(),
        };
        for article in &found {
            text.push_str(&format!(
                "{}: {} ({})\n",
                article.id, article.title, article.os
            ));
        }
        Ok((
            text,
            Did::GuideSearch {
                query: shortened(query, 80),
                found: found.len(),
            },
        ))
    }

    fn guide_read(&mut self, call: &Call) -> Out {
        let mut ids = call.list("ids");
        ids.extend(call.text("id").map(str::to_owned));
        if ids.is_empty() {
            return Err(failed(
                "guide_read needs \"ids\": article ids from <guide_contents>.",
            ));
        }
        let mut text = String::new();
        let mut read: Vec<String> = Vec::new();
        let mut keys: Vec<&str> = Vec::new();
        for id in ids.iter().take(3) {
            let id = id.trim_matches(['`', '"', ' ']);
            match guide::article(id) {
                Some(article) if self.articles.contains(&article.id) => {
                    text.push_str(&format!("{}: already above.\n", article.id));
                }
                Some(article) => {
                    text.push_str(&prompt::excerpt(article));
                    self.articles.push(article.id);
                    read.push(article.id.to_owned());
                    keys.extend(article.keys.iter().map(String::as_str));
                }
                None => text.push_str(&format!(
                    "{:?}: no article has this id; <guide_contents> lists them.\n",
                    shortened(id, 60)
                )),
            }
        }
        if ids.len() > 3 {
            text.push_str("Only the first 3 were read.\n");
        }
        text.push_str(&self.lines_of(&keys));
        if read.is_empty() && !text.contains("already above") {
            return Err(failed(text));
        }
        Ok((
            text,
            Did::Articles {
                ids: read,
                app: false,
            },
        ))
    }

    /// The catalog's lines for `keys`, under their header; nothing when
    /// none has a line or no settings came with the question.
    fn lines_of(&self, keys: &[&str]) -> String {
        let Some(context) = self.context else {
            return String::new();
        };
        let facts = context.facts();
        let lines = catalog::Lines::new(&context.settings, &facts);
        let mut seen: Vec<&str> = Vec::new();
        let mut text = String::new();
        for key in keys {
            if seen.contains(key) {
                continue;
            }
            seen.push(key);
            if let Some(line) = lines.line(key) {
                text.push_str(&line);
                text.push('\n');
            }
        }
        match text.as_str() {
            "" => text,
            _ => format!("Their settings:\n{}\n{text}", lines.header()),
        }
    }

    fn context(&self) -> Result<&'a changes::Context, (Why, String)> {
        self.context
            .ok_or_else(|| failed("The settings can't be read for this question."))
    }

    /// The settings, only while the person lets them be read.
    fn sent(&self) -> Result<&'a changes::Context, (Why, String)> {
        let context = self.context()?;
        match context.send() {
            true => Ok(context),
            false => Err((Why::NotSent, facts::text::NOT_SENT.to_owned())),
        }
    }

    fn facts(&mut self) -> Out {
        let context = self.context()?;
        Ok((
            facts::text::told(&context.facts()),
            Did::Settings { keys: Vec::new() },
        ))
    }

    fn settings_get(&mut self, call: &Call) -> Out {
        /// The most lines one call reads.
        const MOST: usize = 40;
        let context = self.context()?;
        let mut keys = call.list("keys");
        keys.extend(call.text("key").map(str::to_owned));
        if let Some(prefix) = call.text("prefix") {
            keys.extend(
                settings::kinds()
                    .map(|(key, ..)| key)
                    .chain([parts::CLEANUP, parts::PROMPT])
                    .filter(|key| key.starts_with(prefix))
                    .map(str::to_owned),
            );
        }
        let mut seen: Vec<String> = Vec::new();
        keys.retain(|key| {
            let new = !seen.contains(key);
            seen.push(key.clone());
            new
        });
        if keys.is_empty() {
            return Err(failed(
                "settings_get needs \"keys\" (settings keys as the guide and the catalog write \
                 them) or a \"prefix\" some keys start with.",
            ));
        }
        let facts = context.facts();
        let lines = catalog::Lines::new(&context.settings, &facts);
        let mut text = format!("{}\n", lines.header());
        let mut read = Vec::new();
        // A prefix names keys by the dozen: the ones with no line are left
        // out rather than listed.
        let named = call.list("keys").len() + usize::from(call.text("key").is_some());
        for (index, key) in keys.iter().enumerate() {
            match lines.line(key) {
                Some(line) if read.len() < MOST => {
                    text.push_str(&line);
                    text.push('\n');
                    read.push(key.clone());
                }
                Some(_) => {}
                None if index < named => text.push_str(&format!(
                    "{}: not a setting you can read here.\n",
                    shortened(key, 80)
                )),
                None => {}
            }
        }
        if !lines.send() {
            text.push_str(facts::text::NOT_SENT);
        }
        Ok((text, Did::Settings { keys: read }))
    }

    fn cleanup_instructions(&mut self) -> Out {
        let context = self.sent()?;
        self.read.instructions = true;
        Ok((personal::instructions(&context.settings), Did::Instructions))
    }

    fn vocabulary_lookup(&mut self, call: &Call) -> Out {
        let context = self.sent()?;
        let mut words = call.list("words");
        words.extend(call.text("word").map(str::to_owned));
        words.extend(call.text("query").map(str::to_owned));
        if words.is_empty() {
            return Err(failed(
                "vocabulary_lookup needs \"words\": the words or terms to look up.",
            ));
        }
        let texts: Vec<&str> = words.iter().map(String::as_str).collect();
        let (block, named) = personal::vocabulary(&context.settings, &texts);
        let total = context.settings.vocabulary.len();
        self.vocabulary(named, total);
        Ok((block, Did::Vocabulary { named, total }))
    }

    fn vocabulary(&mut self, named: usize, total: usize) {
        let before = self.read.vocabulary.map_or(0, |(named, _)| named);
        self.read.vocabulary = Some((before + named, total));
    }

    fn vocabulary_list(&mut self) -> Out {
        /// The most characters of terms one list holds.
        const MOST: usize = 4_000;
        let context = self.sent()?;
        let all = &context.settings.vocabulary;
        let mut text = format!("<vocabulary_terms total=\"{}\">\n", all.len());
        let mut listed = 0;
        let mut chars = 0;
        for entry in all {
            let term = entry.term.trim();
            chars += term.chars().count() + 1;
            if chars > MOST {
                break;
            }
            text.push_str(term);
            text.push('\n');
            listed += 1;
        }
        if listed < all.len() {
            text.push_str(&format!("and {} more\n", all.len() - listed));
        }
        text.push_str(
            "</vocabulary_terms>\nTerms only; vocabulary_lookup gives an entry's sounds-like \
             spellings.\n",
        );
        self.vocabulary(listed, all.len());
        Ok((
            text,
            Did::Vocabulary {
                named: listed,
                total: all.len(),
            },
        ))
    }

    fn release_notes(&mut self, call: &Call) -> Out {
        let asked = releases::Asked {
            version: call.text("version").map(str::to_owned),
            history: call.flag("history"),
        };
        let selection = releases::select(&asked, releases::Running::here());
        let mut text = releases::prompt::block(&selection);
        text.push_str(&self.lines_of(&selection.settings_keys()));
        self.read.releases = true;
        Ok((
            text,
            Did::Releases {
                versions: selection
                    .notes
                    .iter()
                    .map(|note| note.version.clone())
                    .collect(),
            },
        ))
    }

    /// Leave to read the recordings: the person's, asked for once when
    /// they haven't said.
    fn open(&mut self, live: &dyn Live, did: &mut Vec<Did>) -> Result<Open, (Why, String)> {
        let closed = || (Why::NotAllowed, CLOSED.to_owned());
        match std::mem::take(&mut self.access) {
            Access::Open {
                allowed,
                sources,
                vocabulary,
            } => {
                self.access = Access::Open {
                    allowed: allowed.clone(),
                    sources: sources.clone(),
                    vocabulary: vocabulary.clone(),
                };
                Ok(Open {
                    allowed,
                    sources,
                    vocabulary,
                })
            }
            Access::Ask {
                sources,
                vocabulary,
            } => match live.consent() {
                Consent::Allowed(allowed) => {
                    did.push(Did::Consent { allowed: true });
                    self.access = Access::Open {
                        allowed: allowed.clone(),
                        sources: sources.clone(),
                        vocabulary: vocabulary.clone(),
                    };
                    Ok(Open {
                        allowed,
                        sources,
                        vocabulary,
                    })
                }
                Consent::Declined => {
                    did.push(Did::Consent { allowed: false });
                    self.read.closed = true;
                    Err(closed())
                }
                // Stopped before saying: nothing was decided, so the next
                // question may ask again.
                Consent::Stopped => {
                    self.access = Access::Ask {
                        sources,
                        vocabulary,
                    };
                    Err(failed("Stopped."))
                }
            },
            Access::Closed => {
                self.read.closed = true;
                Err(closed())
            }
        }
    }

    fn recordings(&mut self, tool: &str, call: &Call, open: &Open, live: &dyn Live) -> Out {
        let (allowed, sources) = (&open.allowed, &open.sources);
        let offsets = (call.text("from"), call.text("to"));
        match tool {
            "recordings_search" => self.search(call, open, live),
            "meetings_list" => {
                let limit = call.number("limit").unwrap_or(10) as usize;
                let listed = read::meetings_list(
                    allowed,
                    sources,
                    (call.text("since"), call.text("until"), call.text("query")),
                    limit,
                    self.now_ms,
                )
                .map_err(failed)?;
                Ok(self.listed(listed, |searched, found| Step::Meetings { searched, found }))
            }
            "files_list" => {
                let listed = read::files_list(allowed, sources, call.text("query"));
                Ok(self.listed(listed, |searched, found| Step::Files { searched, found }))
            }
            "meeting_notes" => {
                let id = self.id(call)?;
                let notes = read::meeting_notes(allowed, sources, &id).map_err(failed)?;
                let title = notes.title.clone();
                Ok((self.shelved(vec![notes], None), Did::Notes { title }))
            }
            "meeting_transcript" => {
                let id = self.id(call)?;
                let read = read::meeting_transcript(allowed, sources, &id, offsets, self.now_ms)
                    .map_err(failed)?;
                let title = read.excerpt.title.clone();
                Ok((self.stretch(read, tool), Did::Transcript { title }))
            }
            "file_read" => {
                let id = self.id(call)?;
                let read = read::file_read(allowed, sources, &id, offsets).map_err(failed)?;
                let title = read.excerpt.title.clone();
                Ok((self.stretch(read, tool), Did::File { title }))
            }
            "dictation_read" => {
                let id = self.id(call)?;
                let read = read::dictation_read(allowed, sources, &id).map_err(failed)?;
                Ok((self.shelved(vec![read], None), Did::Dictation))
            }
            _ => {
                let known: Vec<String> = open
                    .vocabulary
                    .iter()
                    .map(|entry| entry.term.clone())
                    .collect();
                let lines = read::usage_lines(allowed, sources, &known, self.now_ms);
                self.read.usage = true;
                let text = match lines.is_empty() {
                    true => "Nothing to go by yet: no dictations or meetings.\n".to_owned(),
                    false => format!("<usage>\n{}\n</usage>\n", lines.join("\n")),
                };
                Ok((text, Did::Usage))
            }
        }
    }

    fn search(&mut self, call: &Call, open: &Open, live: &dyn Live) -> Out {
        let mut kinds = Kinds::default();
        let mut named = call.list("kinds");
        named.extend(call.text("kind").map(str::to_owned));
        for kind in &named {
            match kind.to_lowercase().trim_end_matches('s') {
                "meeting" => kinds.meetings = true,
                "file" => kinds.files = true,
                "dictation" => kinds.dictations = true,
                _ => {
                    return Err(failed(format!(
                        "recordings_search knows the kinds meetings, files, dictations, not {:?}.",
                        shortened(kind, 40)
                    )));
                }
            }
        }
        let asked = Asked {
            query: call.text("query").unwrap_or_default().to_owned(),
            kinds,
            since: call.text("since").map(str::to_owned),
            until: call.text("until").map(str::to_owned),
            latest: call.number("latest").map(|latest| latest as usize),
        };
        // What is left of the turn's budget bounds this search too.
        let left = self.limits.turn_chars.saturating_sub(self.spent);
        let budget = Budget {
            chars: self.limits.search.chars.min(left),
            ..self.limits.search
        };
        let found = read::search(
            &open.allowed,
            &open.sources,
            &asked,
            &open.vocabulary,
            (budget, self.now_ms),
            &mut |step| live.step(step),
        )
        .map_err(failed)?;
        let steps = found.steps.clone();
        self.shelf.stepped(found.steps);
        Ok((
            self.shelved(found.excerpts, None),
            Did::Recordings { steps },
        ))
    }

    fn listed(&mut self, listed: Listed, step: impl Fn(usize, usize) -> Step) -> (String, Did) {
        self.read.listed |= listed.found > 0;
        (
            listed.text,
            Did::Recordings {
                steps: vec![step(listed.searched, listed.found)],
            },
        )
    }

    /// The recording a call names: its `id`, or the one excerpt `n` is of.
    fn id(&self, call: &Call) -> Result<String, (Why, String)> {
        if let Some(n) = call.number("n") {
            return self
                .shelf
                .excerpt(n as u32)
                .map(|excerpt| excerpt.id.clone())
                .ok_or_else(|| failed(format!("No excerpt [{n}] has been read yet.")));
        }
        match call.args.get("id") {
            Some(serde_json::Value::String(id)) if !id.trim().is_empty() => {
                Ok(id.trim().to_owned())
            }
            Some(serde_json::Value::Number(id)) => Ok(id.to_string()),
            _ => Err(failed(format!(
                "{} needs \"id\" (from a list) or \"n\" (an excerpt's number).",
                call.tool
            ))),
        }
    }

    fn stretch(&mut self, read: Stretch, tool: &str) -> String {
        let more = read.more.map(|from| {
            format!("There is more: call {tool} again with \"from\":\"{from}\" to read on.\n")
        });
        self.shelved(vec![read.excerpt], more)
    }

    /// Puts `excerpts` on the turn's shelf and writes the new ones as the
    /// model reads them, numbered on from the last; ones read before are
    /// named by the number they have.
    fn shelved(&mut self, excerpts: Vec<Excerpt>, more: Option<String>) -> String {
        let placed = self.shelf.put(excerpts);
        let new: Vec<Excerpt> = placed
            .iter()
            .filter(|placed| placed.new)
            .filter_map(|placed| self.shelf.excerpt(placed.n).cloned())
            .collect();
        let again: Vec<String> = placed
            .iter()
            .filter(|placed| !placed.new)
            .map(|placed| format!("[{}]", placed.n))
            .collect();
        let mut text = match (new.is_empty(), again.is_empty()) {
            (true, false) => String::new(),
            _ => prompt::tagged("recordings", &new, self.now_ms),
        };
        if !again.is_empty() {
            text.push_str(&format!("Already above: {}.\n", again.join(", ")));
        }
        text.push_str(&more.unwrap_or_default());
        text
    }
}

/// The recordings, opened for one call.
struct Open {
    allowed: Allowed,
    sources: Sources,
    vocabulary: Vec<VocabularyEntry>,
}

/// `text` whole, or cut at `chars` on a line's end, saying it was cut.
fn capped(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((cut, _)) => {
            let cut = text[..cut].rfind('\n').unwrap_or(cut);
            format!(
                "{}\n… (cut here: the rest didn't fit)\n",
                text[..cut].trim_end()
            )
        }
        None => text.to_owned(),
    }
}

fn shortened(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
