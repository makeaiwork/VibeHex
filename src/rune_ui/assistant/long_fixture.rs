//! Long content: what a model writes when it writes much, in the language
//! asked. The other fixtures are short and mostly English, and the layout
//! faults the owner found were in none of them. The layout matrix
//! (`layout::matrix`) lays every surface out with it; three of its cells
//! are scenes here, with golden images, and so is a plan whose values are
//! long: cleanup instructions and a model's name in their chips.

use serde_json::{Value, json};

use super::super::Ui;
use crate::assistant::changes::vocab::KEY as TERM;
use crate::assistant::changes::{Card, CardKind, Item};
use crate::assistant::setup::Ask;
use crate::assistant::store::Turn;
use crate::dictation::cleanup_server::codex::Model;

/// A model's name as long as providers make them.
pub(super) const MODEL: &str = "GPT-6.1-Sol-Extended-Reasoning-Preview-2026-11-12";
const MODEL_ID: &str = "gpt-6.1-sol-extended-reasoning-preview-2026-11-12";

/// How many terms a plan of a long-time user offers.
pub(super) const TERMS: usize = 95;

/// A language's long words.
pub(super) struct Words {
    /// A question as long as a model asks one.
    title: &'static str,
    hint: &'static str,
    why: &'static str,
    option: &'static str,
    note: &'static str,
    paragraph: &'static str,
    /// A term too long for its column.
    term: &'static str,
    terms: &'static [&'static str],
    sounds: &'static [&'static str],
}

const ENGLISH: Words = Words {
    title: "When you dictate a long message to a colleague and stop to think in the middle of a sentence, should the pause be taken out of the recording before it is recognised?",
    hint: "Skip long pauses, together with spoken formatting",
    why: "You said the notes of long calls come out with filler words and repeated starts, and this takes them out before the notes are written",
    option: "Yes, and for every meeting recorded from now on too",
    note: "Applies to dictation and to the notes of calls recorded from now on, never to what is already kept",
    paragraph: "Recordings and notes stay on this computer. The first time, the system asks once to allow recording system audio, and after that every detected call starts by itself: **Meetings › Settings › Detected calls** lists the apps, each with Ask, Always record, or Never.",
    term: "Model Context Protocol server registry",
    terms: &[
        "Kubernetes",
        "kubectl",
        "PostgreSQL",
        "Grafana",
        "Tailscale",
        "Supabase",
    ],
    sounds: &[
        "model context protocol server registry",
        "em see pee server registry",
        "model contexts protocol",
        "em cee pee",
        "model context",
        "protocol server",
        "context protocol registry",
        "the registry of servers",
    ],
};

const RUSSIAN: Words = Words {
    title: "Когда вы диктуете длинное сообщение коллеге и останавливаетесь подумать посреди предложения, нужно ли вырезать эту паузу из записи до распознавания?",
    hint: "Пропуск длинных пауз вместе с голосовым форматированием",
    why: "Вы сказали, что заметки длинных созвонов выходят со словами-паразитами и повторами, и это убирает их до того, как заметки написаны",
    option: "Да, и для каждой встречи, записанной с этого момента",
    note: "Относится к диктовке и к заметкам созвонов, записанных с этого момента, но не к тому, что уже сохранено",
    paragraph: "Записи и заметки остаются на этом компьютере. В первый раз система один раз спросит разрешение на запись системного звука, а дальше каждый найденный созвон начинается сам: в **Meetings › Settings › Detected calls** перечислены приложения, у каждого Ask, Always record или Never.",
    term: "Реестр серверов Model Context Protocol",
    terms: &[
        "Kubernetes",
        "kubectl",
        "PostgreSQL",
        "Grafana",
        "Tailscale",
        "Supabase",
    ],
    sounds: &[
        "модел контекст протокол сервер реджистри",
        "эм си пи сервер реджистри",
        "модель контекст протокол",
        "эм си пи",
        "модел контекст",
        "протокол сервер",
        "контекст протокол реджистри",
        "реестр серверов",
    ],
};

/// German, for its words no line can break.
const GERMAN: Words = Words {
    title: "Soll bei Besprechungsaufzeichnungen die Rindfleischetikettierungsüberwachungsaufgabenübertragung automatisch als Wörterbucheintrag für die Spracherkennungsverbesserung vorgeschlagen werden?",
    hint: "Besprechungsaufzeichnungszusammenfassungen und Sprachformatierungseinstellungen",
    why: "Die Donaudampfschifffahrtsgesellschaftskapitänsbesprechungen enthalten viele Füllwörter und Wiederholungen, und diese Einstellung entfernt sie vor der Zusammenfassungserstellung",
    option: "Ja, und für jede Besprechungsaufzeichnung ab jetzt ebenfalls",
    note: "Gilt für Diktate und Besprechungszusammenfassungen ab jetzt, nie für bereits Gespeichertes",
    paragraph: "Aufzeichnungen und Zusammenfassungen bleiben auf diesem Computer. Beim ersten Mal fragt das System einmalig nach der Systemaudioaufzeichnungsberechtigung, danach startet jede erkannte Videokonferenzbesprechung selbstständig: **Meetings › Settings › Detected calls** listet die Anwendungen auf.",
    term: "Rindfleischetikettierungsüberwachungsaufgabenübertragungsgesetz",
    terms: &[
        "Kubernetes",
        "Datenschutzgrundverordnung",
        "PostgreSQL",
        "Geschwindigkeitsbegrenzung",
        "Tailscale",
        "Kraftfahrzeughaftpflichtversicherung",
    ],
    sounds: &[
        "rindfleischetikettierungsüberwachungsaufgabenübertragungsgesetz",
        "rindfleisch etikettierung",
        "überwachungsaufgabenübertragungsgesetz",
        "etikettierungsgesetz",
        "rindfleischgesetz",
        "aufgabenübertragung",
        "überwachungsaufgaben",
        "das lange gesetz",
    ],
};

/// The long words of a recognition language: German's, Russian's, or
/// English's for any other.
pub(super) fn words(language: &str) -> &'static Words {
    match language {
        "de" => &GERMAN,
        "ru" => &RUSSIAN,
        _ => &ENGLISH,
    }
}

fn term(words: &Words, n: usize) -> Item {
    let name = match n % 10 {
        9 => format!("{} {n}", words.term),
        _ => format!("{}{n}", words.terms[n % words.terms.len()]),
    };
    // One in four with every way it is heard, the rest with one or two.
    let sounds = match n % 4 {
        0 => words.sounds,
        _ => &words.sounds[..1 + n % 2],
    };
    Item {
        key: TERM.into(),
        to: json!({"term": name, "sounds_like": sounds}),
        included: n % 7 != 3,
        ..Item::default()
    }
}

fn model(value: &mut Value) {
    if value.as_str().is_some_and(|id| id.starts_with("codex:")) {
        *value = json!(format!("codex:{MODEL_ID}"));
    }
}

fn card(card: &mut Card, words: &Words) {
    let terms = card.items.iter().any(|item| item.key == TERM);
    card.items.retain(|item| item.key != TERM);
    for item in &mut card.items {
        if !item.why.is_empty() {
            item.why = words.why.into();
        }
        model(&mut item.to);
        // An entry already in Vocabulary, changed: its long name and
        // every way it is heard.
        if card.kind == CardKind::VocabularyEdit {
            item.to = json!({"term": words.term, "sounds_like": words.sounds});
        }
    }
    if let CardKind::Choice { options, .. } = &mut card.kind {
        options.iter_mut().for_each(model);
    }
    if terms {
        card.items.extend((0..TERMS).map(|n| term(words, n)));
    }
}

fn ask(ask: &mut Ask, words: &Words) {
    // The app's own questions are in its tables, each language's words
    // as long as they are.
    if ask.own {
        return;
    }
    ask.q = words.title.into();
    if !ask.hint.is_empty() {
        ask.hint = words.hint.into();
    }
    for (n, option) in ask.options.iter_mut().enumerate() {
        if n == 0 {
            option.label = words.option.into();
            option.note = words.note.into();
        }
        if !option.extra.is_empty() {
            option.extra = MODEL.into();
        }
    }
}

fn turn(turn: &mut Turn, words: &Words) {
    if !turn.question.is_empty() {
        turn.question = words.title.into();
    }
    if !turn.answer.is_empty() {
        turn.answer = format!(
            "{}\n\n{}\n\n1. {}\n2. **{}**: {}",
            turn.answer, words.paragraph, words.why, words.term, words.note
        );
    }
    if !turn.model.is_empty() {
        turn.model = MODEL.into();
    }
    for text in [&mut turn.now, &mut turn.since] {
        if !text.is_empty() {
            *text = format!("{text} {}", words.paragraph);
        }
    }
    if !turn.knew.is_empty() {
        turn.knew = format!(
            "{}\nmeetings.notes.writer = codex:{MODEL_ID} (you chose it)\nProposed and not offered:\n  term {} ({})",
            turn.knew, words.term, words.why
        );
    }
    turn.cards.iter_mut().for_each(|each| card(each, words));
    turn.asks.iter_mut().for_each(|each| ask(each, words));
}

/// Assistant speaking `language`: it follows speech recognition's.
pub(super) fn speak(ui: &mut Ui, language: &str) {
    ui.settings.transcription.model = crate::transcription::models::TranscriptionModelId::Orukeet;
    ui.settings.transcription.language = language.into();
}

/// Codex lists the model with the long name.
fn model_listed(ui: &mut Ui, description: &str) {
    ui.files.codex_models.push(Model {
        id: MODEL_ID.into(),
        name: MODEL.into(),
        description: description.into(),
        efforts: vec!["low".into(), "medium".into(), "high".into()],
        default_effort: "medium".into(),
        default: false,
    });
}

/// Makes everything a model wrote in what `ui` shows long, in `language`:
/// questions, answers, reasons, option labels, the model's name, and a
/// plan's or a card's terms, 95 of them.
pub(super) fn stretch(ui: &mut Ui, language: &str) {
    let words = words(language);
    model_listed(ui, words.note);
    if ui.settings.assistant.engine.is_some() {
        ui.settings.assistant.engine = Some(format!("codex:{MODEL_ID}"));
    }
    ui.assistant.store.each(|conversation| {
        conversation.name = words.title.chars().take(48).collect();
        for each in &mut conversation.turns {
            turn(each, words);
        }
    });
    if let Some(pending) = &mut ui.assistant.consent {
        pending.question = words.title.into();
    }
}

/// A long question being written in the field.
#[cfg(test)]
pub(super) fn draft(ui: &mut Ui, language: &str) {
    let words = words(language);
    ui.assistant.ask = rune::TextEdit::new(format!("{} {}", words.title, words.why));
}

/// A model's follow-up round in German: long questions and hints, an
/// option's label and note at their longest.
pub(in super::super) fn follow_german(ui: &mut Ui) {
    super::setup_fixture::setup_follow_ru(ui);
    speak(ui, "de");
    stretch(ui, "de");
}

/// A plan in Russian with 95 Vocabulary terms folded into its one row.
pub(in super::super) fn plan_russian(ui: &mut Ui) {
    super::terms_fixture::plan_terms(ui);
    speak(ui, "ru");
    stretch(ui, "ru");
}

/// A choice between models with long names, in Russian, as the narrowest
/// window shows it.
pub(in super::super) fn choice_russian(ui: &mut Ui) {
    super::fixture_cards::cards_more(ui);
    speak(ui, "ru");
    stretch(ui, "ru");
}

/// The instructions a plan replaces, and what it writes instead: long
/// enough that three lines don't hold them.
const INSTRUCTIONS_NOW: &str =
    "Rewrite the transcript into clear, natural text and keep every technical term as said.";
const INSTRUCTIONS: &str = "You clean up Russian dictation with English technical terms. Keep the \
     speaker's meaning and order. Remove filler words, false starts and repeats. Write product \
     and library names as their makers do: Kubernetes, PostgreSQL, GitHub, VibeHex. Never \
     translate a term, a command, a file name or a path. Keep numbers as digits. Split long \
     speech into short paragraphs, and turn a spoken list into a list. Return only the cleaned \
     text, with no comment before or after it.";

/// A plan whose values are long: the cleanup instructions, the notes'
/// writer with a long name, and one short switch. Its place.
fn long_values(ui: &mut Ui) -> super::changes::At {
    use super::fixture_cards::{conversation, item, with_cards};
    super::fixture::open(ui);
    model_listed(ui, "Latest workhorse model");
    ui.settings.labs.filler_words = false;
    let question = "Set up cleanup for how I dictate";
    let answer = super::fixture::turn(
        question,
        "Here is what fits how you work. Untick what you don't need.",
        &["topics/ai-cleanup"],
        1,
    );
    let card = Card {
        kind: CardKind::Plan,
        items: vec![
            item(
                crate::assistant::changes::parts::PROMPT,
                json!(INSTRUCTIONS_NOW),
                json!(INSTRUCTIONS),
                "Your dictation mixes Russian with English terms",
            ),
            item(
                "labs.filler_words",
                json!(false),
                json!(true),
                "Takes the ums and ahs out of what you dictate",
            ),
            item(
                "meetings.notes.writer",
                json!("opencode"),
                json!(format!("codex:{MODEL_ID}")),
                "Longer notes read better from a larger model",
            ),
        ],
        ..Card::default()
    };
    let id = conversation(ui, question, vec![with_cards(answer, vec![card])], &[]);
    super::changes::At {
        conversation: id,
        turn: 0,
        card: 0,
    }
}

/// Long values folded: three lines each, "Show all" where there is more.
pub(in super::super) fn values(ui: &mut Ui) {
    long_values(ui);
}

/// The same with the new instructions unfolded: all of them, "Show less".
pub(in super::super) fn values_open(ui: &mut Ui) {
    let at = long_values(ui);
    ui.assistant.values_open = vec![at.value(0, super::value::Slot::To)];
}
