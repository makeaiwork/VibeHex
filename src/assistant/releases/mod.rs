//! What changed in VibeHex, release by release: docs/releases' own notes
//! (`<X.Y.Z-vibe.N>.md`, and `next.md` for the work not released yet),
//! carried in the binary. Front matter gives the version, the date, the
//! Labs features (`labs:`, `hex labs` names) and the settings keys
//! (`keys:`) each release added or changed; the body is short plain bullets.

use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/release_files.rs"));

/// The name `next.md` has: the work not released yet.
pub const NEXT: &str = "next";

/// One release's notes.
#[derive(Debug)]
pub struct Note {
    /// `3.3.0-vibe.4`, or `next`.
    pub version: String,
    /// `2026-11-04`, or `unreleased` for Next.
    pub date: String,
    /// The Labs features it added or changed, by `hex labs` name.
    pub labs: Vec<String>,
    /// Other settings keys it added or changed.
    pub keys: Vec<String>,
    /// Markdown after the front matter.
    pub body: String,
}

impl Note {
    pub fn is_next(&self) -> bool {
        self.version == NEXT
    }

    /// Every settings key it names: its Labs features' keys, then its own.
    pub fn settings_keys(&self) -> Vec<&'static str> {
        let labs = self
            .labs
            .iter()
            .filter_map(|name| crate::labs::Feature::from_name(name))
            .map(crate::labs::Feature::settings_key);
        let keys = self.keys.iter().filter_map(|key| {
            crate::control::settings::kinds()
                .map(|(known, ..)| known)
                .find(|known| known == key)
        });
        let mut all: Vec<&'static str> = Vec::new();
        for key in labs.chain(keys) {
            if !all.contains(&key) {
                all.push(key);
            }
        }
        all
    }
}

/// Every release's notes, oldest first, Next last.
pub fn notes() -> &'static [Note] {
    static NOTES: OnceLock<Vec<Note>> = OnceLock::new();
    NOTES.get_or_init(|| {
        let mut notes: Vec<Note> = FILES.iter().map(|(_, text)| parse(text)).collect();
        notes.sort_by_key(|note| order(&note.version).unwrap_or([u64::MAX; 4]));
        notes
    })
}

/// The newest release's notes, Next aside.
pub fn latest() -> Option<&'static Note> {
    notes().iter().rev().find(|note| !note.is_next())
}

/// The notes for `version`, as written or as `vibe.N` of the newest line
/// ("vibe.5", "5").
pub fn find(version: &str) -> Option<&'static Note> {
    find_in(notes(), version)
}

fn find_in(all: &'static [Note], version: &str) -> Option<&'static Note> {
    let version = version.trim();
    let version = match version.strip_prefix(['v', 'V']) {
        Some(rest) if rest.starts_with(|c: char| c.is_ascii_digit()) => rest,
        _ => version,
    };
    if let Some(note) = all.iter().find(|note| note.version == version) {
        return Some(note);
    }
    let number = version
        .strip_prefix("vibe.")
        .unwrap_or(version)
        .parse::<u64>()
        .ok()?;
    all.iter()
        .rev()
        .find(|note| order(&note.version).is_some_and(|at| at[3] == number))
}

/// `X.Y.Z-vibe.N` as numbers to sort by; `None` for any other name.
pub fn order(version: &str) -> Option<[u64; 4]> {
    let (stable, build) = version.split_once("-vibe.")?;
    let mut parts = stable.split('.').map(|part| part.parse::<u64>().ok());
    let (major, minor, patch) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() || build.is_empty() || !build.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some([major, minor, patch, build.parse().ok()?])
}

fn parse(text: &str) -> Note {
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
    let list = |name: &str| {
        field(name)
            .map(|list| {
                list.trim_matches(['[', ']'])
                    .split(',')
                    .map(|item| item.trim().to_owned())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };
    Note {
        version: field("version").unwrap_or_default(),
        date: field("date").unwrap_or_default(),
        labs: list("labs"),
        keys: list("keys"),
        body: body.trim().to_owned(),
    }
}

#[cfg(test)]
mod tests;
