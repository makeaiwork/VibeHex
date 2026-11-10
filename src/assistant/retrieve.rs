//! Finds the guide articles a question is about, for models too small to
//! read the whole guide: BM25 over each article's title, "Asked as"
//! phrasings, headings, settings keys, and body. Questions come in English
//! or Russian; the guide is English, so Russian finds its way through the
//! Russian "Asked as" lines and a short glossary of the app's own words.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::guide::{self, Article};

const K1: f32 = 1.2;
const B: f32 = 0.75;

/// How much a word counts where it appears.
const TITLE: f32 = 3.0;
const ASKED_AS: f32 = 3.0;
const HEADING: f32 = 2.0;
const KEY: f32 = 2.0;
const BODY: f32 = 1.0;

/// Russian words people use for the app's English ones; a question's
/// Russian word also searches for these.
const GLOSSARY: &[(&str, &str)] = &[
    ("встреч", "meeting call"),
    ("созвон", "meeting call zoom"),
    ("звонк", "call meeting"),
    ("запис", "record recording"),
    ("словар", "vocabulary"),
    ("термин", "vocabulary term"),
    ("модел", "model"),
    ("распозн", "speech model transcribe"),
    ("диктов", "dictation dictate"),
    ("надикт", "dictation dictate"),
    ("вставк", "paste"),
    ("вставл", "paste"),
    ("буфер", "clipboard"),
    ("клавиш", "shortcut key"),
    ("сочетан", "shortcut"),
    ("горяч", "shortcut"),
    ("мыш", "mouse"),
    ("истори", "history"),
    ("конспек", "notes summary"),
    ("заметк", "notes"),
    ("субтит", "subtitle srt"),
    ("глав", "chapters"),
    ("файл", "file files"),
    ("лекци", "file recording"),
    ("микроф", "microphone"),
    ("шум", "noise voice filter"),
    ("пауз", "pause"),
    ("интерн", "online offline privacy"),
    ("облак", "cloud privacy"),
    ("приват", "privacy"),
    ("памят", "memory unload idle"),
    ("обнов", "update"),
    ("язык", "language"),
    ("раскладк", "keyboard layout language"),
    ("пробел", "space spacing"),
    ("строк", "line newline"),
    ("скриншот", "screenshot annotate"),
    ("пометк", "annotate"),
    ("чистк", "cleanup"),
    ("почист", "cleanup"),
    ("лимит", "limit plan usage"),
    ("панел", "sidebar"),
    ("значок", "icon menu bar"),
    ("иконк", "icon menu bar"),
    ("слипа", "spacing space"),
    ("статист", "stats"),
    ("аудио", "audio recording keep"),
    ("прослуш", "listen playback"),
    ("собеседн", "other side system audio"),
    ("вопрос", "questions ask"),
    ("держ", "hold"),
    ("нажим", "tap press"),
    ("нажат", "tap press"),
    ("вернут", "history paste last"),
    ("не встав", "nothing pasted troubleshooting"),
    ("не появ", "nothing pasted troubleshooting"),
    ("не работ", "troubleshooting"),
    ("видеть", "show live"),
    ("пока говор", "live text speaking"),
    ("поменя", "change"),
    ("смени", "change"),
];

/// The articles that best match `question`, best first, at most `limit`.
pub fn search(question: &str, limit: usize) -> Vec<&'static Article> {
    let index = index();
    let mut terms = words(question);
    terms.extend(glossary_terms(question));
    terms.sort();
    terms.dedup();
    let mut scored: Vec<(f32, usize)> = index
        .documents
        .iter()
        .enumerate()
        .map(|(position, document)| (index.score(document, &terms), position))
        .filter(|(score, _)| *score > 0.0)
        .collect();
    scored.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, position)| &guide::articles()[position])
        .collect()
}

struct Document {
    weights: HashMap<String, f32>,
    length: f32,
}

struct Index {
    documents: Vec<Document>,
    frequency: HashMap<String, usize>,
    average: f32,
}

impl Index {
    fn score(&self, document: &Document, terms: &[String]) -> f32 {
        let count = self.documents.len() as f32;
        terms
            .iter()
            .filter_map(|term| {
                let weight = *document.weights.get(term)?;
                let frequency = *self.frequency.get(term)? as f32;
                let idf = ((count - frequency + 0.5) / (frequency + 0.5) + 1.0).ln();
                let norm = K1 * (1.0 - B + B * document.length / self.average);
                Some(idf * weight * (K1 + 1.0) / (weight + norm))
            })
            .sum()
    }
}

fn index() -> &'static Index {
    static INDEX: OnceLock<Index> = OnceLock::new();
    INDEX.get_or_init(|| {
        let documents: Vec<Document> = guide::articles().iter().map(document).collect();
        let mut frequency = HashMap::new();
        for document in &documents {
            for term in document.weights.keys() {
                *frequency.entry(term.clone()).or_insert(0) += 1;
            }
        }
        let average = documents
            .iter()
            .map(|document| document.length)
            .sum::<f32>()
            / documents.len().max(1) as f32;
        Index {
            documents,
            frequency,
            average,
        }
    })
}

fn document(article: &Article) -> Document {
    let mut weights = HashMap::new();
    let mut add = |text: &str, weight: f32| {
        for word in words(text) {
            *weights.entry(word).or_insert(0.0) += weight;
        }
    };
    add(&article.title, TITLE);
    add(&article.asked_as, ASKED_AS);
    add(&article.id.replace(['/', '-'], " "), KEY);
    for key in &article.keys {
        add(&key.replace(['.', '_'], " "), KEY);
    }
    for line in article.body.lines() {
        let weight = if line.starts_with('#') { HEADING } else { BODY };
        add(line, weight);
    }
    let length = weights.values().sum();
    Document { weights, length }
}

/// A text's searchable words: lowercase, stemmed, without the commonest.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace('ё', "е")
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().count() > 1 && !STOP.contains(word))
        .map(stem)
        .collect()
}

/// The glossary's English words for a question's Russian ones.
fn glossary_terms(question: &str) -> Vec<String> {
    let question = question.to_lowercase().replace('ё', "е");
    GLOSSARY
        .iter()
        .filter(|(russian, _)| question.contains(russian))
        .flat_map(|(_, english)| words(english))
        .collect()
}

/// Russian words keep their first five letters, which is near enough to a
/// stem for search; English loses its commonest endings.
fn stem(word: &str) -> String {
    if word.chars().any(|character| matches!(character, 'а'..='я')) {
        return word.chars().take(5).collect();
    }
    if word.len() <= 4 || word.bytes().any(|byte| byte.is_ascii_digit()) {
        return word.to_owned();
    }
    for suffix in ["ings", "ing", "ied", "ies", "ed", "es", "ly", "s"] {
        if let Some(stem) = word.strip_suffix(suffix)
            && stem.len() >= 3
            && !(suffix == "s" && (word.ends_with("ss") || word.ends_with("us")))
        {
            return stem.to_owned();
        }
    }
    word.to_owned()
}

const STOP: &[&str] = &[
    "the",
    "and",
    "for",
    "you",
    "your",
    "can",
    "how",
    "what",
    "with",
    "does",
    "this",
    "that",
    "it",
    "is",
    "to",
    "of",
    "in",
    "on",
    "or",
    "an",
    "be",
    "do",
    "my",
    "me",
    "are",
    "when",
    "from",
    "as",
    "at",
    "by",
    "if",
    "so",
    "not",
    "no",
    "yes",
    "its",
    "it's",
    "into",
    "out",
    "как",
    "что",
    "это",
    "можно",
    "ли",
    "на",
    "не",
    "по",
    "из",
    "за",
    "от",
    "до",
    "все",
    "чтобы",
    "мне",
    "меня",
    "мой",
    "моя",
    "мои",
    "или",
    "есть",
    "так",
    "уже",
    "его",
    "она",
    "они",
    "вы",
    "вас",
    "там",
    "тут",
    "при",
    "для",
    "без",
    "сам",
    "сами",
    "только",
    "всё",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Deserialize)]
    struct Question {
        id: String,
        q: String,
        articles: Vec<String>,
        #[serde(default)]
        web: bool,
    }

    fn questions() -> Vec<Question> {
        include_str!("../../tests/fixtures/assistant/questions.jsonl")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// The eval set's answers live in the articles it names; a small model
    /// sees the top three, so one of them has to be there.
    #[test]
    fn the_eval_questions_find_their_articles_in_the_top_three() {
        let questions: Vec<Question> = questions().into_iter().filter(|q| !q.web).collect();
        let mut misses = Vec::new();
        for question in &questions {
            let found: Vec<&str> = search(&question.q, 3).iter().map(|a| a.id).collect();
            if !question
                .articles
                .iter()
                .any(|id| found.contains(&id.as_str()))
            {
                misses.push(format!(
                    "{}: wanted {:?}, got {:?}",
                    question.id, question.articles, found
                ));
            }
        }
        let hits = questions.len() - misses.len();
        assert!(
            hits * 100 >= questions.len() * 90,
            "{hits}/{} found:\n{}",
            questions.len(),
            misses.join("\n")
        );
    }

    #[test]
    fn every_eval_article_exists() {
        for question in questions() {
            for id in &question.articles {
                assert!(guide::article(id).is_some(), "{}: no {id}", question.id);
            }
        }
    }

    #[test]
    fn english_and_russian_find_the_same_article() {
        assert_eq!(
            search("my dictation sticks to the previous word", 1)[0].id,
            "labs/smart-spacing"
        );
        assert_eq!(
            search("текст слипается с предыдущим словом", 1)[0].id,
            "labs/smart-spacing"
        );
    }

    #[test]
    fn stems_meet_halfway() {
        assert_eq!(stem("recordings"), stem("recording"));
        assert_eq!(stem("встречи"), stem("встречу"));
        assert_eq!(stem("class"), "class");
    }
}
