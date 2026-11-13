//! Roughly how words sound, whichever alphabet they are written in, so
//! "докером" can be held against "Docker" and "barn" against "Night".
//!
//! A [`Key`] is a run of words as sounds, one byte each: Cyrillic by its
//! letters, Latin by a few English spelling habits (`ck`, `ph`, `x`, a
//! silent final `e`, …). It is crude on purpose: [`fit`] then compares two
//! keys with an edit distance where vowels are cheap, voiced and voiceless
//! pairs are near, and a Russian case ending on what was heard costs almost
//! nothing, and says how badly the worst word on each side was matched.

/// The sounds English `sh`, `ch`, and `j`/`zh` stand for.
const SH: u8 = b'S';
const CH: u8 = b'C';
const ZH: u8 = b'J';

/// What leaving out a case ending's sound costs.
const ENDING_COST: f32 = 0.15;
/// What leaving out the second of a sound written twice costs ("оллама").
const DOUBLED_COST: f32 = 0.3;
/// What leaving out a form's final "s" costs: a Russian speaker declines
/// the name instead ("в доках" for Docs).
const PLURAL_COST: f32 = 0.4;
/// The fewest sounds before that "s".
const MIN_PLURAL_STEM: usize = 3;
/// What a vowel for another vowel, or a consonant for one near it, costs.
const NEAR_COST: f32 = 0.3;
/// The most words a key tells apart; later ones count with the last.
const MAX_WORDS: usize = 12;

/// Russian case endings a heard form may carry ("кодексом", "в гитхабе"),
/// longest first.
const ENDINGS: &[&str] = &[
    "ами", "ями", "ов", "ев", "ом", "ем", "ам", "ям", "ах", "ях", "ой", "ей", "ую", "ию", "а", "у",
    "е", "ы", "и", "я", "ю",
];
/// The fewest letters a word keeps before an ending.
const MIN_STEM: usize = 3;

/// A run of words as sounds.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Key {
    sounds: Vec<u8>,
    /// The word each sound came from.
    words: Vec<u8>,
    /// What leaving each sound out costs.
    costs: Vec<f32>,
    /// Each word's sounds, summed by cost.
    weights: Vec<f32>,
}

impl Key {
    /// The key of `words`; with `endings`, their Russian case endings are
    /// cheap to leave out.
    pub(super) fn of<'a>(words: impl IntoIterator<Item = &'a str>, endings: bool) -> Self {
        let mut key = Self::default();
        for word in words {
            let index = key.weights.len().min(MAX_WORDS - 1);
            if index == key.weights.len() {
                key.weights.push(0.0);
            }
            key.push_word(word, index as u8, endings);
        }
        key
    }

    pub(super) fn is_empty(&self) -> bool {
        self.sounds.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.sounds.len()
    }

    /// The key's first `count` sounds as one word.
    pub(super) fn start(&self, count: usize) -> Self {
        let count = count.min(self.sounds.len());
        Self {
            sounds: self.sounds[..count].to_vec(),
            words: vec![0; count],
            costs: self.costs[..count].to_vec(),
            weights: vec![self.costs[..count].iter().sum()],
        }
    }

    pub(super) fn weight(&self) -> f32 {
        self.weights.iter().sum()
    }

    fn push_word(&mut self, word: &str, index: u8, endings: bool) {
        let letters: Vec<char> = word
            .chars()
            .flat_map(char::to_lowercase)
            .filter(|c| c.is_alphanumeric())
            .collect();
        let stem = if endings {
            stem_len(&letters)
        } else {
            letters.len()
        };
        let first = self.sounds.len();
        // A Latin letter on its own is said by its name ("X" in "UX").
        if let [letter] = letters[..]
            && let Some(name) = letter_name(letter)
        {
            for &sound in name {
                self.push(sound, first, index, false);
            }
            return;
        }
        let mut at = 0;
        while at < letters.len() {
            let c = letters[at];
            if c.is_ascii_lowercase() {
                let end = letters[at..]
                    .iter()
                    .position(|c| !c.is_ascii_lowercase())
                    .map_or(letters.len(), |length| at + length);
                self.push_latin(&letters[at..end], first, index);
                at = end;
            } else if c.is_ascii_digit() {
                self.push(c as u8, first, index, false);
                at += 1;
            } else if c == 'д' && letters.get(at + 1) == Some(&'ж') {
                self.push(ZH, first, index, at >= stem);
                at += 2;
            } else {
                for &sound in cyrillic(c) {
                    self.push(sound, first, index, at >= stem);
                }
                at += 1;
            }
        }
        if !endings
            && matches!(letters.last(), Some('s' | 'с'))
            && self.sounds.len() > first + MIN_PLURAL_STEM
            && self.sounds.last() == Some(&b's')
            && let Some(cost) = self.costs.last_mut()
        {
            self.weights[index as usize] -= *cost - PLURAL_COST;
            *cost = PLURAL_COST;
        }
    }

    /// Adds a sound to the word that starts at `first`; the second of a
    /// sound written twice is cheap to leave out.
    fn push(&mut self, sound: u8, first: usize, word: u8, ending: bool) {
        let doubled = self.sounds.len() > first && self.sounds.last() == Some(&sound);
        let cost = if ending {
            ENDING_COST
        } else if doubled {
            DOUBLED_COST.min(weight(sound))
        } else {
            weight(sound)
        };
        self.sounds.push(sound);
        self.words.push(word);
        self.costs.push(cost);
        self.weights[word as usize] += cost;
    }

    /// A Latin stretch by English spelling habits.
    fn push_latin(&mut self, letters: &[char], first: usize, word: u8) {
        let mut at = 0;
        while at < letters.len() {
            let rest = &letters[at..];
            let starts = |text: &str| {
                rest.len() >= text.len() && text.chars().zip(rest).all(|(a, b)| a == *b)
            };
            let (sounds, taken): (&[u8], usize) = if starts("tion") {
                (&[SH, b'n'], 4)
            } else if starts("igh") {
                (b"ai", 3)
            } else if starts("sch") {
                (b"sk", 3)
            } else if starts("sh") {
                (&[SH], 2)
            } else if starts("ch") {
                (&[CH], 2)
            } else if starts("ph") {
                (b"f", 2)
            } else if starts("th") {
                (b"t", 2)
            } else if starts("wh") {
                (b"v", 2)
            } else if starts("ck") {
                (b"k", 2)
            } else if starts("qu") {
                (b"kv", 2)
            } else if starts("ee") || starts("ea") {
                (b"i", 2)
            } else if starts("oo") {
                (b"u", 2)
            } else {
                let sounds: &[u8] = match rest[0] {
                    'c' if matches!(rest.get(1), Some('e' | 'i' | 'y')) => b"s",
                    'c' | 'q' => b"k",
                    'x' => b"ks",
                    'w' => b"v",
                    'y' => b"i",
                    'j' => &[ZH],
                    // A final "e" after a consonant is silent ("Vibe").
                    'e' if rest.len() == 1
                        && at >= 2
                        && !is_vowel_letter(letters[at - 1])
                        && letters[..at].iter().any(|c| is_vowel_letter(*c)) =>
                    {
                        b""
                    }
                    letter => {
                        self.push(letter as u8, first, word, false);
                        b""
                    }
                };
                (sounds, 1)
            };
            for &sound in sounds {
                self.push(sound, first, word, false);
            }
            at += taken;
        }
    }
}

/// How a Latin letter is named when said on its own.
fn letter_name(letter: char) -> Option<&'static [u8]> {
    Some(match letter {
        'a' => b"ei",
        'b' => b"bi",
        'c' => b"si",
        'd' => b"di",
        'e' => b"i",
        'f' => b"ef",
        'g' => &[ZH, b'i'],
        'h' => &[b'e', b'i', CH],
        'i' => b"ai",
        'j' => &[ZH, b'e', b'i'],
        'k' => b"kei",
        'l' => b"el",
        'm' => b"em",
        'n' => b"en",
        'o' => b"ou",
        'p' => b"pi",
        'q' => b"kiu",
        'r' => b"ar",
        's' => b"es",
        't' => b"ti",
        'u' => b"iu",
        'v' => b"vi",
        'w' => b"dablu",
        'x' => b"iks",
        'y' => b"uai",
        'z' => b"zed",
        _ => return None,
    })
}

fn is_vowel_letter(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

/// How many of a word's letters come before its case ending; all of them
/// when it has none or isn't Cyrillic.
pub(super) fn stem_len(letters: &[char]) -> usize {
    if !letters.iter().all(|c| ('а'..='я').contains(c) || *c == 'ё') {
        return letters.len();
    }
    for ending in ENDINGS {
        let length = ending.chars().count();
        if letters.len() >= length + MIN_STEM
            && ending
                .chars()
                .zip(&letters[letters.len() - length..])
                .all(|(a, b)| a == *b)
        {
            return letters.len() - length;
        }
    }
    letters.len()
}

/// A Cyrillic letter's sounds; nothing for a sign or another alphabet.
fn cyrillic(c: char) -> &'static [u8] {
    match c {
        'а' | 'я' => b"a",
        'б' => b"b",
        'в' => b"v",
        'г' | 'ґ' => b"g",
        'д' => b"d",
        'е' | 'э' | 'є' => b"e",
        'ё' | 'о' => b"o",
        'ж' => &[ZH],
        'з' => b"z",
        'и' | 'й' | 'ы' | 'і' | 'ї' => b"i",
        'к' => b"k",
        'л' => b"l",
        'м' => b"m",
        'н' => b"n",
        'п' => b"p",
        'р' => b"r",
        'с' => b"s",
        'т' => b"t",
        'у' | 'ю' => b"u",
        'ф' => b"f",
        'х' => b"h",
        'ц' => b"ts",
        'ч' => &[CH],
        'ш' | 'щ' => &[SH],
        _ => b"",
    }
}

fn is_vowel(sound: u8) -> bool {
    matches!(sound, b'a' | b'e' | b'i' | b'o' | b'u')
}

/// What leaving a sound out, or adding it, costs.
fn weight(sound: u8) -> f32 {
    match sound {
        sound if is_vowel(sound) => 0.5,
        b'h' => 0.7,
        _ => 1.0,
    }
}

/// Consonants a listener mixes up: voiced with voiceless, the hissing ones
/// with each other, and Russian "г" for English "h".
fn near(a: u8, b: u8) -> bool {
    const PAIRS: &[(u8, u8)] = &[
        (b'b', b'p'),
        (b'd', b't'),
        (b'g', b'k'),
        (b'v', b'f'),
        (b'z', b's'),
        (b'g', b'h'),
        (b'g', ZH),
        (b'z', ZH),
        (b's', SH),
        (ZH, SH),
        (ZH, CH),
        (SH, CH),
    ];
    PAIRS
        .iter()
        .any(|&(x, y)| (a == x && b == y) || (a == y && b == x))
}

fn swap(a: u8, b: u8) -> f32 {
    if a == b {
        0.0
    } else if (is_vowel(a) && is_vowel(b)) || near(a, b) {
        NEAR_COST
    } else {
        1.0
    }
}

/// How well what was heard fits a form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Fit {
    /// A whole consonant is added, missing, or another one; vowels and
    /// near consonants never count.
    pub consonant_off: bool,
    /// 1 for the same sounds, toward 0 as they differ.
    pub similarity: f32,
    /// The share of the worst-matched heard word that fits nothing in the
    /// form.
    pub heard_miss: f32,
    /// The share of the form's worst-matched word that nothing heard fits.
    pub form_miss: f32,
}

/// The best `fit` can come to for these two keys, from their lengths alone.
pub(super) fn fit_bound(heard: &Key, form: &Key) -> f32 {
    let (a, b) = (heard.weight(), form.weight());
    let longest = a.max(b);
    if longest <= 0.0 {
        return 0.0;
    }
    // Endings are nearly free to drop, so they may hide a longer heard word.
    1.0 - ((a - b).abs() / longest).min(1.0)
}

/// Holds `heard` against `form` sound by sound. `table` is scratch space.
pub(super) fn fit(heard: &Key, form: &Key, table: &mut Vec<f32>) -> Fit {
    let (rows, columns) = (heard.sounds.len(), form.sounds.len());
    let longest = heard.weight().max(form.weight());
    if rows == 0 || columns == 0 || longest <= 0.0 {
        return Fit {
            consonant_off: true,
            similarity: 0.0,
            heard_miss: 1.0,
            form_miss: 1.0,
        };
    }
    let width = columns + 1;
    table.clear();
    table.resize((rows + 1) * width, 0.0);
    for column in 1..=columns {
        table[column] = table[column - 1] + form.costs[column - 1];
    }
    for row in 1..=rows {
        table[row * width] = table[(row - 1) * width] + heard.costs[row - 1];
        for column in 1..=columns {
            let swapped = table[(row - 1) * width + column - 1]
                + swap(heard.sounds[row - 1], form.sounds[column - 1]);
            let dropped = table[(row - 1) * width + column] + heard.costs[row - 1];
            let added = table[row * width + column - 1] + form.costs[column - 1];
            table[row * width + column] = swapped.min(dropped).min(added);
        }
    }
    let distance = table[rows * width + columns];
    // Walk back, charging each step to the words it touched.
    let mut heard_missed = [0.0_f32; MAX_WORDS];
    let mut form_missed = [0.0_f32; MAX_WORDS];
    let mut consonant_off = false;
    let (mut row, mut column) = (rows, columns);
    while row > 0 || column > 0 {
        let here = table[row * width + column];
        if row > 0 && column > 0 {
            let cost = swap(heard.sounds[row - 1], form.sounds[column - 1]);
            if (table[(row - 1) * width + column - 1] + cost - here).abs() < 1e-4 {
                heard_missed[heard.words[row - 1] as usize] += cost;
                form_missed[form.words[column - 1] as usize] += cost;
                consonant_off |= cost >= 1.0;
                row -= 1;
                column -= 1;
                continue;
            }
        }
        if row > 0 && (table[(row - 1) * width + column] + heard.costs[row - 1] - here).abs() < 1e-4
        {
            heard_missed[heard.words[row - 1] as usize] += heard.costs[row - 1];
            consonant_off |= heard.costs[row - 1] >= 1.0;
            row -= 1;
        } else {
            form_missed[form.words[column - 1] as usize] += form.costs[column - 1];
            consonant_off |= form.costs[column - 1] >= 1.0;
            column -= 1;
        }
    }
    let worst = |missed: &[f32; MAX_WORDS], key: &Key| {
        key.weights
            .iter()
            .zip(missed)
            .filter(|(weight, _)| **weight > 0.0)
            .map(|(weight, missed)| missed / weight)
            .fold(0.0_f32, f32::max)
    };
    Fit {
        consonant_off,
        similarity: (1.0 - distance / longest).max(0.0),
        heard_miss: worst(&heard_missed, heard),
        form_miss: worst(&form_missed, form),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sounds(text: &str) -> String {
        String::from_utf8(Key::of(text.split_whitespace(), false).sounds).unwrap()
    }

    fn similarity(heard: &str, form: &str) -> f32 {
        fit(
            &Key::of(heard.split_whitespace(), true),
            &Key::of(form.split_whitespace(), false),
            &mut Vec::new(),
        )
        .similarity
    }

    #[test]
    fn both_alphabets_come_to_the_same_sounds() {
        assert_eq!(sounds("Codex"), sounds("кодекс"));
        assert_eq!(sounds("Docker"), sounds("докер"));
        assert_eq!(sounds("Whatsap"), sounds("ватсап"));
        assert_eq!(sounds("code"), sounds("код"));
        assert_eq!(sounds("Open Code"), sounds("опенкод"));
    }

    #[test]
    fn a_hearing_of_a_name_fits_it_and_another_word_does_not() {
        assert!(similarity("клод", "Claude") > 0.75);
        assert!(similarity("плейврайт", "Playwright") > 0.7);
        assert!(similarity("гитхабом", "GitHub") > 0.7);
        assert!(similarity("сентри", "Sentry") > 0.9);
        assert!(similarity("уборка", "cleanup") < 0.5);
        assert!(similarity("код", "Codex") < 0.6);
        assert!(similarity("кума", "Docker") < 0.5);
    }

    #[test]
    fn a_case_ending_on_what_was_heard_costs_almost_nothing() {
        assert!(similarity("кодексом", "Codex") > 0.9);
        assert!(similarity("докере", "Docker") > 0.9);
        // The form's own letters are never an ending.
        assert!(similarity("докер", "Dockerama") < 0.8);
    }

    #[test]
    fn the_worst_word_on_each_side_is_told() {
        let mut table = Vec::new();
        let form = Key::of(["Night", "shift"], false);
        let close = fit(&Key::of(["найт", "шифт"], true), &form, &mut table);
        assert!(close.heard_miss < 0.3 && close.form_miss < 0.3, "{close:?}");
        // "barn" is not "Night", however well "shift" fits.
        let other = fit(&Key::of(["barn", "shift"], true), &form, &mut table);
        assert!(other.heard_miss > 0.5, "{other:?}");
        // One word heard leaves the form's other word unsaid.
        let half = fit(&Key::of(["найт"], true), &form, &mut table);
        assert!(half.form_miss > 0.5, "{half:?}");
    }
}
