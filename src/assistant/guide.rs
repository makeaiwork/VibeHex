//! The user guide as compiled in: one article per Labs feature
//! (`labs/<hex labs name>`) and per topic (`topics/<slug>`). Front matter
//! gives the title, settings keys, and systems; the trailing
//! `<!-- sources: … -->` comment is for authors and never reaches a model.

use std::sync::OnceLock;

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
}

impl Article {
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
    let keys = field("keys")
        .map(|list| {
            list.trim_matches(['[', ']'])
                .split(',')
                .map(|key| key.trim().to_owned())
                .filter(|key| !key.is_empty())
                .collect()
        })
        .or_else(|| field("key").map(|key| vec![key]))
        .unwrap_or_default();
    let body = without_comments(body).trim().to_owned();
    Article {
        id,
        title: field("title").unwrap_or_else(|| id.to_owned()),
        keys,
        os: field("os").unwrap_or_default(),
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
    fn the_whole_guide_stays_under_32k_tokens() {
        let chars: usize = articles().iter().map(|article| article.body.len()).sum();
        assert!(chars / 4 < 32_000, "about {} tokens", chars / 4);
    }

    #[test]
    fn comments_are_removed_wherever_they_are() {
        assert_eq!(without_comments("a<!-- x -->b<!-- y"), "ab");
    }
}
