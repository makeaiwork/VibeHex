//! How speech recognition may write an English name someone says in
//! Russian: «вайб хекс», «вайпхекс» for VibeHex. A search for the name
//! looks for these too, so a dictation that says it in Cyrillic is found.
//! Small and deterministic: a word split at its capitals, a rough
//! letter-by-letter reading of how English sounds, and the last consonant
//! of each part voiced or not (б/п, г/к, д/т, з/с, в/ф), the way Russian
//! speech blurs it.

/// The most forms one word gets.
const MOST: usize = 16;

/// The ways `written`, a Latin word of three or more letters as the
/// question has it, may come out of speech recognition: its capitalised
/// parts as words ("vibe hex"), and its sound in Cyrillic, joined and
/// apart (вайбхекс, вайб хекс, вайпхекс). Lowercase, without the word
/// itself; none for a word that isn't Latin.
pub fn forms(written: &str) -> Vec<String> {
    let letters = written.chars().filter(char::is_ascii_alphabetic).count();
    if letters < 3 || !written.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Vec::new();
    }
    let parts = parts(written);
    let mut forms: Vec<String> = Vec::new();
    let mut add = |form: String| {
        if !form.is_empty() && form != written.to_lowercase() && !forms.contains(&form) {
            forms.push(form);
        }
    };
    if parts.len() > 1 {
        add(parts.join(" "));
    }
    // Each part's sound, then with its last consonant voiced or not.
    let sounds: Vec<Vec<String>> = parts
        .iter()
        .map(|part| {
            let sound = sounds(part);
            let mut variants = vec![sound.clone()];
            if parts.len() <= 3
                && let Some(other) = revoiced(&sound)
            {
                variants.push(other);
            }
            variants
        })
        .collect();
    let mut combos: Vec<Vec<&str>> = vec![Vec::new()];
    for variants in &sounds {
        combos = combos
            .iter()
            .flat_map(|combo| {
                variants.iter().map(move |variant| {
                    let mut next = combo.clone();
                    next.push(variant.as_str());
                    next
                })
            })
            .collect();
    }
    for combo in combos {
        add(combo.concat());
        if combo.len() > 1 {
            add(combo.join(" "));
        }
    }
    forms.truncate(MOST);
    forms
}

/// `VibeHex` → `vibe`, `hex`; `HTTPServer` → `http`, `server`; lowercase.
fn parts(written: &str) -> Vec<String> {
    let chars: Vec<char> = written.chars().collect();
    let mut parts: Vec<String> = Vec::new();
    let mut part = String::new();
    for (index, &c) in chars.iter().enumerate() {
        let previous = index.checked_sub(1).map(|at| chars[at]);
        let next = chars.get(index + 1);
        let boundary = match previous {
            Some(previous) => {
                (c.is_ascii_uppercase() && previous.is_ascii_lowercase())
                    || (c.is_ascii_uppercase()
                        && previous.is_ascii_uppercase()
                        && next.is_some_and(char::is_ascii_lowercase))
                    || (c.is_ascii_digit() != previous.is_ascii_digit())
            }
            None => false,
        };
        if boundary && !part.is_empty() {
            parts.push(std::mem::take(&mut part));
        }
        part.push(c.to_ascii_lowercase());
    }
    if !part.is_empty() {
        parts.push(part);
    }
    parts
}

fn vowel(c: char) -> bool {
    "aeiouy".contains(c)
}

/// How a lowercase English word sounds, written in Cyrillic: «вайб» for
/// `vibe`, «хекс» for `hex`, «хаб» for `hub`.
fn sounds(word: &str) -> String {
    let w: Vec<char> = word.chars().collect();
    let n = w.len();
    // A last `e` after a consonant is silent ("code", "apple"); after one
    // consonant it makes the vowel before say its name ("vibe" → вайб).
    let silent = n > 3 && w[n - 1] == 'e' && !vowel(w[n - 2]);
    let long = (silent && vowel(w[n - 3]) && (n < 4 || !vowel(w[n - 4]))).then(|| n - 3);
    let mut out = String::new();
    let mut i = 0;
    while i < n {
        let c = w[i];
        let next = w.get(i + 1).copied();
        let after = w.get(i + 2).copied();
        if silent && i == n - 1 {
            break;
        }
        let pair = match (c, next) {
            ('s', Some('h')) => Some("ш"),
            ('c', Some('h')) => Some("ч"),
            ('t', Some('h')) => Some("т"),
            ('p', Some('h')) => Some("ф"),
            ('c', Some('k')) => Some("к"),
            ('q', Some('u')) => Some("кв"),
            ('o', Some('o')) => Some("у"),
            ('e', Some('e' | 'a')) => Some("и"),
            ('a', Some('i' | 'y')) | ('e', Some('y' | 'i')) => Some("ей"),
            ('o', Some('u')) => Some("ау"),
            ('o', Some('w')) => Some("оу"),
            ('o', Some('a')) | ('a', Some('u' | 'w')) => Some("о"),
            ('w', Some('h')) => Some("в"),
            ('k', Some('n')) if i == 0 => Some("н"),
            ('g', Some('h')) => Some(""),
            _ => None,
        };
        if let Some(pair) = pair {
            out.push_str(pair);
            i += 2;
            continue;
        }
        let previous = i.checked_sub(1).map(|at| w[at]);
        // A doubled consonant is said once.
        if previous == Some(c) && !vowel(c) {
            i += 1;
            continue;
        }
        let closed = next.is_some_and(|next| !vowel(next)) && after.is_none_or(|a| !vowel(a));
        let sound = match c {
            'a' | 'e' | 'i' | 'o' | 'u' | 'y' if long == Some(i) => match c {
                'a' => "ей",
                'e' => "и",
                'i' | 'y' => "ай",
                'u' if matches!(previous, Some('r' | 'l' | 'j')) => "у",
                'u' => "ю",
                _ => "о",
            },
            'a' => "а",
            'e' if i == 0 => "э",
            'e' => "е",
            'i' => "и",
            'o' => "о",
            'u' if closed => "а",
            'u' => "у",
            'y' if previous.is_none_or(vowel) && next.is_some_and(vowel) => "й",
            'y' if previous.is_some_and(vowel) => "й",
            'y' => "и",
            'b' => "б",
            'c' if next.is_some_and(|next| "eiy".contains(next)) => "с",
            'c' | 'k' | 'q' => "к",
            'd' => "д",
            'f' => "ф",
            'g' => "г",
            'h' => "х",
            'j' => "дж",
            'l' => "л",
            'm' => "м",
            'n' => "н",
            'p' => "п",
            'r' => "р",
            's' => "с",
            't' => "т",
            'v' | 'w' => "в",
            'x' => "кс",
            'z' => "з",
            _ => {
                out.push(c);
                i += 1;
                continue;
            }
        };
        out.push_str(sound);
        i += 1;
    }
    out
}

/// `sound` with its last consonant voiced or voiceless the other way:
/// «вайб» → «вайп», «код» → «кот»; none when it ends otherwise.
fn revoiced(sound: &str) -> Option<String> {
    const PAIRS: &[(char, char)] = &[
        ('б', 'п'),
        ('г', 'к'),
        ('д', 'т'),
        ('з', 'с'),
        ('в', 'ф'),
        ('ж', 'ш'),
    ];
    let last = sound.chars().last()?;
    let other = PAIRS.iter().find_map(|&(voiced, voiceless)| {
        if last == voiced {
            Some(voiceless)
        } else if last == voiceless {
            Some(voiced)
        } else {
            None
        }
    })?;
    let mut out: String = sound.chars().take(sound.chars().count() - 1).collect();
    out.push(other);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_comes_apart_and_sounds_in_cyrillic() {
        let forms = forms("VibeHex");
        for form in ["vibe hex", "вайбхекс", "вайб хекс", "вайпхекс", "вайп хекс"]
        {
            assert!(forms.contains(&form.to_owned()), "{form} in {forms:?}");
        }
        assert!(!forms.contains(&"vibehex".to_owned()));
        assert!(forms.len() <= MOST);
    }

    #[test]
    fn english_sounds_the_way_russian_hears_it() {
        for (word, sound) in [
            ("vibe", "вайб"),
            ("hex", "хекс"),
            ("hub", "хаб"),
            ("code", "код"),
            ("rust", "раст"),
            ("slack", "слак"),
            ("cursor", "карсор"),
            ("grafana", "графана"),
            ("figma", "фигма"),
        ] {
            assert_eq!(sounds(word), sound, "{word}");
        }
        assert_eq!(parts("GitHub"), ["git", "hub"]);
        assert_eq!(parts("HTTPServer"), ["http", "server"]);
        assert_eq!(parts("GPT4"), ["gpt", "4"]);
        assert!(forms("GitHub").contains(&"гитхаб".to_owned()));
    }

    #[test]
    fn short_or_cyrillic_words_get_none() {
        assert!(forms("Go").is_empty());
        assert!(forms("бюджет").is_empty());
        assert!(forms("e-mail").is_empty());
    }
}
