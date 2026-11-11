//! Guided setup, "Set VibeHex up for me": three questions with option
//! chips, then one list of changes the person ticks and applies.
//!
//! The questions are the app's own, with fixed ids (`questions`). The
//! Start screen's button draws them at once, in the window's language,
//! with no model call; a typed request ("помоги настроить") asks the model
//! to write them in the message's language (`hex-ask` lines), and `asks`
//! keeps only known ids, falling back to the app's own when they are
//! missing or broken. The answers go back as one turn the model reads as
//! `<answers>`, and it writes a `hex-plan`; `plan` tidies that into one
//! checklist, and when nothing usable came, `baseline` makes one from
//! rules checked against this VibeHex's keys, models, and OS.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::changes::catalog::{self, Shorter};
use super::changes::parse::Proposal;
use super::changes::vocab::{self, Term};
use super::changes::{Card, CardKind};
use crate::app_settings::AppSettings;
use crate::control::settings;
use crate::desktop::i18n::tr;
use crate::transcription::models::{self, TranscriptionModelId};

/// The most lines a plan keeps.
pub const PLAN_LINES: usize = 8;

/// One question with option chips.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Ask {
    /// `use`, `language`, or `cloud`.
    pub id: String,
    /// The question as the model wrote it; for the app's own (`own`), the
    /// English the pane translates.
    pub q: String,
    /// A few words beside it: "pick any".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hint: String,
    /// Several options may be chosen, then Continue.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub multi: bool,
    pub options: Vec<Choice>,
    /// The app's own words, shown in the window's language.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub own: bool,
    /// The option ids picked so far.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub chosen: Vec<String>,
    /// Answered: it shows as chosen chips.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
}

/// One option of a question.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

/// What the person picked for one question.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Answer {
    pub id: String,
    pub chosen: Vec<String>,
}

/// Where a setup conversation is.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Stage {
    /// An ordinary question.
    #[default]
    None,
    /// A typed setup request: the model writes the questions.
    Ask,
    /// The answers came: the model writes the plan. `line` is them as the
    /// model reads them, with the labels the person saw.
    Plan { answers: Vec<Answer>, line: String },
}

/// The fixed questions: id, the English question, its hint, whether
/// several may be picked, and the options' ids and English labels.
type Question = (
    &'static str,
    &'static str,
    &'static str,
    bool,
    &'static [(&'static str, &'static str)],
);

const QUESTIONS: [Question; 3] = [
    (
        "use",
        "What do you use VibeHex for?",
        "pick any",
        true,
        &[
            ("dictation_any", "Dictation into any app"),
            ("dictation_code", "Dictation into code and the terminal"),
            ("meetings", "Recording calls"),
            ("files", "Transcribing files"),
        ],
    ),
    (
        "language",
        "Which language do you usually speak?",
        "",
        false,
        &[
            ("ru", "Russian"),
            ("en", "English"),
            ("mixed", "Both, mixed"),
        ],
    ),
    (
        "cloud",
        "May text go to a cloud model?",
        "for cleaning up dictation and meeting notes",
        false,
        &[
            ("chatgpt", "Yes, through my ChatGPT"),
            ("this_mac", "Only on this Mac"),
            ("no_cleanup", "No cleanup for now"),
        ],
    ),
];

/// The app's own three questions on `os`, in English for the pane to
/// translate. Off a Mac, "this Mac" is "this computer".
pub fn questions(os: &str) -> Vec<Ask> {
    QUESTIONS
        .iter()
        .map(|&(id, q, hint, multi, options)| Ask {
            id: id.into(),
            q: q.into(),
            hint: hint.into(),
            multi,
            own: true,
            options: options
                .iter()
                .map(|&(id, label)| Choice {
                    id: id.into(),
                    label: own_label(id, label, os).into(),
                })
                .collect(),
            ..Ask::default()
        })
        .collect()
}

/// Every English word the app's own questions show, for the translation
/// tables' test.
#[cfg(test)]
pub fn own_words() -> Vec<&'static str> {
    let mut words = vec!["Only on this computer"];
    for (_, q, hint, _, options) in QUESTIONS {
        words.extend([q, hint].into_iter().filter(|word| !word.is_empty()));
        words.extend(options.iter().map(|(_, label)| *label));
    }
    words
}

fn own_label<'a>(id: &str, label: &'a str, os: &str) -> &'a str {
    if id == "this_mac" && os != "macos" {
        "Only on this computer"
    } else {
        label
    }
}

/// The questions to show for the `hex-ask` lines a model wrote: its own
/// words when every fixed question came with known option ids, else the
/// app's own.
pub fn asks(written: &[Value], os: &str) -> Vec<Ask> {
    let own = questions(os);
    let read: Option<Vec<Ask>> = own
        .iter()
        .map(|fixed| {
            let line = written
                .iter()
                .find(|line| line.get("id").and_then(Value::as_str) == Some(&fixed.id))?;
            let q = line.get("q").and_then(Value::as_str)?.trim();
            let options: Vec<Choice> =
                line.get("options")?
                    .as_array()?
                    .iter()
                    .filter_map(|option| {
                        let id = option.get("id").and_then(Value::as_str)?;
                        let label = option.get("label").and_then(Value::as_str)?.trim();
                        (fixed.options.iter().any(|known| known.id == id) && !label.is_empty())
                            .then(|| Choice {
                                id: id.into(),
                                label: label.into(),
                            })
                    })
                    .collect();
            (!q.is_empty() && options.len() >= 2).then(|| Ask {
                id: fixed.id.clone(),
                q: q.into(),
                hint: line
                    .get("hint")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .into(),
                multi: fixed.multi,
                options,
                ..Ask::default()
            })
        })
        .collect();
    read.unwrap_or(own)
}

/// The `hex-ask` lines a model is shown to write, in English.
pub fn ask_lines(os: &str) -> String {
    questions(os)
        .iter()
        .map(|ask| {
            let mut line = serde_json::json!({
                "id": ask.id,
                "q": ask.q,
                "options": ask.options.iter().map(|option| {
                    serde_json::json!({"id": option.id, "label": option.label})
                }).collect::<Vec<_>>(),
            });
            if !ask.hint.is_empty() {
                line["hint"] = ask.hint.clone().into();
            }
            if ask.multi {
                line["multi"] = true.into();
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The answers as the model reads them: `use: dictation_code (Dictation
/// into code…), meetings (…); language: ru (…)`, with `label` naming each
/// option as the person saw it.
pub fn answers_line(asks: &[Ask], label: impl Fn(&Ask, &Choice) -> String) -> String {
    asks.iter()
        .filter(|ask| !ask.chosen.is_empty())
        .map(|ask| {
            let chosen: Vec<String> = ask
                .options
                .iter()
                .filter(|option| ask.chosen.contains(&option.id))
                .map(|option| format!("{} ({})", option.id, label(ask, option)))
                .collect();
            format!("{}: {}", ask.id, chosen.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What the guide is searched for, for these answers.
pub fn query(answers: &[Answer]) -> String {
    let mut words = vec!["set up"];
    for id in answers.iter().flat_map(|answer| &answer.chosen) {
        words.push(match id.as_str() {
            "dictation_any" => "dictation smart spacing vocabulary",
            "dictation_code" => "vibe coding terminal code",
            "meetings" => "meetings notes calls",
            "files" => "transcribe files",
            "ru" => "Russian speech model",
            "en" => "English speech model",
            "mixed" => "speech model languages",
            "chatgpt" => "Codex ChatGPT AI cleanup",
            "this_mac" => "offline on this computer",
            _ => continue,
        });
    }
    words.join(" ")
}

fn picked<'a>(answers: &'a [Answer], id: &str) -> impl Iterator<Item = &'a str> {
    answers
        .iter()
        .filter(move |answer| answer.id == id)
        .flat_map(|answer| answer.chosen.iter().map(String::as_str))
}

/// A plan from rules, for when the model wrote none that the app could
/// use. Each rule names a key this VibeHex has and that this OS allows,
/// and a model only when it is in the catalog and usable here; what is set
/// that way already is left out.
pub fn baseline(answers: &[Answer], settings: &AppSettings) -> Vec<Proposal> {
    let uses: Vec<&str> = picked(answers, "use").collect();
    let language = picked(answers, "language").next().unwrap_or_default();
    let cloud = picked(answers, "cloud").next().unwrap_or_default();
    let dictates = uses.contains(&"dictation_any") || uses.contains(&"dictation_code");
    let code = uses.contains(&"dictation_code");
    let mut lines: Vec<(&str, Value, &'static str, bool)> = Vec::new();
    if let Some((model, why)) = speech_model(language, settings) {
        lines.push((
            "transcription.model",
            Value::from(model.as_str()),
            why,
            false,
        ));
    }
    // Russian vibe coding switches AI cleanup on, so only with a cloud
    // model the person allowed, which then runs it.
    let vibe = code && matches!(language, "ru" | "mixed") && cloud == "chatgpt";
    if vibe {
        lines.push((
            "labs.russian_vibe_coding",
            Value::Bool(true),
            tr("Russian speech with English tech terms written as they should be"),
            false,
        ));
    }
    if dictates && cloud == "chatgpt" {
        lines.push((
            "dictation_processing.server.runs_on",
            Value::from("codex"),
            tr("Cleans up dictation on your ChatGPT plan"),
            false,
        ));
        // Russian vibe coding switches cleanup on itself.
        if !vibe {
            lines.push((
                super::changes::parts::CLEANUP,
                Value::Bool(true),
                tr("Every dictation is cleaned up before it pastes"),
                false,
            ));
        }
    }
    if dictates && cloud == "no_cleanup" {
        lines.push((
            super::changes::parts::CLEANUP,
            Value::Bool(false),
            tr("Dictation pastes as you said it"),
            false,
        ));
    }
    if uses.contains(&"meetings") {
        lines.push((
            "meetings.enabled",
            Value::Bool(true),
            tr("Records calls, with a transcript and notes"),
            false,
        ));
        if cloud == "chatgpt" {
            lines.push((
                "meetings.notes.writer",
                Value::from("codex"),
                tr("Notes written on your ChatGPT plan, good in Russian too"),
                false,
            ));
        }
    }
    if uses.contains(&"files") {
        lines.push((
            "labs.files",
            Value::Bool(true),
            tr("Turns recordings you already have into text"),
            false,
        ));
    }
    if dictates {
        lines.push((
            "labs.smart_spacing",
            Value::Bool(true),
            tr("Adds the space you'd type after a word"),
            false,
        ));
        lines.push((
            "labs.vocabulary_quick_add",
            Value::Bool(true),
            tr("Adds a misheard word to Vocabulary with one shortcut, handy if mistakes repeat"),
            true,
        ));
    }
    lines
        .into_iter()
        .filter(|(key, value, ..)| {
            settings::unavailable(key).is_none()
                && super::changes::current(settings, key).as_ref() != Some(value)
        })
        .take(PLAN_LINES)
        .map(|(key, value, why, optional)| Proposal {
            key: key.into(),
            value: Some(value),
            why: why.into(),
            optional,
            plan: true,
            line: key.into(),
            ..Proposal::default()
        })
        .chain(
            // Russian vibe coding brings its own terms.
            (code && matches!(language, "ru" | "mixed") && !vibe)
                .then(code_terms)
                .into_iter()
                .flatten(),
        )
        .collect()
}

/// Terms someone dictating code in Russian likely says, for an optional
/// Vocabulary row; the plan leaves out the ones the vocabulary has.
fn code_terms() -> Vec<Proposal> {
    const TERMS: [(&str, &[&str]); 5] = [
        ("GitHub", &["гитхаб", "гит хаб"]),
        ("Claude Code", &["клод код"]),
        ("pull request", &["пул реквест", "пулл реквест"]),
        ("TypeScript", &["тайпскрипт", "тайп скрипт"]),
        ("Docker", &["докер"]),
    ];
    TERMS
        .iter()
        .map(|(term, sounds_like)| Proposal {
            key: vocab::KEY.into(),
            value: Some(
                Term {
                    term: (*term).into(),
                    sounds_like: sounds_like.iter().map(|&text| text.into()).collect(),
                }
                .value(),
            ),
            why: tr("Tech terms you likely say, spelled as they should be").into(),
            optional: true,
            plan: true,
            line: vocab::KEY.into(),
            ..Proposal::default()
        })
        .collect()
}

/// The speech model for `language`, if the one in use doesn't suit it:
/// GigaAM v3 for Russian, a model that hears both for mixed, and the
/// picker's English recommendation for English.
fn speech_model(
    language: &str,
    settings: &AppSettings,
) -> Option<(TranscriptionModelId, &'static str)> {
    let now = models::definition(settings.transcription.model);
    let (model, why) = match language {
        "ru" => (
            TranscriptionModelId::GigaAmV3,
            tr("Best accuracy for Russian"),
        ),
        "mixed" if !(now.supports_language("ru") && now.supports_language("en")) => (
            TranscriptionModelId::ParakeetV3,
            tr("Hears Russian and English"),
        ),
        "en" if !now.supports_language("en") => (
            models::choices_for_runtime("en").first()?.model.id,
            tr("Made for English"),
        ),
        _ => return None,
    };
    let usable = models::MODELS
        .iter()
        .any(|known| known.id == model && known.available());
    (usable && model != settings.transcription.model).then_some((model, why))
}

/// One checklist from a plan reply's cards: every item in it once, none
/// already set or held here, at most [`PLAN_LINES`]. A shorter retention
/// and a change that needs a Codex sign-in start unchecked (owner
/// decision 4). `None` when nothing is left.
pub fn plan(cards: Vec<Card>, settings: &AppSettings, state: &catalog::State) -> Option<Card> {
    let mut items = Vec::new();
    let (terms, settings_items): (Vec<_>, Vec<_>) = cards
        .into_iter()
        .flat_map(|card| card.items)
        .partition(|item| item.key == vocab::KEY);
    for item in settings_items {
        let key = match super::changes::parts::part(&item.key, settings) {
            Some(part) => part.parent().to_owned(),
            None => item.key.clone(),
        };
        let held = settings::unavailable(&key).is_some();
        let known = items
            .iter()
            .any(|known: &super::changes::Item| known.key == item.key);
        if held || known || item.from == item.to || items.len() == PLAN_LINES {
            continue;
        }
        items.push(item);
    }
    // Every term in one row, never checked to begin with.
    if items.len() < PLAN_LINES
        && let Some(mut row) = vocab::row(terms)
    {
        row.included = false;
        items.push(row);
    }
    for item in &mut items {
        let shorter: Option<Shorter> = catalog::shorter(&item.key, &item.from, &item.to);
        let signed_out =
            catalog::needs_codex(&item.key, &item.to) && state.codex_signed_in == Some(false);
        if shorter.is_some() || signed_out {
            item.included = false;
        }
    }
    (!items.is_empty()).then(|| Card {
        kind: CardKind::Plan,
        items,
        ..Card::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::changes;
    use serde_json::json;

    fn answers(uses: &[&str], language: &str, cloud: &str) -> Vec<Answer> {
        vec![
            Answer {
                id: "use".into(),
                chosen: uses.iter().map(|id| (*id).into()).collect(),
            },
            Answer {
                id: "language".into(),
                chosen: vec![language.into()],
            },
            Answer {
                id: "cloud".into(),
                chosen: vec![cloud.into()],
            },
        ]
    }

    fn keys(lines: &[Proposal]) -> Vec<&str> {
        lines.iter().map(|line| line.key.as_str()).collect()
    }

    #[test]
    fn the_apps_questions_have_their_fixed_ids() {
        let asks = questions("macos");
        let ids: Vec<&str> = asks.iter().map(|ask| ask.id.as_str()).collect();
        assert_eq!(ids, ["use", "language", "cloud"]);
        assert!(asks[0].multi && !asks[1].multi && !asks[2].multi);
        assert_eq!(asks[2].options[1].label, "Only on this Mac");
        assert_eq!(
            questions("windows")[2].options[1].label,
            "Only on this computer"
        );
        let options: Vec<&str> = asks[0].options.iter().map(|o| o.id.as_str()).collect();
        assert_eq!(
            options,
            ["dictation_any", "dictation_code", "meetings", "files"]
        );
    }

    #[test]
    fn a_models_questions_keep_known_ids_or_fall_back() {
        let written: Vec<Value> = [
            r#"{"id":"use","q":"Для чего вам VibeHex?","hint":"можно несколько","multi":false,"options":[{"id":"dictation_any","label":"Диктовка"},{"id":"meetings","label":"Созвоны"},{"id":"teleport","label":"Телепорт"}]}"#,
            r#"{"id":"language","q":"На каком языке?","options":[{"id":"ru","label":"Русский"},{"id":"en","label":"English"}]}"#,
            r#"{"id":"cloud","q":"В облако можно?","options":[{"id":"chatgpt","label":"Да"},{"id":"no_cleanup","label":"Нет"}]}"#,
            r#"{"id":"mood","q":"Как дела?","options":[]}"#,
        ]
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
        let asks = asks(&written, "macos");
        assert_eq!(asks.len(), 3, "an unknown question is dropped");
        assert!(!asks[0].own && asks[0].multi, "multi is the app's to say");
        assert_eq!(asks[0].q, "Для чего вам VibeHex?");
        assert_eq!(asks[0].options.len(), 2, "an unknown option is dropped");
        // One question missing: the app's own three instead.
        let fallback = super::asks(&written[..2], "macos");
        assert!(fallback.iter().all(|ask| ask.own));
        assert_eq!(super::asks(&[], "macos"), questions("macos"));
    }

    #[test]
    fn the_answers_read_as_ids_and_the_labels_seen() {
        let mut asks = questions("macos");
        asks[0].chosen = vec!["dictation_code".into(), "meetings".into()];
        asks[1].chosen = vec!["ru".into()];
        let line = answers_line(&asks, |_, option| option.label.clone());
        assert_eq!(
            line,
            "use: dictation_code (Dictation into code and the terminal), meetings (Recording calls); language: ru (Russian)"
        );
    }

    /// Every rule's key is real, its value one a save takes, and its model
    /// one this OS can run.
    #[test]
    fn every_baseline_rule_resolves_to_a_valid_change() {
        let settings = AppSettings::default();
        let rules = crate::rune_ui::shortcut_rules();
        let all = ["dictation_any", "dictation_code", "meetings", "files"];
        for language in ["ru", "en", "mixed"] {
            for cloud in ["chatgpt", "this_mac", "no_cleanup"] {
                let lines = baseline(&answers(&all, language, cloud), &settings);
                let terms = lines.iter().filter(|line| line.key == vocab::KEY).count();
                assert!(lines.len() - terms <= PLAN_LINES);
                let (cards, refused) = changes::resolve(&lines, &settings, &rules);
                assert!(refused.is_empty(), "{language} {cloud}: {refused:?}");
                assert_eq!(cards.len(), 1, "one plan");
                assert_eq!(cards[0].items.len(), lines.len());
                let state = catalog::State::default();
                let plan = plan(cards, &settings, &state).unwrap();
                assert!(plan.items.len() <= PLAN_LINES);
            }
        }
    }

    /// Dictating code in Russian on this Mac: one Vocabulary row of terms,
    /// unchecked, without the ones the vocabulary has.
    #[test]
    fn russian_code_gets_an_unchecked_row_of_terms() {
        let settings = AppSettings {
            vocabulary: vec![crate::dictation::vocabulary::VocabularyEntry {
                term: "github".into(),
                sounds_like: Vec::new(),
            }],
            ..AppSettings::default()
        };
        let rules = crate::rune_ui::shortcut_rules();
        let lines = baseline(&answers(&["dictation_code"], "ru", "this_mac"), &settings);
        let (cards, _) = changes::resolve(&lines, &settings, &rules);
        let plan = plan(cards, &settings, &catalog::State::default()).unwrap();
        let row = plan
            .items
            .iter()
            .find(|item| item.key == vocab::KEY)
            .unwrap();
        assert!(!row.included);
        let terms = vocab::terms(&row.to);
        assert_eq!(terms.len(), 4);
        assert!(terms.iter().all(|term| term.term != "GitHub"));
        // With vibe coding, which brings its own terms, there is none.
        let lines = baseline(&answers(&["dictation_code"], "ru", "chatgpt"), &settings);
        assert!(lines.iter().all(|line| line.key != vocab::KEY));
    }

    #[test]
    fn russian_code_and_chatgpt_make_the_setup_boards_plan() {
        let settings = AppSettings::default();
        let lines = baseline(
            &answers(&["dictation_code", "meetings"], "ru", "chatgpt"),
            &settings,
        );
        // The board's list, less what this OS hasn't (Smart spacing, Quick
        // add, and Files are Mac labs).
        let here = |keys: &[&'static str]| -> Vec<&'static str> {
            keys.iter()
                .copied()
                .filter(|key| settings::unavailable(key).is_none())
                .collect()
        };
        assert_eq!(
            keys(&lines),
            here(&[
                "transcription.model",
                "labs.russian_vibe_coding",
                "dictation_processing.server.runs_on",
                "meetings.enabled",
                "meetings.notes.writer",
                "labs.smart_spacing",
                "labs.vocabulary_quick_add",
            ])
        );
        assert_eq!(lines[0].value, Some(json!("gigaam_v3")));
        assert!(
            lines
                .iter()
                .all(|line| line.optional == (line.key == "labs.vocabulary_quick_add"))
        );
        // English, offline, files only: a lab, and no model change.
        let lines = baseline(&answers(&["files"], "en", "this_mac"), &settings);
        assert_eq!(keys(&lines), here(&["labs.files"]));
    }

    #[test]
    fn what_is_set_already_is_left_out() {
        let mut settings = AppSettings::default();
        settings.transcription.model = TranscriptionModelId::GigaAmV3;
        settings.meetings.enabled = true;
        let lines = baseline(&answers(&["meetings"], "ru", "this_mac"), &settings);
        assert!(lines.is_empty(), "{:?}", keys(&lines));
    }

    #[test]
    fn a_plan_is_one_list_with_risky_lines_unchecked() {
        let settings = AppSettings::default();
        let rules = crate::rune_ui::shortcut_rules();
        let line = |key: &str, value: Value| Proposal {
            key: key.into(),
            value: Some(value),
            plan: true,
            ..Proposal::default()
        };
        let mut lines = vec![
            line("labs.filler_words", json!(true)),
            line("labs.filler_words", json!(true)),
            line("dictation_processing.server.runs_on", json!("codex")),
            line("meetings.retention", json!("day")),
        ];
        lines.push(line(
            "labs.spoken_formatting",
            changes::current(&settings, "labs.spoken_formatting").unwrap(),
        ));
        let (cards, _) = changes::resolve(&lines, &settings, &rules);
        let state = catalog::State {
            codex_signed_in: Some(false),
            ..catalog::State::default()
        };
        let plan = plan(cards, &settings, &state).unwrap();
        let shown: Vec<(&str, bool)> = plan
            .items
            .iter()
            .map(|item| (item.key.as_str(), item.included))
            .collect();
        assert_eq!(
            shown,
            [
                ("labs.filler_words", true),
                ("dictation_processing.server.runs_on", false),
                ("meetings.retention", false),
            ]
        );
        assert!(super::plan(Vec::new(), &settings, &state).is_none());
    }
}
