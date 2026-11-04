//! What transcription heard where AI cleanup wrote something else, for Labs ›
//! Add to vocabulary: selecting a word cleanup changed ("Directus") finds the
//! spelling the speech model wrote ("Динкдан"), the one Vocabulary needs.
//!
//! The transcript and the cleaned text are aligned word by word (their
//! longest common subsequence); the words both share anchor the alignment,
//! and what the transcript had between the anchors around the selection is
//! what cleanup replaced.

/// How many of the newest History entries are looked through.
const RECENT: usize = 100;
/// The largest alignment table; a longer pair aligns a window around the
/// selection instead.
const MAX_CELLS: usize = 2_000_000;
/// Words either side of the selection a windowed alignment keeps.
const WINDOW: usize = 60;
/// Words either side of the selection compared with an entry's.
const FIT: usize = 6;

/// The transcript's spelling of `selected` where AI cleanup wrote it, from
/// the History entry it most likely came from: the one whose cleaned text
/// has the words `around` it (the text before and after the selection, when
/// the app says), else the newest that changed it, passing over spellings
/// Vocabulary already lists (those are handled; the new one is what's
/// wanted). `None` when no entry changed it.
pub(crate) fn from_history(selected: &str, around: Option<&(String, String)>) -> Option<String> {
    let path = crate::app_paths::support_dir().ok()?.join("history.json");
    let mut entries = crate::history::read_snapshot(&path).ok()?;
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
    let pairs: Vec<(&str, &str)> = entries
        .iter()
        .take(RECENT)
        .filter(|entry| entry.raw_text != entry.final_text)
        .map(|entry| (entry.raw_text.as_str(), entry.final_text.as_str()))
        .collect();
    let listed = listed_spellings(&crate::dictation::vocabulary::entries());
    best(selected, around, &pairs, &listed)
}

/// What `from_history` picks among `pairs` of transcript and cleaned text,
/// newest first.
fn best(
    selected: &str,
    around: Option<&(String, String)>,
    pairs: &[(&str, &str)],
    listed: &[String],
) -> Option<String> {
    let wanted = words(selected);
    if wanted.is_empty() {
        return None;
    }
    let (before, after) = around
        .map(|(before, after)| (words(before), words(after)))
        .unwrap_or_default();
    let mut found = Vec::new();
    let mut fitting: Option<(usize, String)> = None;
    for (raw, cleaned) in pairs {
        let cleaned_words: Vec<String> = cleaned.split_whitespace().map(normal).collect();
        for start in occurrences(&cleaned_words, &wanted) {
            let Some(heard) = heard_at(&wanted, start, raw, &cleaned_words) else {
                continue;
            };
            let fit = fit(&cleaned_words, start, start + wanted.len(), &before, &after);
            if fit > fitting.as_ref().map_or(0, |(best, _)| *best) {
                fitting = Some((fit, heard.clone()));
            }
            found.push(heard);
        }
    }
    fitting
        .map(|(_, heard)| heard)
        .or_else(|| newest_unlisted(found, listed))
}

/// How many of the words just before and after the selection (`before`,
/// `after`) also sit around `cleaned[start..end]`, counting outward from it
/// and stopping at the first that differs on each side, up to `FIT` each.
fn fit(cleaned: &[String], start: usize, end: usize, before: &[String], after: &[String]) -> usize {
    let backward = before
        .iter()
        .rev()
        .zip(cleaned[..start].iter().rev())
        .take(FIT)
        .take_while(|(a, b)| a == b)
        .count();
    let forward = after
        .iter()
        .zip(&cleaned[end..])
        .take(FIT)
        .take_while(|(a, b)| a == b)
        .count();
    backward + forward
}

/// Every term and spelling Vocabulary lists, as compared.
fn listed_spellings(entries: &[crate::dictation::vocabulary::VocabularyEntry]) -> Vec<String> {
    entries
        .iter()
        .flat_map(|entry| std::iter::once(&entry.term).chain(&entry.sounds_like))
        .map(|spelling| spelling.trim().to_lowercase())
        .collect()
}

/// The first of `found` (newest first) that `listed` lacks, else the first.
fn newest_unlisted(found: Vec<String>, listed: &[String]) -> Option<String> {
    found
        .iter()
        .find(|heard| !listed.contains(&heard.to_lowercase()))
        .or(found.first())
        .cloned()
}

/// What `raw` said where `cleaned` first has `selected`; `None` when
/// `cleaned` lacks it, the transcript already said it, or the two don't line
/// up there.
#[cfg(test)]
fn heard_before(selected: &str, raw: &str, cleaned: &str) -> Option<String> {
    let wanted = words(selected);
    let cleaned_words: Vec<String> = cleaned.split_whitespace().map(normal).collect();
    let start = occurrences(&cleaned_words, &wanted).next()?;
    heard_at(&wanted, start, raw, &cleaned_words)
}

/// A text's words as compared, leaving out bare punctuation.
fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(normal)
        .filter(|word| !word.is_empty())
        .collect()
}

/// Where `wanted` (not empty) starts in `cleaned`, each time.
fn occurrences<'a>(
    cleaned: &'a [String],
    wanted: &'a [String],
) -> impl Iterator<Item = usize> + 'a {
    cleaned
        .windows(wanted.len().max(1))
        .enumerate()
        .filter(move |(_, words)| !wanted.is_empty() && *words == wanted)
        .map(|(index, _)| index)
}

/// What `raw` said where the cleaned text's words `cleaned_normal` have
/// `wanted` at `start`.
fn heard_at(
    wanted: &[String],
    start: usize,
    raw: &str,
    cleaned_normal: &[String],
) -> Option<String> {
    let raw_words: Vec<&str> = raw.split_whitespace().collect();
    let raw_normal: Vec<String> = raw_words.iter().map(|word| normal(word)).collect();
    let end = start + wanted.len();
    let (cleaned_from, cleaned_to, raw_from, raw_to) =
        window(start, end, cleaned_normal.len(), raw_normal.len());
    let anchors: Vec<(usize, usize)> = common(
        &cleaned_normal[cleaned_from..cleaned_to],
        &raw_normal[raw_from..raw_to],
    )
    .into_iter()
    .map(|(c, r)| (c + cleaned_from, r + raw_from))
    .collect();
    if (start..end).all(|index| anchors.iter().any(|&(c, _)| c == index)) {
        return None;
    }
    let from = anchors
        .iter()
        .rev()
        .find(|&&(c, _)| c < start)
        .map_or(raw_from, |&(_, r)| r + 1);
    let to = anchors
        .iter()
        .find(|&&(c, _)| c >= end)
        .map_or(raw_to, |&(_, r)| r);
    // A span much longer than the selection means the texts don't line up.
    if to <= from || to - from > 2 * wanted.len() + 2 {
        return None;
    }
    let heard = raw_words[from..to]
        .join(" ")
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_string();
    let same = heard
        .split_whitespace()
        .map(normal)
        .eq(wanted.iter().cloned());
    (!heard.is_empty() && !same).then_some(heard)
}

/// A word as compared: lowercase, without the punctuation around it.
fn normal(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// The ranges aligned: everything when the table is small enough, else
/// `WINDOW` words around the selection and the transcript's words near the
/// same share of the text.
fn window(start: usize, end: usize, cleaned: usize, raw: usize) -> (usize, usize, usize, usize) {
    if (cleaned + 1) * (raw + 1) <= MAX_CELLS {
        return (0, cleaned, 0, raw);
    }
    let centre = start * raw / cleaned.max(1);
    (
        start.saturating_sub(WINDOW),
        (end + WINDOW).min(cleaned),
        centre.saturating_sub(3 * WINDOW),
        (centre + 3 * WINDOW + end - start).min(raw),
    )
}

/// The longest common subsequence of `a` and `b` as index pairs, in order.
fn common(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    let width = b.len() + 1;
    let mut table = vec![0_u32; (a.len() + 1) * width];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j, mut pairs) = (0, 0, Vec::new());
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_cleanup_replaced_finds_what_was_heard() {
        let raw = "Я написал приложение Динкдан для того, чтобы работать с агентами.";
        let cleaned = "Я написал приложение Directus для того, чтобы работать с агентами.";
        assert_eq!(
            heard_before("Directus", raw, cleaned).as_deref(),
            Some("Динкдан")
        );
        // Punctuation around the selection or the word doesn't matter.
        let raw = "открой динкдан, пожалуйста";
        let cleaned = "Открой Directus, пожалуйста.";
        assert_eq!(
            heard_before("Directus,", raw, cleaned).as_deref(),
            Some("динкдан")
        );
    }

    #[test]
    fn several_words_map_to_several() {
        let raw = "запусти клод код сейчас";
        let cleaned = "Запусти Claude Code сейчас.";
        assert_eq!(
            heard_before("Claude Code", raw, cleaned).as_deref(),
            Some("клод код")
        );
    }

    #[test]
    fn an_unchanged_or_missing_selection_finds_nothing() {
        let raw = "я написал приложение динкдан";
        let cleaned = "Я написал приложение Directus.";
        assert_eq!(heard_before("приложение", raw, cleaned), None);
        assert_eq!(heard_before("GitHub", raw, cleaned), None);
        assert_eq!(heard_before("  ", raw, cleaned), None);
    }

    #[test]
    fn a_spelling_vocabulary_already_lists_gives_way_to_a_new_one() {
        let listed = vec!["directus".to_string(), "директус".to_string()];
        let found = vec!["Директус".to_string(), "Динкдан".to_string()];
        assert_eq!(newest_unlisted(found, &listed).as_deref(), Some("Динкдан"));
        let found = vec!["Директус".to_string()];
        assert_eq!(newest_unlisted(found, &listed).as_deref(), Some("Директус"));
        assert_eq!(newest_unlisted(Vec::new(), &listed), None);
    }

    #[test]
    fn the_words_around_the_selection_pick_its_dictation() {
        let pairs = [
            ("он сказал директус вчера", "Он сказал Directus вчера."),
            (
                "я написал приложение динкдан для агентов",
                "Я написал приложение Directus для агентов.",
            ),
        ];
        let around = (
            "Я написал приложение ".to_string(),
            " для агентов.".to_string(),
        );
        assert_eq!(
            best("Directus", Some(&around), &pairs, &[]).as_deref(),
            Some("динкдан")
        );
        // Without the text around it, the newest.
        assert_eq!(
            best("Directus", None, &pairs, &[]).as_deref(),
            Some("директус")
        );
        // Text around it that no entry has is no help either.
        let around = ("совсем другое ".to_string(), String::new());
        assert_eq!(
            best("Directus", Some(&around), &pairs, &[]).as_deref(),
            Some("директус")
        );
    }

    #[test]
    fn a_long_text_aligns_a_window_around_the_selection() {
        let filler: Vec<String> = (0..1500).map(|i| format!("w{i}")).collect();
        let raw = format!("{} динкдан {}", filler.join(" "), filler.join(" "));
        let cleaned = format!("{} Directus {}", filler.join(" "), filler.join(" "));
        assert_eq!(
            heard_before("Directus", &raw, &cleaned).as_deref(),
            Some("динкдан")
        );
    }
}
