//! What a model reads for one question: Assistant's instructions, the
//! guide's table of contents, the articles search found, the conversation
//! so far, and the question. Nothing else: no transcripts, history,
//! meetings, or keys. The model ends its reply with a `Sources:` line,
//! which the pane turns into "From the guide" chips and never shows.

use super::guide::{self, Article};
use super::retrieve;

/// How many articles a model reads: a small local one has a short window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Room {
    Small,
    Large,
}

impl Room {
    fn articles(self) -> usize {
        match self {
            Self::Small => 3,
            Self::Large => 6,
        }
    }

    /// How much of each earlier answer goes back in.
    fn earlier_answer_chars(self) -> usize {
        match self {
            Self::Small => 600,
            Self::Large => 4_000,
        }
    }

    /// How many earlier turns go back in, newest kept.
    fn earlier_turns(self) -> usize {
        match self {
            Self::Small => 2,
            Self::Large => 8,
        }
    }
}

/// One earlier question and its answer.
#[derive(Clone, Copy, Debug)]
pub struct Turn<'a> {
    pub question: &'a str,
    pub answer: &'a str,
}

/// A question, ready for an engine.
#[derive(Debug)]
pub struct Prompt {
    pub instructions: String,
    pub input: String,
    /// The articles it carries, best first.
    pub articles: Vec<&'static str>,
}

const INSTRUCTIONS: &str = "You are Assistant, the help built into VibeHex, a dictation app \
for macOS, Windows, and Linux. Answer the person's question about VibeHex from the guide \
excerpts you are given.

- Reply in the language of the person's latest question, whatever the language of the guide \
or of the app. If that question has no clear language, keep the conversation's.
- Name panes, buttons, and settings exactly as the guide writes them, in English, with › \
between levels (Labs › Meetings), so they can be found on screen.
- Answer first, in a sentence or two, then the steps as a short numbered list when there \
are steps. No preamble.
- Say which systems a feature works on when it matters, and say plainly when VibeHex cannot \
do something.
- Use only the excerpts. When they don't answer the question, say the guide doesn't cover \
it. Never invent settings, keys, buttons, or menus.
- You cannot change settings yourself; say where the person changes them.
- For questions that are not about VibeHex, say in one sentence that you help with VibeHex.
- End with one last line: `Sources:` and the ids of the articles you used, comma-separated \
(for example `Sources: labs/meetings, topics/codex`), or `Sources: none`.";

/// The prompt for `question` after `earlier` turns, oldest first.
pub fn build(question: &str, earlier: &[Turn], room: Room) -> Prompt {
    // A follow-up ("and on Windows?") is about what the last question was.
    let query = match earlier.last() {
        Some(last) => format!("{} {}", last.question, question),
        None => question.to_owned(),
    };
    let found = retrieve::search(&query, room.articles());
    let mut input = String::from("<guide_contents>\n");
    for article in guide::articles() {
        input.push_str(&format!(
            "{}: {} ({})\n",
            article.id, article.title, article.os
        ));
    }
    input.push_str("</guide_contents>\n");
    for article in &found {
        input.push_str(&excerpt(article));
    }
    let kept = earlier.len().saturating_sub(room.earlier_turns());
    for turn in &earlier[kept..] {
        input.push_str(&format!(
            "\n<earlier_question>\n{}\n</earlier_question>\n<earlier_answer>\n{}\n</earlier_answer>\n",
            turn.question.trim(),
            shortened(turn.answer.trim(), room.earlier_answer_chars()),
        ));
    }
    input.push_str(&format!("\n<question>\n{}\n</question>\n", question.trim()));
    Prompt {
        instructions: INSTRUCTIONS.to_owned(),
        input,
        articles: found.iter().map(|article| article.id).collect(),
    }
}

fn excerpt(article: &Article) -> String {
    format!(
        "\n<guide_article id=\"{}\">\n{}\n</guide_article>\n",
        article.id, article.body
    )
}

fn shortened(text: &str, chars: usize) -> String {
    match text.char_indices().nth(chars) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

/// A reply split into what the pane shows and the guide articles it names
/// on its `Sources:` line, only ids the guide has, in order.
pub fn split_sources(reply: &str) -> (&str, Vec<&'static str>) {
    let trimmed = reply.trim_end();
    let (body, last) = trimmed.rsplit_once('\n').unwrap_or(("", trimmed));
    let line = last.trim().trim_matches(['*', '`', '_']).trim();
    let Some(list) = line
        .get(..8)
        .filter(|head| head.eq_ignore_ascii_case("sources:"))
        .map(|_| &line[8..])
    else {
        return (trimmed, Vec::new());
    };
    let mut ids = Vec::new();
    for id in list
        .split([',', ';', ' '])
        .map(|id| id.trim_matches(['`', '.', '*']))
    {
        if let Some(article) = guide::article(id)
            && !ids.contains(&article.id)
        {
            ids.push(article.id);
        }
    }
    (body.trim_end(), ids)
}

/// While a reply streams, the part to show: never a half-written
/// `Sources:` line.
pub fn draft(reply: &str) -> &str {
    let (body, last) = reply.rsplit_once('\n').unwrap_or(("", reply));
    let head: String = last
        .trim_start_matches(['*', '`', '_', ' '])
        .chars()
        .take(8)
        .collect::<String>()
        .to_lowercase();
    if !head.is_empty() && ("sources:".starts_with(&head) || head.starts_with("sources:")) {
        body.trim_end()
    } else {
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_carries_the_contents_and_the_articles_search_found() {
        let prompt = build("Can VibeHex record my Zoom calls?", &[], Room::Large);
        assert!(prompt.input.contains("<guide_contents>"));
        assert!(prompt.input.contains("topics/privacy: Privacy"));
        assert!(prompt.articles.len() <= 6);
        assert!(prompt.articles.iter().any(|id| id.contains("meetings")));
        for id in &prompt.articles {
            assert!(
                prompt
                    .input
                    .contains(&format!("<guide_article id=\"{id}\">"))
            );
        }
        assert!(
            prompt
                .input
                .ends_with("<question>\nCan VibeHex record my Zoom calls?\n</question>\n")
        );
        assert!(
            prompt
                .instructions
                .contains("language of the person's latest question")
        );
    }

    #[test]
    fn a_small_model_reads_three_articles_and_short_earlier_answers() {
        let long = "x".repeat(5_000);
        let earlier = [Turn {
            question: "first",
            answer: &long,
        }];
        let prompt = build("и на Windows?", &earlier, Room::Small);
        assert!(prompt.articles.len() <= 3);
        assert!(prompt.input.contains(&format!("{}…", "x".repeat(600))));
        assert!(!prompt.input.contains(&"x".repeat(601)));
    }

    #[test]
    fn only_the_newest_earlier_turns_go_back_in() {
        let earlier: Vec<Turn> = (0..10)
            .map(|_| Turn {
                question: "q",
                answer: "a",
            })
            .collect();
        let prompt = build("next", &earlier, Room::Large);
        assert_eq!(prompt.input.matches("<earlier_question>").count(), 8);
    }

    /// The prompt is the guide, the session, and the question: nothing a
    /// person said elsewhere in the app.
    #[test]
    fn the_prompt_holds_only_the_guide_the_session_and_the_question() {
        let earlier = [Turn {
            question: "earlier question",
            answer: "earlier answer",
        }];
        let prompt = build("new question", &earlier, Room::Large);
        let mut rest = prompt.input.clone();
        let contents_end = rest.find("</guide_contents>\n").unwrap() + "</guide_contents>\n".len();
        rest.replace_range(..contents_end, "");
        for id in &prompt.articles {
            rest = rest.replace(&excerpt(guide::article(id).unwrap()), "");
        }
        for text in ["earlier question", "earlier answer", "new question"] {
            rest = rest.replacen(text, "", 1);
        }
        for tag in ["earlier_question", "earlier_answer", "question"] {
            rest = rest
                .replace(&format!("<{tag}>"), "")
                .replace(&format!("</{tag}>"), "");
        }
        assert_eq!(rest.trim(), "");
    }

    /// assistant::* never reaches for what a person said or keeps.
    #[test]
    fn assistant_code_names_no_personal_data() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/assistant");
        let forbidden = [
            "crate::history",
            "meeting::requests",
            "meeting::transcript",
            "files::store",
            "keys::get",
        ];
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            let code = text.split("#[cfg(test)]").next().unwrap();
            for name in forbidden {
                assert!(!code.contains(name), "{} names {name}", path.display());
            }
        }
    }

    #[test]
    fn the_sources_line_becomes_known_ids_and_disappears() {
        let reply =
            "Turn on Labs › Meetings.\n\nSources: labs/meetings, topics/nope, `topics/meetings`";
        let (body, ids) = split_sources(reply);
        assert_eq!(body, "Turn on Labs › Meetings.");
        assert_eq!(ids, ["labs/meetings", "topics/meetings"]);
        assert_eq!(split_sources("No line here").1, Vec::<&str>::new());
        assert_eq!(split_sources("A.\n**Sources:** none"), ("A.", vec![]));
    }

    #[test]
    fn a_streaming_sources_line_never_shows() {
        assert_eq!(draft("Answer.\nSour"), "Answer.");
        assert_eq!(draft("Answer.\nSources: labs/"), "Answer.");
        assert_eq!(draft("Answer.\nSome more"), "Answer.\nSome more");
        assert_eq!(draft("Answer"), "Answer");
    }
}
