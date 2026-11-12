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

#[cfg(test)]
mod tests {
    use super::*;

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
