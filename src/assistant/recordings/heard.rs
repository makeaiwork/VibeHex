//! Whether a word was heard right. A Russian speech model writes an
//! English word it hears in Russian letters («веб», «монки», «нью сешн»),
//! and cleanup then writes it in English. That is the word heard right,
//! not a mishearing, and nothing Vocabulary would fix. A mishearing is a
//! heard form that isn't the word itself in Russian letters: «вольт» for
//! Bolt, «мамки» for Monkey.
//!
//! The two are told apart by their consonants: both are brought to one
//! rough alphabet (vowels dropped, `ph` and «ф» alike, `sh`, `-tion` and
//! «ш» alike, repeats collapsed) and compared, one letter apart allowed in
//! a long word, since English spelling is no guide to how a word sounds.

/// The consonants of `text` in the rough alphabet, spaces dropped.
fn key(text: &str) -> Vec<char> {
    let lower: Vec<char> = text.to_lowercase().chars().collect();
    let mut key: Vec<char> = Vec::new();
    let mut at = 0;
    let rest = |at: usize, with: &str| {
        let with: Vec<char> = with.chars().collect();
        lower.len() >= at + with.len() && lower[at..at + with.len()] == with[..]
    };
    let vowel = |c: char| "aeiouy".contains(c);
    while at < lower.len() {
        let c = lower[at];
        let before = at.checked_sub(1).map(|before| lower[before]);
        let after = lower.get(at + 1).copied();
        // What `c` (and what follows it) sounds like, and how far it runs.
        let (sound, run): (&str, usize) = match c {
            // English, the longest spelling first.
            _ if rest(at, "ssion") => ("Sn", 5),
            _ if rest(at, "tion") || rest(at, "sion") => ("Sn", 4),
            _ if rest(at, "sch") => ("sk", 3),
            _ if rest(at, "sh") => ("S", 2),
            _ if rest(at, "gh") => ("", 2),
            _ if at == 0 && rest(at, "kn") => ("n", 2),
            _ if at == 0 && rest(at, "wr") => ("r", 2),
            _ if rest(at, "ch") => ("C", 2),
            _ if rest(at, "ph") => ("f", 2),
            _ if rest(at, "th") => ("t", 2),
            _ if rest(at, "ck") => ("k", 2),
            _ if rest(at, "qu") => ("kv", 2),
            _ if rest(at, "wh") => ("v", 2),
            'c' if after.is_some_and(|after| "eiy".contains(after)) => ("s", 1),
            'c' | 'q' => ("k", 1),
            'x' => ("ks", 1),
            'j' => ("g", 1),
            'z' => ("s", 1),
            // `new`, `show`, `window`: a vowel there, `v` before one.
            'w' if before.is_some_and(vowel) && !after.is_some_and(vowel) => ("", 1),
            'w' => ("v", 1),
            'h' => ("", 1),
            'a'..='z' if vowel(c) => ("", 1),
            'a'..='z' => {
                key.push(c);
                at += 1;
                continue;
            }
            // Russian.
            _ if rest(at, "дж") => ("g", 2),
            'б' => ("b", 1),
            'в' => ("v", 1),
            'г' | 'ж' => ("g", 1),
            'д' => ("d", 1),
            'з' | 'с' => ("s", 1),
            'к' => ("k", 1),
            'л' => ("l", 1),
            'м' => ("m", 1),
            'н' => ("n", 1),
            'п' => ("p", 1),
            'р' => ("r", 1),
            'т' => ("t", 1),
            'ф' => ("f", 1),
            'ц' => ("ts", 1),
            'ч' => ("C", 1),
            'ш' | 'щ' => ("S", 1),
            // Vowels, х (an `h`), й, the signs, digits, marks.
            _ => ("", 1),
        };
        key.extend(sound.chars());
        at += run;
    }
    key.dedup();
    key
}

/// Whether `heard` is `word` itself: the same letters, or the word said
/// in Russian letters.
pub fn right(heard: &str, word: &str) -> bool {
    let flat = |text: &str| -> String {
        text.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    if flat(heard) == flat(word) {
        return true;
    }
    let (one, other) = (key(heard), key(word));
    let allowed = usize::from(one.len().min(other.len()) >= 4);
    super::terms::distance(&one, &other) <= allowed
}

/// How often `word` was misheard: the count of every heard form that
/// isn't the word itself.
pub fn misheard<'a>(word: &str, heard: impl IntoIterator<Item = (&'a str, usize)>) -> usize {
    heard
        .into_iter()
        .filter(|(heard, _)| !right(heard, word))
        .map(|(_, count)| count)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_in_russian_letters_is_heard_right() {
        // Heard, the word: the word itself, as a Russian model writes it.
        let right_ones = [
            ("веб", "Web"),
            ("монки", "Monkey"),
            ("манки", "Monkey"),
            ("хиру", "Hero"),
            ("нью сешн", "New Session"),
            ("нью сешен", "New Session"),
            ("виспер", "Whisper"),
            ("лайт", "Light"),
            ("ноу", "Know"),
            ("дженеретив", "Generative"),
            ("курсор", "Cursor"),
            ("промпт", "prompt"),
            ("шоу", "Show"),
            ("виндоу", "Window"),
            ("экшн", "Action"),
            ("фото", "Photo"),
            ("web", "Web"),
            ("new-session", "New Session"),
        ];
        for (heard, word) in right_ones {
            assert!(right(heard, word), "{heard} is {word}");
        }
    }

    #[test]
    fn another_word_is_a_mishearing() {
        let wrong = [
            ("вольт", "Bolt"),
            ("мамки", "Monkey"),
            ("вэп", "Web"),
            ("ной сешн", "Old Session"),
            ("арт", "Arc"),
            ("кот", "Code"),
            ("лайн", "Light"),
        ];
        for (heard, word) in wrong {
            assert!(!right(heard, word), "{heard} isn't {word}");
        }
        assert_eq!(
            misheard("Web", [("веб", 7), ("вэп", 2), ("уэп", 1)]),
            3,
            "the plain form doesn't count"
        );
    }
}
