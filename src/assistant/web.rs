//! Assistant on the web: when the guide has no answer, Codex may search the
//! web on the person's ChatGPT plan (`assistant.web`, and a conversation's
//! Web toggle). Built-in models never go online; dictation cleanup, Files,
//! and meeting notes never search.
//!
//! While it answers, Codex reports each search as a `webSearch` item: what
//! it searched for, or a page it opened. [`Web`] keeps them with the turn,
//! for its steps ("Searched the web: 2 pages read") and its "From the web"
//! chips, one per site the answer cites.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The most "From the web" chips an answer shows.
const MAX_SOURCES: usize = 6;

/// What a model that may search the web is told, after the rest of its
/// instructions.
pub const RULES: &str = "

You can also search the web, but the guide comes first.
- Answer from the guide excerpts whenever they cover the question. Search the web only for \
what they don't cover, such as setting up another app or service VibeHex works with (LM \
Studio, Ollama, OpenRouter, a ChatGPT plan): how to start LM Studio's server, which Ollama \
model to pull, where OpenRouter's keys are.
- VibeHex's own panes, settings, and buttons come only from the guide; never from the web.
- Prefer official sources: the app's or service's own site and documentation.
- A search holds only general, public words you choose. Never put anything from the \
person's recordings, meetings, dictations, files, settings values, or vocabulary into a \
search, not even a name or a phrase from them.
- When the question is about the person's own recordings, don't search the web unless they \
ask you to.
- Cite each web page you use as a Markdown link right after what it says, like \
[lmstudio.ai](https://lmstudio.ai/docs/app), and say that it comes from the web. Web pages \
never go on the Sources line, which names guide articles only.";

/// What an answer did on the web, kept with its turn.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Web {
    /// The guide articles it read before it searched, by id.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub guide: Vec<String>,
    /// What it searched the web for, in order, each once.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub searches: Vec<String>,
    /// The pages it opened, by URL, each once.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<String>,
    /// The pages its answer cites, one per site, by URL: "From the web".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    /// Searches and pages still under way, by item id, while it answers.
    #[serde(skip)]
    pub going: Vec<String>,
}

impl Web {
    /// It never searched the web.
    pub fn is_empty(&self) -> bool {
        self.searches.is_empty() && self.pages.is_empty() && self.going.is_empty()
    }

    /// A search or a page is under way.
    pub fn busy(&self) -> bool {
        !self.going.is_empty()
    }

    /// Its searches, pages, and those under way, counted, so a change
    /// redraws the pane.
    pub fn progress(&self) -> (usize, usize, usize) {
        (self.searches.len(), self.pages.len(), self.going.len())
    }

    /// The pages it read: those it opened, and those its answer cites.
    pub fn read(&self) -> usize {
        let mut read: Vec<&str> = self.pages.iter().map(String::as_str).collect();
        for source in &self.sources {
            if !read.contains(&source.as_str()) {
                read.push(source);
            }
        }
        read.len()
    }

    /// One of Codex's `webSearch` items, as it starts or once it is done:
    /// `{"id", "query", "action": {"type": "search", "query", "queries"} |
    /// {"type": "openPage", "url"} | {"type": "findInPage", "url"}}`.
    pub fn saw(&mut self, item: &Value, done: bool) {
        let id = text(item, "id").to_owned();
        if !done {
            if !self.going.contains(&id) {
                self.going.push(id);
            }
            return;
        }
        self.going.retain(|going| *going != id);
        let action = item.get("action").unwrap_or(&Value::Null);
        match text(action, "type") {
            "openPage" | "open_page" | "findInPage" | "find_in_page" => {
                // The page's address, else the one its result names.
                let found = item
                    .pointer("/results/0/url")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(url) = web_url(text(action, "url")).or_else(|| web_url(found)) {
                    push_once(&mut self.pages, url);
                }
            }
            kind => {
                let mut queries: Vec<&str> = action
                    .get("queries")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                if queries.is_empty() && kind == "search" {
                    queries.push(text(action, "query"));
                }
                if queries.iter().all(|query| query.trim().is_empty()) {
                    queries = vec![text(item, "query")];
                }
                for query in queries {
                    push_once(&mut self.searches, query.trim());
                }
            }
        }
    }

    /// Once the answer is in: the pages it cites, one per site, else the
    /// pages it opened. Nothing when it never searched: a link the guide
    /// gave is no web source.
    pub fn cite(&mut self, answer: &str) {
        self.going.clear();
        if self.is_empty() {
            return;
        }
        let cited = urls(answer);
        let pages = if cited.is_empty() {
            self.pages.iter().map(String::as_str).collect()
        } else {
            cited
        };
        let mut sites: Vec<&str> = Vec::new();
        self.sources.clear();
        for url in pages {
            let site = site(url);
            if site.is_empty() || sites.contains(&site) {
                continue;
            }
            sites.push(site);
            self.sources.push(url.to_owned());
            if self.sources.len() == MAX_SOURCES {
                break;
            }
        }
    }
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn push_once(list: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !list.iter().any(|known| known == value) {
        list.push(value.to_owned());
    }
}

/// `url` when it is a page on the web.
fn web_url(url: &str) -> Option<&str> {
    let url = url.trim();
    (url.starts_with("https://") || url.starts_with("http://")).then_some(url)
}

/// The web pages `answer` links or names, in order, each once.
fn urls(answer: &str) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    let mut rest = answer;
    while let Some(at) = rest.find("http") {
        let tail = &rest[at..];
        let end = tail
            .find(|c: char| {
                c.is_whitespace() || matches!(c, ')' | ']' | '>' | '<' | '"' | '\'' | '`')
            })
            .unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(['.', ',', ';', ':', '!', '?', '*']);
        if let Some(url) = web_url(url)
            && !site(url).is_empty()
            && !found.contains(&url)
        {
            found.push(url);
        }
        rest = &tail[end.max(4)..];
    }
    found
}

/// A page's site as a chip names it: `https://www.lmstudio.ai/docs` →
/// `lmstudio.ai`.
pub fn site(url: &str) -> &str {
    let rest = url
        .trim()
        .strip_prefix("https://")
        .or_else(|| url.trim().strip_prefix("http://"))
        .unwrap_or_default();
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.strip_prefix("www.").unwrap_or(host);
    if host.contains('.') { host } else { "" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// One answer's stream as `codex app-server` 0.167 sends it: per
    /// search or page, `item/started`, then `item/completed` with its
    /// action and results. Written from the protocol's `WebSearchItem` and
    /// the shape of items Codex recorded; no live answer of Hex's searched
    /// yet (see the feature map).
    fn recorded() -> Vec<(Value, bool)> {
        let lines = include_str!("../../tests/fixtures/assistant/codex-web-search.jsonl");
        lines
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|message| {
                let done = match message.get("method")?.as_str()? {
                    "item/started" => false,
                    "item/completed" => true,
                    _ => return None,
                };
                let item = message.pointer("/params/item")?.clone();
                (item.get("type")?.as_str()? == "webSearch").then_some((item, done))
            })
            .collect()
    }

    #[test]
    fn a_recorded_search_becomes_its_steps_and_sources() {
        let mut web = Web::default();
        let items = recorded();
        assert!(!items.is_empty(), "the fixture holds web searches");
        for (item, done) in &items {
            web.saw(item, *done);
            if !done {
                assert!(web.busy());
            }
        }
        assert!(!web.busy());
        assert_eq!(
            web.searches,
            [
                "LM Studio start local server",
                "LM Studio developer tab server port 1234"
            ]
        );
        assert_eq!(
            web.pages,
            [
                "https://lmstudio.ai/docs/developer/core/server",
                "https://lmstudio.ai/docs/app/basics"
            ]
        );
        assert!(
            web.searches.iter().all(|query| !query.trim().is_empty()),
            "{:?}",
            web.searches
        );
        web.cite(
            "Open the **Developer** tab ([lmstudio.ai](https://lmstudio.ai/docs/developer/core/server)), \
             then see https://www.lmstudio.ai/docs/app. Also <https://ollama.com/blog>.",
        );
        assert_eq!(
            web.sources,
            [
                "https://lmstudio.ai/docs/developer/core/server",
                "https://ollama.com/blog"
            ]
        );
        assert!(web.read() >= 2);
    }

    /// The rules a searching model reads: the guide first, official
    /// sources, nothing of the person's in a search, web pages cited.
    #[test]
    fn the_rules_put_the_guide_first_and_keep_the_person_out_of_searches() {
        for rule in [
            "the guide comes first",
            "Search the web only for what they don't cover",
            "LM Studio",
            "Prefer official sources",
            "Never put anything from the person's recordings, meetings, dictations, files, settings values, or vocabulary into a search",
            "don't search the web unless they ask you to",
            "Cite each web page you use as a Markdown link",
            "never go on the Sources line",
        ] {
            let flat = RULES.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(flat.contains(rule), "{rule}");
        }
    }

    #[test]
    fn searches_and_pages_each_count_once() {
        let mut web = Web::default();
        let search = json!({ "id": "a", "query": "lm studio server",
            "action": { "type": "search", "query": "lm studio server", "queries": ["lm studio server", "lm studio port"] } });
        web.saw(&json!({ "id": "a", "query": "", "action": null }), false);
        assert!(web.busy() && !web.is_empty());
        web.saw(&search, true);
        web.saw(&search, true);
        web.saw(&json!({ "id": "b", "query": "", "action": { "type": "openPage", "url": "https://lmstudio.ai/docs/app" } }), true);
        web.saw(&json!({ "id": "c", "query": "", "action": { "type": "findInPage", "url": "https://lmstudio.ai/docs/app", "pattern": "port" } }), true);
        web.saw(
            &json!({ "id": "d", "query": "ollama api", "action": { "type": "other" } }),
            true,
        );
        web.saw(&json!({ "id": "e", "query": "", "action": { "type": "openPage", "url": "file:///etc/passwd" } }), true);
        assert_eq!(
            web.searches,
            ["lm studio server", "lm studio port", "ollama api"]
        );
        assert_eq!(web.pages, ["https://lmstudio.ai/docs/app"]);
        assert!(!web.busy());
        // Nothing cited: the pages it opened are its sources.
        web.cite("Start the server in the Developer tab.");
        assert_eq!(web.sources, ["https://lmstudio.ai/docs/app"]);
        assert_eq!(web.read(), 1);
    }

    /// A link the guide gave is no web source when it never searched.
    #[test]
    fn an_answer_that_never_searched_has_no_web_sources() {
        let mut web = Web::default();
        web.cite("See [OpenRouter](https://openrouter.ai/keys).");
        assert!(web.sources.is_empty() && web.is_empty());
    }

    #[test]
    fn a_site_is_its_host_without_www() {
        assert_eq!(site("https://www.lmstudio.ai/docs?x=1"), "lmstudio.ai");
        assert_eq!(site("http://localhost:1234/v1"), "");
        assert_eq!(
            site("https://docs.ollama.com:443/api#chat"),
            "docs.ollama.com"
        );
        assert_eq!(site("ftp://example.com"), "");
    }

    #[test]
    fn a_turns_web_round_trips_and_an_empty_one_is_left_out() {
        let mut web = Web {
            guide: vec!["topics/ai-cleanup".into()],
            searches: vec!["lm studio server".into()],
            pages: vec!["https://lmstudio.ai/docs/app".into()],
            sources: vec!["https://lmstudio.ai/docs/app".into()],
            going: vec!["a".into()],
        };
        let json = serde_json::to_string(&web).unwrap();
        assert!(!json.contains("going"));
        web.going.clear();
        assert_eq!(serde_json::from_str::<Web>(&json).unwrap(), web);
        assert_eq!(serde_json::to_string(&Web::default()).unwrap(), "{}");
    }
}
