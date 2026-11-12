//! The one gate everything Assistant proposes passes before the pane sees
//! it: a setting card from an answer, a plan's row (the app's rules', a
//! follow-up's, a model's, the preset's), a follow-up question, a term to
//! add, the instructions. `control::settings::check` is the one way a
//! change is applied; this is the one way one is offered.
//!
//! One verdict per proposal ([`Verdict`]): show it, show it unchecked, or
//! drop it with why. The rules, in order, for a setting ([`Gate::setting`]):
//!
//! 1. the key exists, and no card ever changes it (`catalog`);
//! 2. the preset asked for where it isn't offered gets words, no card;
//! 3. this VibeHex has it on this OS, and an organization doesn't hold it;
//! 4. this computer can use it (a notch, a mouse's extra buttons);
//! 5. the value is one a save takes (`settings::check`, the same check);
//! 6. it differs from the value now (no card changes a value to itself);
//! 7. in a plan, it wasn't declined in an earlier setup while the setting
//!    is still as it was then;
//! 8. in an answer, it isn't a way to look something up for a question
//!    that asked for no change (`lookup`);
//! 9. in a plan or the preset it comes unchecked when it shortens a
//!    retention, runs on Codex while signed out, needs an engine that isn't
//!    ready, or (a plan's inferred line) replaces a value the person chose.
//!
//! A term passes [`Gate::term`] (a term `hex vocabulary add` takes, not in
//! Vocabulary, not declined, and not an ordinary word heard right), and a
//! follow-up question [`Gate::ask`].
//!
//! What a card shows once it exists (`changes::state`: a value moved
//! since, an organization's lock set later) is not this gate's: that is
//! the live state of a card already offered.

mod lookup;
#[cfg(test)]
mod tests;

use serde_json::{Map, Value};

use crate::app_settings::AppSettings;
use crate::assistant::changes::catalog::{self, Class};
use crate::assistant::changes::vocab::Term;
use crate::assistant::changes::{self, Item, names, parts};
use crate::assistant::facts::State;
use crate::assistant::recordings::terms;
use crate::assistant::setup::{engines, follow, past};
use crate::control::settings::{self, ShortcutRules};
use crate::dictation::vocabulary::{self, VocabularyEntry};

/// Who proposes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Origin {
    /// A model's line in its answer to a question the person typed.
    Answer,
    /// Setup's plan: the app's rules, a follow-up's Yes, a model's lines,
    /// the terms the use shows.
    Plan,
    /// The owner's preset, which the person asked for by name.
    Preset,
}

/// What becomes of a proposal.
#[derive(Clone, Debug, PartialEq)]
pub enum Verdict {
    Show,
    /// Shown, and not checked for the person.
    Unchecked(Held),
    Drop(Why),
}

/// Why a row starts unchecked.
#[derive(Clone, Debug, PartialEq)]
pub enum Held {
    /// The person set the value there is now themselves.
    Yours,
    /// It shortens a retention: older recordings would go.
    Shorter,
    /// It runs on Codex, signed out.
    SignedOut,
    /// Its engine isn't ready: where to finish first, in English.
    NotReady(&'static str),
}

/// Why a proposal is dropped.
#[derive(Clone, Debug, PartialEq)]
pub enum Why {
    /// No such setting in this VibeHex.
    Unknown,
    /// A key no card ever changes.
    Never,
    /// The value isn't one a save takes.
    Invalid(String),
    /// Not in this VibeHex on this OS, or an organization holds it.
    Elsewhere(String),
    /// This computer can't use it.
    Hardware,
    /// It is that value now.
    Same,
    /// On already (a follow-up question).
    On,
    /// Declined in an earlier setup, and the setting is as it was then.
    Declined,
    /// Answered in an earlier setup.
    Answered,
    /// A way to look something up, for a question that asked no change.
    Lookup,
    /// The preset asked for where it isn't offered.
    NotOffered,
    /// An ordinary word heard right, not a term.
    Ordinary,
    /// Vocabulary has the term.
    Has,
}

impl Why {
    /// As the record under an answer says it.
    pub fn words(&self) -> String {
        match self {
            Self::Unknown => "no such setting".into(),
            Self::Never => "never changed from a card".into(),
            Self::Invalid(why) => why.clone(),
            Self::Elsewhere(why) => format!("not available here ({why})"),
            Self::Hardware => "this computer can't use it".into(),
            Self::Same => "already set to that".into(),
            Self::On => "on already".into(),
            Self::Declined => "declined in an earlier setup".into(),
            Self::Answered => "answered in an earlier setup".into(),
            Self::Lookup => "only a way to look something up".into(),
            Self::NotOffered => "the preset isn't offered with this recognition language".into(),
            Self::Ordinary => "an ordinary word heard right, not a term".into(),
            Self::Has => "Vocabulary has it".into(),
        }
    }

    /// The refusal the person and the model see, for a line the model got
    /// wrong; the rest are dropped without a row.
    pub fn refusal(&self) -> Option<changes::Why> {
        match self {
            Self::Unknown => Some(changes::Why::Unknown),
            Self::Never => Some(changes::Why::Never),
            Self::Invalid(why) => Some(changes::Why::Invalid(why.clone())),
            _ => None,
        }
    }
}

/// One proposal dropped, for the turn's record.
#[derive(Clone, Debug, PartialEq)]
pub struct Dropped {
    /// `labs.smart_spacing = true`, `term Web`, `ask labs.notch_indicator`.
    pub what: String,
    pub why: Why,
}

impl Dropped {
    /// Whether it is a term: maybe a word the person said.
    pub fn term(&self) -> bool {
        self.what.starts_with("term ")
    }

    pub fn line(&self) -> String {
        format!("{}: {}", self.what, self.why.words())
    }
}

const RECORD: &str = "Proposed and not offered:";

/// What the turn keeps of what was dropped, under "What Assistant knew".
pub fn record(dropped: &[Dropped]) -> String {
    if dropped.is_empty() {
        return String::new();
    }
    let lines: Vec<String> = dropped.iter().map(Dropped::line).collect();
    format!("{RECORD}\n{}\n", lines.join("\n"))
}

/// The dropped lines of a turn's record, as [`record`] wrote them.
#[cfg(test)]
pub fn recorded(knew: &str) -> Vec<&str> {
    knew.split_once(RECORD)
        .map(|(_, lines)| lines.lines().filter(|line| !line.is_empty()).collect())
        .unwrap_or_default()
}

/// A setting someone proposes.
#[derive(Clone, Copy, Debug)]
pub struct Offer<'a> {
    pub key: &'a str,
    pub to: &'a Value,
    /// The person's own answer this round (an engine picked, a
    /// follow-up's Yes): never "You chose" over their own choice.
    pub own: bool,
    /// Where to finish first, when the rules know it isn't ready.
    pub note: Option<&'static str>,
}

impl<'a> Offer<'a> {
    pub fn of(key: &'a str, to: &'a Value) -> Self {
        Self {
            key,
            to,
            own: false,
            note: None,
        }
    }
}

/// The keys an engine is chosen with: unchecked while it isn't ready.
const ENGINES: [&str; 5] = [
    "dictation_processing.server.runs_on",
    "meetings.notes.writer",
    "meetings.questions.engine",
    "files.cleanup_engine",
    "files.request_engine",
];

/// The gate, for one install as it is now.
pub struct Gate<'a> {
    pub settings: &'a AppSettings,
    pub state: &'a State,
    /// The question being answered, when there is one.
    question: &'a str,
    shortcuts: ShortcutRules,
}

impl<'a> Gate<'a> {
    pub fn new(settings: &'a AppSettings, state: &'a State) -> Self {
        Self {
            settings,
            state,
            question: "",
            shortcuts: crate::rune_ui::shortcut_rules(),
        }
    }

    /// The same gate for an answer to `question`.
    pub fn asked(mut self, question: &'a str) -> Self {
        self.question = question;
        self
    }

    /// The whole key a save takes for `key`: a part's parent.
    pub fn whole(&self, key: &str) -> String {
        match parts::part(key, self.settings) {
            Some(part) => part.parent().to_owned(),
            None => key.to_owned(),
        }
    }

    /// Whether this VibeHex has `key` here and this computer can use it.
    pub fn here(&self, key: &str) -> Result<(), Why> {
        if let Some(error) = settings::unavailable(&self.whole(key)) {
            return Err(Why::Elsewhere(error.message));
        }
        if follow::unfit(key, &self.state.hardware) {
            return Err(Why::Hardware);
        }
        Ok(())
    }

    /// Whether `value` is one a save of `key` takes, as `check` checks it.
    pub fn fits(&self, key: &str, value: &Value) -> Result<(), Why> {
        let values = match parts::part(key, self.settings) {
            Some(part) => {
                let (parent, json) = part.with(self.settings, value).map_err(Why::Invalid)?;
                Map::from_iter([(parent.to_owned(), json)])
            }
            None => Map::from_iter([(key.to_owned(), value.clone())]),
        };
        settings::check(self.settings, &values, &self.shortcuts)
            .map(|_| ())
            .map_err(|error| match error.code {
                "unknown-setting" => Why::Unknown,
                "read-only" => Why::Never,
                "unsupported" | "managed" => Why::Elsewhere(error.message),
                _ => Why::Invalid(error.message),
            })
    }

    /// What `key` at `to` needs first that isn't set: `(key, value)`.
    pub fn requires(&self, key: &str, to: &Value) -> Vec<(String, Value)> {
        match parts::part(key, self.settings) {
            Some(part) => settings::missing(self.settings, part.parent(), &Value::Bool(true)),
            None => settings::missing(self.settings, key, to),
        }
    }

    /// The verdict on an edit of a Vocabulary entry: the entry and its
    /// spellings are checked where the edit is made (`vocab_edit::card`).
    pub fn edit(&self, origin: Origin) -> Verdict {
        match origin == Origin::Answer
            && crate::assistant::intent::preset_not_offered(self.question, self.settings)
        {
            true => Verdict::Drop(Why::NotOffered),
            false => Verdict::Show,
        }
    }

    /// Rules 1 to 5: whether `key` to `to` may be offered at all, whatever
    /// the value is now (a choice shows its options whatever is set).
    pub fn allowed(&self, origin: Origin, key: &str, to: &Value) -> Result<Value, Why> {
        let now = changes::current(self.settings, key).ok_or(Why::Unknown)?;
        if catalog::class(key, to) == Class::Never {
            return Err(Why::Never);
        }
        if origin == Origin::Answer
            && crate::assistant::intent::preset_not_offered(self.question, self.settings)
        {
            return Err(Why::NotOffered);
        }
        self.reachable(key, to)?;
        self.fits(key, to)?;
        Ok(now)
    }

    /// Rules 3 and 4 for `key` at `to`: here, and so is what it needs
    /// first (a Labs feature another OS has is no way to it).
    pub fn reachable(&self, key: &str, to: &Value) -> Result<(), Why> {
        self.here(key)?;
        for (needed, _) in self.requires(key, to) {
            if let Some(error) = settings::unavailable(&needed) {
                return Err(Why::Elsewhere(error.message));
            }
        }
        Ok(())
    }

    /// The verdict on a setting.
    pub fn setting(&self, origin: Origin, offer: Offer) -> Verdict {
        let Offer { key, to, own, note } = offer;
        let now = match self.allowed(origin, key, to) {
            Ok(now) => now,
            Err(why) => return Verdict::Drop(why),
        };
        if names::same(self.settings, key, &now, to) {
            return Verdict::Drop(Why::Same);
        }
        match origin {
            Origin::Answer => {
                if lookup::only(key, self.question) {
                    return Verdict::Drop(Why::Lookup);
                }
                Verdict::Show
            }
            Origin::Plan | Origin::Preset => {
                if origin == Origin::Plan
                    && past::held(&self.state.declined, key, to, self.settings)
                {
                    return Verdict::Drop(Why::Declined);
                }
                match note.or_else(|| self.not_ready(key, to)) {
                    // Codex signed out has its sign-in row, and no note.
                    Some("") => return Verdict::Unchecked(Held::SignedOut),
                    Some(note) => return Verdict::Unchecked(Held::NotReady(note)),
                    None => {}
                }
                if catalog::shorter(key, &now, to).is_some() {
                    return Verdict::Unchecked(Held::Shorter);
                }
                if catalog::needs_codex(key, to) && self.state.codex_signed_in.is(&false) {
                    return Verdict::Unchecked(Held::SignedOut);
                }
                if origin == Origin::Plan && !own && self.yours(key) {
                    return Verdict::Unchecked(Held::Yours);
                }
                Verdict::Show
            }
        }
    }

    /// Where to finish first when `key` picks an engine that isn't ready.
    fn not_ready(&self, key: &str, to: &Value) -> Option<&'static str> {
        if !ENGINES.contains(&key) {
            return None;
        }
        let id = to.as_str().unwrap_or_default();
        let id = id.split(':').next().unwrap_or_default();
        engines::not_ready(id, self.settings, self.state)
    }

    /// Whether `key` is set to something but its default: the person
    /// chose it.
    fn yours(&self, key: &str) -> bool {
        let default = AppSettings::default();
        changes::current(self.settings, key) != changes::current(&default, key)
    }

    /// `item` as its verdict starts it: checked, or unchecked with why.
    pub fn start(item: &mut Item, held: &Held) {
        item.included = false;
        match held {
            Held::Yours => item.yours = true,
            Held::NotReady(note) => item.note = (*note).to_owned(),
            Held::Shorter | Held::SignedOut => {}
        }
    }

    /// The verdict on a term to add, with how it is said to sound: one
    /// `hex vocabulary add` takes, that Vocabulary lacks, that wasn't
    /// declined, and that isn't an ordinary word heard right. The preset's
    /// terms, a follow-up's (`own`), and a term the question itself names
    /// are the person's ask and skip the last rule.
    pub fn term(&self, origin: Origin, term: &Term, own: bool) -> Verdict {
        // The same check `hex vocabulary add` makes, on a list of its own.
        let mut scratch: Vec<VocabularyEntry> = Vec::new();
        if let Err(why) = vocabulary::add(&mut scratch, &term.term, &term.sounds_like) {
            return Verdict::Drop(Why::Invalid(why));
        }
        if origin == Origin::Answer
            && crate::assistant::intent::preset_not_offered(self.question, self.settings)
        {
            return Verdict::Drop(Why::NotOffered);
        }
        if vocabulary::contains(&self.settings.vocabulary, &term.term) {
            return Verdict::Drop(Why::Has);
        }
        let lower = term.term.to_lowercase();
        if origin == Origin::Plan
            && !own
            && self
                .state
                .declined
                .iter()
                .filter(|line| line.key == changes::vocab::KEY)
                .flat_map(|line| changes::vocab::terms(&line.to))
                .any(|declined| declined.term.to_lowercase() == lower)
        {
            return Verdict::Drop(Why::Declined);
        }
        let asked = own
            || origin == Origin::Preset
            || (origin == Origin::Answer && self.question.to_lowercase().contains(&lower));
        if !asked && !terms::passes(&term.term, &term.sounds_like) {
            return Verdict::Drop(Why::Ordinary);
        }
        Verdict::Show
    }

    /// The verdict on a follow-up question about `id` (a Labs switch, or
    /// another setting a model asks about), against `after`, the settings
    /// once round one's lines are saved. `picked` when the app chose it
    /// this round (it left out what was answered before itself).
    pub fn ask(&self, id: &str, after: &AppSettings, picked: bool) -> Verdict {
        if follow::OWN.iter().any(|own| own.id == id) {
            let settled = self.answered(id) || self.declined_key(changes::vocab::KEY);
            return match settled && !picked {
                true => Verdict::Drop(Why::Answered),
                false => Verdict::Show,
            };
        }
        let Some(now) = changes::current(after, id) else {
            return Verdict::Drop(Why::Unknown);
        };
        if catalog::never(id) {
            return Verdict::Drop(Why::Never);
        }
        if let Err(why) = self.here(id) {
            return Verdict::Drop(why);
        }
        if now == Value::Bool(true) {
            return Verdict::Drop(Why::On);
        }
        if !picked && self.answered(id) {
            return Verdict::Drop(Why::Answered);
        }
        if !picked && self.declined_key(id) {
            return Verdict::Drop(Why::Declined);
        }
        Verdict::Show
    }

    fn answered(&self, id: &str) -> bool {
        self.state.answered.iter().any(|ask| ask.id == id)
    }

    /// Whether a line on `key` was declined and the setting is as it was.
    fn declined_key(&self, key: &str) -> bool {
        self.state.declined.iter().any(|line| {
            line.key == key && past::held(&self.state.declined, key, &line.to, self.settings)
        })
    }
}
