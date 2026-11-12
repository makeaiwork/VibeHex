//! Whether a question is about what dictation's cleanup or recognition
//! does to the person's words: their cleanup instructions ("is it my
//! prompt?", «в промпте есть что-то?»), a word written as another («пишет
//! AI cleanup вместо AI», "replaces X with Y"), or terms it gets wrong. Read
//! from the words alone, in the languages the app speaks; what the prompt
//! then carries (`assistant::personal`) is the person's instructions and
//! the Vocabulary entries the words name.

/// What a question about cleanup needs to be answered for this person.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Wants {
    /// Their cleanup instructions and modes' replacements.
    pub instructions: bool,
    /// The Vocabulary entries the words name.
    pub vocabulary: bool,
}

impl Wants {
    #[cfg(test)]
    fn any(self) -> bool {
        self.instructions || self.vocabulary
    }

    pub fn or(self, other: Self) -> Self {
        Self {
            instructions: self.instructions || other.instructions,
            vocabulary: self.vocabulary || other.vocabulary,
        }
    }
}

/// The instructions: prompt, instructions.
const PROMPT: &[&str] = &[
    "prompt",
    "промпт",
    "инструкци",
    "інструкці",
    "instruction",
    "instrukcj",
    "anweisung",
    "instrucci",
    "consigne",
    "instruç",
    "提示词",
    "指令",
    "プロンプト",
    "指示",
];

/// Cleanup itself.
const CLEANUP: &[&str] = &[
    "cleanup",
    "clean up",
    "clean-up",
    "cleans up",
    "клинап",
    "очистк",
    "очища",
    "чистит",
    "чистка",
    "редактур",
    "постобработ",
    "очищен",
    "bereinig",
    "limpieza",
    "limpia",
    "nettoyage",
    "nettoie",
    "limpeza",
    "porządk",
    "整理",
    "清理",
    "クリーンアップ",
    "rewrite",
    "переписыва",
];

/// One word written as another, or added.
const REPLACED: &[&str] = &[
    "instead of",
    "replaces",
    "replaced",
    "turns into",
    "adds",
    "вместо",
    "замен",
    "дописыва",
    "добавляет",
    "подставля",
    "превраща",
    "замість",
    "statt",
    "ersetzt",
    "en lugar de",
    "en vez de",
    "reemplaza",
    "au lieu de",
    "remplace",
    "em vez de",
    "substitui",
    "zamiast",
    "zamienia",
    "而不是",
    "替换",
    "代わりに",
    "置き換",
];

/// Recognition hearing a word wrong, or Vocabulary.
const HEARD: &[&str] = &[
    "распозна",
    "слышит",
    "recogni",
    "mishear",
    "heard as",
    "transcribed as",
    "comes out as",
    "словар",
    "vocabulary",
    "sounds like",
    "sounds-like",
    "термин",
    "розпізна",
    "словник",
    "erkannt",
    "wortschatz",
    "vocabulario",
    "reconoc",
    "vocabulaire",
    "reconna",
    "vocabulário",
    "reconhec",
    "słownik",
    "rozpozna",
    "识别",
    "词汇",
    "認識",
    "語彙",
];

fn names(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

/// What `question` needs of the person's cleanup setup, if anything.
pub fn wants(question: &str) -> Wants {
    let text = question.to_lowercase();
    let prompt = names(&text, PROMPT);
    let cleanup = names(&text, CLEANUP);
    let replaced = names(&text, REPLACED);
    let heard = names(&text, HEARD);
    Wants {
        instructions: prompt || cleanup || replaced,
        vocabulary: cleanup || replaced || heard,
    }
}

/// Whether `question` speaks of the cleanup instructions themselves
/// ("what does my prompt say?", «что в моих инструкциях?»): what an agent
/// turn is told to read first.
pub fn instructions(question: &str) -> bool {
    names(&question.to_lowercase(), PROMPT)
}

/// "X instead of Y": the words that follow the form written.
const INSTEAD: &[&str] = &[
    "instead of",
    "rather than",
    "вместо",
    "а не ",
    "замість",
    "statt",
    "anstatt",
    "en lugar de",
    "en vez de",
    "au lieu de",
    "em vez de",
    "zamiast",
];

/// Writing, the verb before the form written ("it writes X").
const WRITES: &[&str] = &[
    "write",
    "wrote",
    "writing",
    "types",
    "puts",
    "inserts",
    "gives",
    "turns into",
    "comes out as",
    "пишет",
    "написал",
    "напечатал",
    "вставля",
    "выда",
    "получает",
    "получил",
    "превраща",
    "пише",
    "schreibt",
    "geschrieben",
    "escribe",
    "écrit",
    "escreve",
    "pisze",
];

/// The most words a form written is read as, without quotes.
const FORM_WORDS: usize = 3;

/// The form dictation wrote, when `question` asks why it wrote one thing
/// instead of another («почему пишет AI cleanup вместо AI», "why does it
/// write Cuba Nettie instead of Kubernetes"), in lowercase: what the
/// History search then looks for in what was written. A quoted form
/// counts first; otherwise the words after the writing verb, up to the
/// "instead of".
pub fn written(question: &str) -> Option<String> {
    let text = question.to_lowercase();
    let (at, marker) = INSTEAD
        .iter()
        .filter_map(|marker| text.find(marker).map(|at| (at, *marker)))
        .min_by_key(|(at, _)| *at)?;
    let (before, after) = (&text[..at], &text[at + marker.len()..]);
    let form = quoted(before)
        .last()
        .cloned()
        .or_else(|| after_verb(before).map(|words| last_words(&words, FORM_WORDS)))
        // «вместо AI пишет AI cleanup»: the form after the verb.
        .or_else(|| after_verb(after).map(|words| first_words(&words, FORM_WORDS)))
        .or_else(|| Some(last_words(before, 2)))?;
    let form = form.trim().to_owned();
    (form.chars().filter(|c| c.is_alphanumeric()).count() >= 2).then_some(form)
}

/// The quoted parts of `text`, in order.
fn quoted(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    for (open, close) in [('«', '»'), ('"', '"'), ('“', '”'), ('„', '“')] {
        let mut rest = text;
        while let Some(start) = rest.find(open) {
            let tail = &rest[start + open.len_utf8()..];
            let Some(end) = tail.find(close) else {
                break;
            };
            parts.push((text.len() - rest.len() + start, tail[..end].to_owned()));
            rest = &tail[end + close.len_utf8()..];
        }
    }
    parts.sort_by_key(|(at, _)| *at);
    parts.into_iter().map(|(_, part)| part).collect()
}

/// What follows the last writing verb in `text`, to its clause's end.
fn after_verb(text: &str) -> Option<String> {
    let (at, verb) = WRITES
        .iter()
        .filter_map(|verb| text.rfind(verb).map(|at| (at, *verb)))
        .max_by_key(|(at, _)| *at)?;
    // The rest of the verb's word ("writes", "пишется").
    let rest = &text[at + verb.len()..];
    let rest = rest.trim_start_matches(|c: char| c.is_alphanumeric());
    let clause = rest
        .split(['.', ',', ';', '?', '!', '\n'])
        .next()
        .unwrap_or_default();
    let words: Vec<&str> = clause
        .split_whitespace()
        .filter(|word| !FILLER.contains(word))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// Words between a verb and its form: "it writes it as X".
const FILLER: &[&str] = &[
    "it", "as", "me", "out", "мне", "у", "меня", "es", "il", "le",
];

fn last_words(text: &str, count: usize) -> String {
    let clause = text
        .rsplit(['.', ',', ';', '?', '!', '\n'])
        .next()
        .unwrap_or(text);
    let words: Vec<&str> = clause.split_whitespace().collect();
    words[words.len().saturating_sub(count)..].join(" ")
}

fn first_words(text: &str, count: usize) -> String {
    text.split_whitespace()
        .take(count)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_form_written_is_read_from_instead_of() {
        assert_eq!(
            written("Why does it write Cuba Nettie instead of Kubernetes?").as_deref(),
            Some("cuba nettie")
        );
        assert_eq!(
            written("Почему пишет AI cleanup вместо AI?").as_deref(),
            Some("ai cleanup")
        );
        assert_eq!(
            written("Почему вместо AI пишет AI cleanup").as_deref(),
            Some("ai cleanup")
        );
        assert_eq!(
            written("Why do I get \"Hex labs\" instead of \"VibeHex\"?").as_deref(),
            Some("hex labs")
        );
        assert_eq!(
            written("Warum schreibt es Kuba statt Kubernetes?").as_deref(),
            Some("kuba")
        );
        assert_eq!(written("How do I turn on meetings?"), None);
    }

    #[test]
    fn cleanup_questions_want_the_instructions_and_vocabulary() {
        let owner = wants(
            "можешь проверить, в промпте есть ли какие-то штуки, которые могли привести к этому, для AI cleanup",
        );
        assert!(owner.instructions && owner.vocabulary);
        let heard = wants(
            "у меня бывает, продиктовал слово, и оно распозналось как часть, например AI cleanup. Если я просто скажу AI, он может AI cleanup вставить",
        );
        assert!(heard.instructions && heard.vocabulary);
        assert!(wants("Why does it write Kubernetes instead of Cuba Nettie?").any());
        assert!(wants("Warum ersetzt die Bereinigung meine Wörter?").instructions);
        assert!(wants("Is my prompt making it add commas?").instructions);
        assert_eq!(wants("Can VibeHex record my Zoom calls?"), Wants::default());
        assert_eq!(wants("Как включить встречи?"), Wants::default());
    }
}
