//! The user guide as compiled in: one article per Labs feature
//! (`labs/<hex labs name>`) and per topic (`topics/<slug>`). Front matter
//! gives the title, settings keys, and systems; the trailing
//! `<!-- sources: … -->` comment is for authors and never reaches a model.

use std::sync::OnceLock;

use crate::desktop::hardware::Need;

include!(concat!(env!("OUT_DIR"), "/guide_files.rs"));

/// One guide article.
#[derive(Debug)]
pub struct Article {
    /// `labs/smart-spacing`, `topics/meetings`.
    pub id: &'static str,
    pub title: String,
    /// The `hex settings` keys it names (`key:` or `keys: [..]`).
    pub keys: Vec<String>,
    /// The systems it applies to, as written: `macOS`, `every OS`.
    pub os: String,
    /// Markdown after the front matter, without comments.
    pub body: String,
    /// The "Asked as" phrasings, English and Russian.
    pub asked_as: String,
    /// A Labs feature's uses setup offers it for (`for: [code, ru]`):
    /// `dictation`, `code`, `meetings`, `files`, and the qualifiers `ru`
    /// and `cleanup`; empty, setup never offers it on its own.
    pub uses: Vec<String>,
    /// Setup's yes/no question about it, in English (`ask:`).
    pub ask: String,
    /// What else it takes after the switch, in English (`then:`), if
    /// anything.
    pub then: String,
    /// What it needs of the hardware (`needs: [notch]`); one unmet here
    /// keeps it out of setup and the answers' suggestions.
    pub needs: Vec<Need>,
    /// Setup's question while it isn't known whether the need is met
    /// (`ask_if:`, "If your mouse has extra buttons, …"); else `ask`.
    pub ask_if: String,
}

impl Article {
    /// Whether this computer may have what it needs: false only when a
    /// need is known not to be met.
    pub fn fits(&self, hardware: &crate::desktop::hardware::Hardware) -> bool {
        hardware.allows(&self.needs)
    }

    /// Setup's question about it here: `ask_if` while a need is unknown,
    /// else `ask`.
    pub fn question(&self, hardware: &crate::desktop::hardware::Hardware) -> &str {
        let unsure = self.needs.iter().any(|need| hardware.has(*need).is_none());
        if unsure && !self.ask_if.is_empty() {
            &self.ask_if
        } else {
            &self.ask
        }
    }

    /// Whether it describes a Labs feature, not a topic.
    pub fn is_lab(&self) -> bool {
        self.id.starts_with("labs/")
    }
}

/// Every article, sorted by id.
pub fn articles() -> &'static [Article] {
    static ARTICLES: OnceLock<Vec<Article>> = OnceLock::new();
    ARTICLES.get_or_init(|| FILES.iter().map(|(id, text)| parse(id, text)).collect())
}

/// The article with this id.
pub fn article(id: &str) -> Option<&'static Article> {
    articles().iter().find(|article| article.id == id)
}

fn parse(id: &'static str, text: &str) -> Article {
    let text = text.replace("\r\n", "\n");
    let (front, body) = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .unwrap_or(("", &text));
    let field = |name: &str| {
        front.lines().find_map(|line| {
            line.strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(':'))
                .map(|value| value.trim().to_owned())
        })
    };
    let list = |text: String| -> Vec<String> {
        text.trim_matches(['[', ']'])
            .split(',')
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    };
    let keys = field("keys")
        .map(list)
        .or_else(|| field("key").map(|key| vec![key]))
        .unwrap_or_default();
    let body = without_comments(body).trim().to_owned();
    Article {
        id,
        title: field("title").unwrap_or_else(|| id.to_owned()),
        keys,
        os: field("os").unwrap_or_default(),
        uses: field("for").map(list).unwrap_or_default(),
        ask: field("ask").unwrap_or_default(),
        then: field("then").unwrap_or_default(),
        needs: field("needs")
            .map(list)
            .unwrap_or_default()
            .iter()
            .filter_map(|need| Need::parse(need))
            .collect(),
        ask_if: field("ask_if").unwrap_or_default(),
        asked_as: asked_as(&body),
        body,
    }
}

/// Text with every `<!-- … -->` removed.
fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("-->")
            .map_or("", |end| &rest[start + end + 3..]);
    }
    out.push_str(rest);
    out
}

/// The paragraph that starts with `**Asked as.**`, without that label.
fn asked_as(body: &str) -> String {
    let Some(start) = body.find("**Asked as.**") else {
        return String::new();
    };
    let paragraph = &body[start + "**Asked as.**".len()..];
    let end = paragraph.find("\n\n").unwrap_or(paragraph.len());
    paragraph[..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `needs:` names only what the app can tell, and an article with one
    /// asks the unsure way too.
    #[test]
    fn hardware_needs_parse_and_come_with_an_unsure_question() {
        use crate::desktop::hardware::Need;
        let needs = |id: &str| article(id).unwrap().needs.clone();
        assert_eq!(needs("labs/notch-indicator"), [Need::Notch]);
        assert_eq!(needs("labs/mouse-shortcuts"), [Need::MouseButtons]);
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/guide");
        for article in articles() {
            let text = std::fs::read_to_string(root.join(format!("{}.md", article.id))).unwrap();
            let written = text.lines().find_map(|line| line.strip_prefix("needs:"));
            let named = written.map_or(0, |list| list.split(',').count());
            assert_eq!(
                article.needs.len(),
                named,
                "{}: an unknown need",
                article.id
            );
            assert_eq!(
                article.needs.is_empty(),
                article.ask_if.is_empty(),
                "{}: needs and ask_if go together",
                article.id
            );
        }
    }

    #[test]
    fn every_guide_file_is_an_article_without_its_sources() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/guide");
        let on_disk = ["labs", "topics"]
            .iter()
            .map(|part| std::fs::read_dir(root.join(part)).unwrap().count())
            .sum::<usize>();
        assert_eq!(articles().len(), on_disk);
        for article in articles() {
            assert!(
                !article.body.contains("<!--"),
                "{} keeps a comment",
                article.id
            );
            assert!(
                !article.body.starts_with("---"),
                "{} keeps front matter",
                article.id
            );
            assert!(
                article.body.starts_with("# "),
                "{} has no title heading",
                article.id
            );
            assert!(
                !article.asked_as.is_empty(),
                "{} has no Asked as",
                article.id
            );
            assert!(
                !article.title.is_empty() && !article.os.is_empty(),
                "{}",
                article.id
            );
        }
    }

    #[test]
    fn front_matter_gives_title_keys_and_systems() {
        let spacing = article("labs/smart-spacing").unwrap();
        assert_eq!(spacing.title, "Smart spacing");
        assert_eq!(spacing.keys, ["labs.smart_spacing"]);
        assert_eq!(spacing.os, "macOS");
        assert!(spacing.is_lab());
        let meetings = article("topics/meetings").unwrap();
        assert!(meetings.keys.iter().any(|key| key == "meetings.enabled"));
        assert!(!meetings.is_lab());
        assert!(spacing.asked_as.contains("слипается"));
    }

    /// Codex gets the whole guide in its prompt, so it has to stay small.
    #[test]
    fn the_whole_guide_stays_under_50k_tokens() {
        let chars: usize = articles().iter().map(|article| article.body.len()).sum();
        assert!(chars / 4 < 50_000, "about {} tokens", chars / 4);
    }

    /// Every prompt carries the table of contents, small models included.
    #[test]
    fn the_table_of_contents_stays_under_3k_tokens() {
        let chars = crate::assistant::prompt::contents().len();
        assert!(chars / 4 < 3_000, "about {} tokens", chars / 4);
    }

    /// A small model reads three articles in a short window, so none is long.
    #[test]
    fn every_article_stays_under_2k_tokens() {
        let long: Vec<String> = articles()
            .iter()
            .filter(|article| article.body.len() / 4 >= 2_000)
            .map(|article| format!("{}: about {} tokens", article.id, article.body.len() / 4))
            .collect();
        assert!(long.is_empty(), "{}", long.join("\n"));
    }

    /// Setup asks about Labs features from their front matter: each one
    /// says whom it is for and asks one short question, and one it may
    /// offer turns on with a switch.
    #[test]
    fn every_labs_article_says_whom_setup_asks_and_how() {
        const TAGS: [&str; 6] = ["dictation", "code", "meetings", "files", "ru", "cleanup"];
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/guide/labs");
        for article in articles().iter().filter(|article| article.is_lab()) {
            let file = root.join(format!("{}.md", &article.id["labs/".len()..]));
            let text = std::fs::read_to_string(&file).unwrap();
            assert!(text.contains("\nfor:"), "{} has no for:", article.id);
            let words = article.ask.split_whitespace().count();
            assert!(
                (1..=12).contains(&words) && article.ask.ends_with('?'),
                "{}: ask {:?}",
                article.id,
                article.ask
            );
            assert!(
                article.then.split_whitespace().count() <= 12,
                "{}",
                article.id
            );
            for tag in &article.uses {
                assert!(TAGS.contains(&tag.as_str()), "{}: {tag}", article.id);
            }
            if article.uses.is_empty() {
                continue;
            }
            assert!(
                article
                    .uses
                    .iter()
                    .any(|tag| !["ru", "cleanup"].contains(&tag.as_str())),
                "{} names no use",
                article.id
            );
            let key = article.keys.first().map(String::as_str).unwrap_or_default();
            let kind = crate::control::settings::kinds().find(|(known, ..)| *known == key);
            assert!(
                kind.is_some_and(|(_, kind, ..)| kind == "bool"),
                "{}: {key} is no switch",
                article.id
            );
        }
    }

    #[test]
    fn comments_are_removed_wherever_they_are() {
        assert_eq!(without_comments("a<!-- x -->b<!-- y"), "ab");
    }
}
