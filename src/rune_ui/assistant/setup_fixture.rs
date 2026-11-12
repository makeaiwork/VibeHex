//! Guided setup as scenes (the Setup board), on the owner's own settings
//! (`setup::sample`): what is set now and round one preselected from it;
//! a follow-up round; the plan grouped by area, with a "Yes" checked, a
//! value the person chose unchecked, an engine not ready yet, and
//! "Already set" unfolded; and a plan applied with one row failed and a
//! speech model downloading.

use std::time::Instant;

use serde_json::{Map, Value, json};

use super::super::Ui;
use super::changes::At;
use super::fixture::{AT, MINUTE, open};
use super::fixture_cards::{conversation, item};
use crate::assistant::changes::catalog::Class;
use crate::assistant::changes::{Card, CardKind, Decision, Item, current, parts};
use crate::assistant::facts::{Fact, Facts, State};
use crate::assistant::recordings::Step;
use crate::assistant::recordings::usage::{self, Usage};
use crate::assistant::setup::past::Setup;
use crate::assistant::setup::since::{self, Since};
use crate::assistant::setup::{self, Ask, follow};
use crate::assistant::store::Turn;
use crate::desktop::i18n::tr;
use crate::dictation::ShortcutMode;
use crate::transcription::models::{ModelPreparationStage, TranscriptionModelId};

/// The owner's settings, and the state a scene plays: a Mac, Codex
/// signed in, no OpenRouter key.
fn sample(ui: &mut Ui) -> State {
    setup::sample(&mut ui.settings);
    ui.settings.shortcut_mode = ShortcutMode::Toggle;
    State {
        os: "macos",
        codex_signed_in: Fact::Known(true),
        opencode_running: Fact::Known(false),
        openrouter_key: Fact::Known(false),
        ..State::default()
    }
}

/// Round one as the app asks it, preselected from the settings.
fn first(ui: &Ui, state: &State) -> Vec<Ask> {
    let mut asks = setup::questions("macos");
    setup::preselect(&mut asks, &ui.settings, state);
    asks
}

/// Round one's turn: what is set now above it.
fn opening(ui: &Ui, asks: Vec<Ask>) -> Turn {
    Turn {
        now: setup::now::now(&Facts::of(&ui.settings, &State::default())),
        asks,
        created_ms: AT,
        ..Turn::default()
    }
}

/// Round one answered: code, calls, and files, in Russian with tech terms,
/// dictation on this Mac and notes on `notes`.
fn answered(ui: &Ui, state: &State, notes: &str) -> Vec<Ask> {
    let mut asks = first(ui, state);
    for ask in &mut asks {
        if ask.id == "use" {
            ask.chosen = ["dictation_any", "dictation_code", "meetings", "files"]
                .map(String::from)
                .to_vec();
        }
        if ask.id == "notes" {
            ask.chosen = vec![notes.into()];
        }
        ask.done = true;
    }
    asks
}

/// A follow-up round of four, as the app writes them.
fn round(answered: bool) -> Vec<Ask> {
    let ask = |key: &str, q: &str, hint: &str| {
        let mut ask = Ask {
            id: key.into(),
            q: q.into(),
            hint: hint.into(),
            own: true,
            ..Ask::default()
        };
        ask.options = vec![
            setup::Choice {
                id: "yes".into(),
                label: follow::YES.into(),
                changes: vec![(key.into(), Value::Bool(true))],
                ..setup::Choice::default()
            },
            setup::Choice {
                id: "no".into(),
                label: follow::NO.into(),
                ..setup::Choice::default()
            },
        ];
        ask
    };
    let mut asks = vec![
        ask(
            "labs.stop_shortcut",
            "Finish a dictation by pressing Return instead of your shortcut?",
            "Stop shortcut",
        ),
        ask(
            "labs.spoken_symbols",
            "Say comma, hashtag, or underscore and get the symbol?",
            "Spoken symbols",
        ),
        ask(
            "labs.stop_when_call_ends",
            "Stop the recording by itself when the call ends?",
            "Stop when the call ends",
        ),
        ask(
            "labs.filler_words",
            "Remove ums and ahs from what you dictate?",
            "Filler words",
        ),
    ];
    asks[0].chosen = vec!["yes".into()];
    asks[0].touched = true;
    if answered {
        asks[2].chosen = vec!["yes".into()];
        for ask in &mut asks {
            ask.done = true;
        }
    }
    asks
}

/// What is set now, and round one preselected from it: code and calls
/// and files, Russian with English tech terms, dictation on this Mac,
/// notes on Codex.
pub(in super::super) fn setup_ask(ui: &mut Ui) {
    open(ui);
    let state = sample(ui);
    // The owner dictates Russian: Assistant's words are Russian.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let asks = first(ui, &state);
    let turn = opening(ui, asks);
    conversation(ui, "Set up for me", vec![turn], &[]);
}

/// Round one answered, and the first follow-up round open, one answered;
/// two answered in an earlier setup, unfolded under "Answered before".
pub(in super::super) fn setup_follow(ui: &mut Ui) {
    open(ui);
    let state = sample(ui);
    // The owner dictates Russian: Assistant's words are Russian.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let mut asks = round(false);
    asks.truncate(2);
    let mut before = round(true);
    for (ask, (key, q, hint, chosen)) in before.iter_mut().zip([
        (
            "labs.screenshots",
            "Take screenshots you can mark up and talk over?",
            "Screenshots",
            "no",
        ),
        (
            "labs.smart_spacing",
            "Add a space automatically when you dictate right after a word?",
            "Smart spacing",
            "yes",
        ),
    ]) {
        ask.id = key.into();
        ask.q = q.into();
        ask.hint = hint.into();
        ask.options[0].changes = vec![(key.into(), Value::Bool(true))];
        ask.chosen = vec![chosen.into()];
        ask.done = false;
        ask.before = true;
    }
    before.truncate(2);
    asks.extend(before);
    let turns = vec![
        opening(ui, answered(ui, &state, "codex")),
        Turn {
            asks,
            created_ms: AT + MINUTE,
            ..Turn::default()
        },
    ];
    let id = conversation(ui, "Set up for me", turns, &[]);
    super::ask::unfold_before(
        ui,
        super::ask::At {
            conversation: id,
            turn: 1,
        },
    );
}

/// A follow-up round in Russian, as a model asks it: a long question
/// wraps inside the card, and every hint sits on its own line under its
/// question.
pub(in super::super) fn setup_follow_ru(ui: &mut Ui) {
    open(ui);
    let state = sample(ui);
    ui.settings.ui_language = Some("ru".into());
    crate::desktop::i18n::apply(Some("ru"));
    let mut asks = round(false);
    asks.truncate(2);
    for (ask, (key, q, hint)) in asks.iter_mut().zip([
        (
            "labs.screenshots",
            "Делать скриншоты, на которых можно рисовать и которые можно комментировать голосом?",
            "Screenshots",
        ),
        (
            "labs.caps_lock_dictation",
            "Запускать диктовку одним нажатием Caps Lock?",
            "Caps Lock dictation",
        ),
    ]) {
        ask.id = key.into();
        ask.q = q.into();
        ask.hint = crate::desktop::i18n::tr_window(hint).to_owned();
        ask.own = false;
        ask.options[0].changes = vec![(key.into(), Value::Bool(true))];
        // As a model's question is kept (`follow::reworded`): its hint in
        // the window's words, Yes and No the app's, in the language spoken
        // when it was asked.
        for option in &mut ask.options {
            option.label = crate::desktop::i18n::tr(&option.label).to_owned();
        }
    }
    let turns = vec![
        opening(ui, answered(ui, &state, "codex")),
        Turn {
            asks,
            created_ms: AT + MINUTE,
            ..Turn::default()
        },
    ];
    conversation(ui, "Настрой меня", turns, &[]);
}

/// The plan's rows: two follow-ups said yes to, the writer and Files on
/// OpenRouter without a key, and a model's line over a choice the person
/// made, unchecked. What fits stays under "Already set".
fn rows(ui: &Ui) -> (Vec<Item>, Vec<Item>) {
    let settings = &ui.settings;
    let mut spoken = item(
        "labs.spoken_symbols",
        json!(false),
        json!(true),
        "You said yes to this",
    );
    spoken.included = true;
    let mut mode = item(
        "shortcut_mode",
        json!("toggle"),
        json!("hold"),
        "Hold to talk suits short commands to an agent",
    );
    mode.yours = true;
    mode.included = false;
    let stop = item(
        "labs.stop_shortcut",
        json!(false),
        json!(true),
        "You said yes to this",
    );
    let calls = item(
        "labs.stop_when_call_ends",
        json!(false),
        json!(true),
        "You said yes to this",
    );
    let writer_now = current(settings, "meetings.notes.writer").unwrap_or(Value::Null);
    let mut writer = item(
        "meetings.notes.writer",
        writer_now,
        json!("openrouter"),
        "Notes written by a cloud model through OpenRouter",
    );
    writer.included = false;
    writer.note = "Add an OpenRouter key in Output › AI cleanup first".into();
    let mut files = item(
        "files.cleanup_engine",
        json!("codex"),
        json!("openrouter"),
        "Cleans up transcribed files there",
    );
    files.included = false;
    files.note = writer.note.clone();
    let kept = [
        "dictation_processing.server.runs_on",
        "labs.russian_vibe_coding",
        "labs.vocabulary_quick_add",
        "meetings.enabled",
        "labs.files",
        "transcription.model",
    ]
    .iter()
    .map(|key| {
        let now = current(settings, key).unwrap_or(Value::Null);
        Item {
            key: (*key).into(),
            from: now.clone(),
            to: now,
            ..Item::default()
        }
    })
    .collect();
    (vec![spoken, mode, stop, calls, writer, files], kept)
}

/// The answers turn: no bubble, the app's line, and the plan.
fn answers(asks: &[Ask], plan: Card) -> Turn {
    Turn {
        answer: tr("Here is what fits how you work. Untick what you don't need.").into(),
        answers: setup::answers(asks),
        cards: vec![plan],
        created_ms: AT + 2 * MINUTE,
        ..Turn::default()
    }
}

/// The plan grouped by area: Yes lines checked, "You chose" unchecked,
/// OpenRouter not ready, and "Already set" folded.
pub(in super::super) fn setup_plan(ui: &mut Ui) {
    plan_scene(ui, false);
}

/// The same plan with "Already set" unfolded.
pub(in super::super) fn setup_kept(ui: &mut Ui) {
    plan_scene(ui, true);
}

fn plan_scene(ui: &mut Ui, unfolded: bool) {
    open(ui);
    ui.platform.system_audio = true;
    let state = sample(ui);
    // The owner dictates Russian: Assistant's words are Russian.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let first = answered(ui, &state, "openrouter");
    let follow = round(true);
    let (items, kept) = rows(ui);
    let plan = Card {
        kind: CardKind::Plan,
        items,
        kept,
        ..Card::default()
    };
    let asked: Vec<Ask> = first.iter().chain(&follow).cloned().collect();
    let turns = vec![
        opening(ui, first),
        Turn {
            asks: follow,
            created_ms: AT + MINUTE,
            ..Turn::default()
        },
        answers(&asked, plan),
    ];
    let id = conversation(ui, "Set up for me", turns, &[]);
    if unfolded {
        ui.assistant.kept_open = vec![At {
            conversation: id,
            turn: 2,
            card: 0,
        }];
    }
}

/// The rows the applied scene saved, from a fresh install.
fn fresh_rows(ui: &mut Ui) -> Vec<Item> {
    let settings = &mut ui.settings;
    settings.transcription.model = TranscriptionModelId::ParakeetUnifiedEnglish;
    settings.meetings.enabled = false;
    settings.labs.russian_vibe_coding = false;
    let mut model = item(
        "transcription.model",
        json!("parakeet_unified_en"),
        json!("parakeet_v3"),
        "Hears Russian with English tech terms and reads your Vocabulary",
    );
    model.class = Class::Download;
    let mut meetings = item(
        "meetings.enabled",
        json!(false),
        json!(true),
        "Records calls, with a transcript and notes",
    );
    meetings.class = Class::Permission;
    let cleanup = current(settings, parts::CLEANUP).unwrap_or(Value::Null);
    vec![
        item(
            "labs.russian_vibe_coding",
            json!(false),
            json!(true),
            "Russian speech with English tech terms written as they should be",
        ),
        item(
            "labs.spoken_symbols",
            json!(false),
            json!(true),
            "You said yes to this",
        ),
        meetings,
        item(
            "meetings.notes.writer",
            Value::Null,
            json!("codex"),
            "Notes on your ChatGPT plan, good with long calls",
        ),
        model,
    ]
    .into_iter()
    .chain((cleanup == json!(false)).then(|| {
        item(
            parts::CLEANUP,
            json!(false),
            json!(true),
            "Every dictation is cleaned up before it pastes",
        )
    }))
    .collect()
}

/// Applied: four of five rows saved, Meetings' writer refused, and
/// Parakeet v3 downloading; Undo while the conversation stays open.
pub(in super::super) fn setup_result(ui: &mut Ui) {
    open(ui);
    ui.platform.system_audio = true;
    let state = sample(ui);
    // The owner dictates Russian: Assistant's words are Russian.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    let first = answered(ui, &state, "codex");
    let mut items = fresh_rows(ui);
    items[3].failed = Some(crate::desktop::i18n::tr("Codex isn't installed on this Mac").into());
    let settings = &mut ui.settings;
    settings.labs.russian_vibe_coding = true;
    settings.labs.spoken_symbols = true;
    settings.meetings.enabled = true;
    let mut before = Map::new();
    let mut after = Map::new();
    for (key, old, new) in [
        ("labs.russian_vibe_coding", json!(false), json!(true)),
        ("labs.spoken_symbols", json!(false), json!(true)),
        ("meetings.enabled", json!(false), json!(true)),
    ] {
        before.insert(key.into(), old);
        after.insert(key.into(), new);
    }
    if let Some(view) = ui.view.as_mut() {
        let model = TranscriptionModelId::ParakeetV3;
        let bytes = crate::transcription::models::definition(model)
            .download_bytes()
            .unwrap_or(0);
        view.transcription.preparing = Some(model);
        view.transcription.downloaded_bytes = bytes * 42 / 100;
        view.transcription.preparation_stage = Some(ModelPreparationStage::Downloading);
        ui.preparing = Some((model, Instant::now()));
    }
    let plan = Card {
        kind: CardKind::Plan,
        items,
        decision: Decision::Applied {
            before,
            after,
            at_ms: AT + 3 * MINUTE,
        },
        ..Card::default()
    };
    let turns = vec![opening(ui, first.clone()), answers(&first, plan)];
    conversation(ui, "Set up for me", turns, &[(1, 0)]);
}

/// "Set me up like Denis" typed on a fresh install: his settings as one
/// plan, what fits folded, the keep-meetings time unchecked.
pub(in super::super) fn setup_preset(ui: &mut Ui) {
    open(ui);
    // Offered with Russian recognition alone.
    ui.settings.transcription.language = "ru".into();
    super::preset::begin(ui, "Сделай настройки как у Дениса".into());
    dated(ui);
}

/// His plan put off, then his shortcuts and whether they suit.
pub(in super::super) fn setup_preset_shortcuts(ui: &mut Ui) {
    open(ui);
    ui.settings.transcription.language = "ru".into();
    super::preset::begin(ui, String::new());
    if let Some(conversation) = ui.assistant.open {
        let at = At {
            conversation,
            turn: 0,
            card: 0,
        };
        super::changes::decline(ui, at);
    }
    dated(ui);
}

/// A returning setup on Codex: the owner set up a month ago and declined
/// a Labs feature then; what is set now, the updates since in one line
/// and what his use shows as a short list, then what the app read folded
/// to one line, the model's suggestions, and round one folded with his
/// answers. The terms Vocabulary lacks wait for the plan.
pub(in super::super) fn setup_returning(ui: &mut Ui) {
    let (turns, _) = returning(ui);
    conversation(ui, "Set up for me", turns, &[]);
}

/// The returning setup's plan: no setting left to change, its terms a row
/// each under why they're offered (the sure ones checked), the last one
/// being edited, and "Already set" folded.
pub(in super::super) fn setup_returning_plan(ui: &mut Ui) {
    let (mut turns, state) = returning(ui);
    // The owner dictates Russian: the plan's words are Russian too.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    // Round one answered as preselected.
    if let Some(look) = turns.last_mut() {
        look.asks.iter_mut().for_each(|ask| ask.done = true);
    }
    let look = turns.last().cloned().unwrap_or_default();
    let answered = setup::answers(&look.asks);
    let plan = setup::plan::with_terms(Vec::new(), &answered, &look.terms, &ui.settings, &state)
        .unwrap_or_default();
    let edited = plan.items.len().saturating_sub(1);
    turns.push(answers(&look.asks, plan));
    let id = conversation(ui, "Set up for me", turns, &[]);
    let at = At {
        conversation: id,
        turn: 2,
        card: 0,
    };
    super::vocab::edit(ui, at, edited);
    ui.assistant.vocab_sounds = rune::TextEdit::new("супабейс, супабэйз".to_owned());
}

/// The returning setup's opening and look, and the state it plays.
fn returning(ui: &mut Ui) -> (Vec<Turn>, State) {
    open(ui);
    let state = sample(ui);
    // The owner dictates Russian: Assistant's words are Russian.
    let _speaking = crate::assistant::language::speak(&ui.settings);
    // The pane reads the settings as drafted: its Vocabulary too.
    let terms = ui.settings.vocabulary.clone();
    ui.vocabulary.reload(&terms);
    let asks = answered(ui, &state, "codex");
    let last = Setup {
        at_ms: AT - 30 * 24 * 60 * MINUTE,
        answers: setup::answers(&asks),
        declined: vec![setup::past::Line {
            key: "labs.playback".into(),
            to: Value::Bool(true),
            from: Value::Bool(false),
        }],
        ..Setup::default()
    };
    ui.assistant.store.finish(last.clone());
    let usage = Usage {
        dictations: 412,
        last_30_days: 138,
        apps: vec![
            ("Ghostty".into(), 160),
            ("Cursor".into(), 31),
            ("Telegram".into(), 88),
        ],
        code_share: 46,
        long_share: 27,
        median_words: 34,
        terms: vec![
            usage::Term {
                word: "GitHub".into(),
                count: 21,
                heard: vec!["гитхаб".into()],
                fixed: 21,
            },
            usage::Term {
                word: "Claude Code".into(),
                count: 14,
                heard: vec!["клод код".into()],
                fixed: 14,
            },
            usage::Term {
                word: "Supabase".into(),
                count: 6,
                heard: vec!["супабейс".into()],
                fixed: 2,
            },
        ],
        fixes: vec![
            usage::Fix {
                heard: "гитхаб".into(),
                written: "GitHub".into(),
                count: 21,
            },
            usage::Fix {
                heard: "клод код".into(),
                written: "Claude Code".into(),
                count: 14,
            },
        ],
        meetings: 23,
        meeting_languages: vec![(usage::Language::Russian, 19), (usage::Language::Latin, 4)],
    };
    let release = |version: &str, title: &str| since::Release {
        version: version.into(),
        title: title.into(),
        ..since::Release::default()
    };
    let since = Since {
        last: Some(last),
        releases: vec![
            release(
                "3.3.0-vibe.6",
                "Files, Stop shortcut, Listen to the recording, and AI cleanup on Codex",
            ),
            release(
                "next",
                "Assistant, VibeHex in Russian, Screenshots, and more speech models",
            ),
        ],
        from: None,
        usage: Some(usage),
    };
    // Round one as `ask::start` preselects it: as answered last time.
    let mut asks = first(ui, &state);
    since::preselect(&mut asks, &since);
    setup::preselect(&mut asks, &ui.settings, &state);
    let usage = since.usage.as_ref();
    let turns = vec![
        Turn {
            now: setup::now::now(&Facts::of(&ui.settings, &State::default())),
            since: since::paragraph(&since),
            created_ms: AT,
            ..Turn::default()
        },
        // On Codex the opening looks first: what the app read, the model's
        // suggestions, then round one folded.
        Turn {
            answer: SUGGESTIONS.into(),
            model: "GPT-6.1-Sol".into(),
            seconds: 8.0,
            steps: vec![Step::Looked {
                dictations: usage.map_or(0, |usage| usage.dictations),
                meetings: usage.map_or(0, |usage| usage.meetings),
            }],
            asks,
            folded: true,
            facts: since::prompt(&since),
            terms: usage.map(since::candidates).unwrap_or_default(),
            created_ms: AT + MINUTE,
            ..Turn::default()
        },
    ];
    (turns, state)
}

/// What a model suggests from the returning scene's use, written in the
/// owner's default language as the prompt asks.
const SUGGESTIONS: &str = "- Половина диктовок уходит в Ghostty и Cursor, а очистка раз за разом исправляет гитхаб → GitHub: с ним в Vocabulary модель распознавания сразу напишет его верно.
- 27% диктовок длинные. Codex очищает их лучше, чем этот Mac, ценой нескольких секунд.
- С прошлой настройки появились Files: они превращают уже имеющиеся записи в текст.";

fn dated(ui: &mut Ui) {
    if let Some(id) = ui.assistant.open {
        ui.assistant.store.date(id, AT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Scene = (&'static str, fn(&mut Ui));

    #[test]
    fn every_setup_scene_has_nothing_left_to_save() {
        let scenes: [Scene; 10] = [
            ("setup-ask", setup_ask),
            ("setup-preset", setup_preset),
            ("setup-preset-shortcuts", setup_preset_shortcuts),
            ("setup-follow", setup_follow),
            ("setup-follow-ru", setup_follow_ru),
            ("setup-plan", setup_plan),
            ("setup-kept", setup_kept),
            ("setup-result", setup_result),
            ("setup-returning", setup_returning),
            ("setup-returning-plan", setup_returning_plan),
        ];
        for (name, scene) in scenes {
            let mut ui = super::super::super::scenes::mac_fixture();
            scene(&mut ui);
            assert!(ui.drafted() == ui.settings, "{name} has a draft to save");
        }
    }

    /// The opening scene's chips are the owner's: code, calls, and files;
    /// Russian with tech terms; dictation on this Mac; notes on Codex; and
    /// the server LM Studio's port names.
    #[test]
    fn the_opening_scene_is_preselected_from_the_owners_settings() {
        let mut ui = super::super::super::scenes::mac_fixture();
        setup_ask(&mut ui);
        let turn = &ui.assistant.conversation().unwrap().turns[0];
        let chosen = |id: &str| -> Vec<String> {
            turn.asks
                .iter()
                .find(|ask| ask.id == id)
                .unwrap()
                .chosen
                .clone()
        };
        assert_eq!(
            chosen("use"),
            ["dictation_any", "dictation_code", "meetings", "files"]
        );
        assert_eq!(chosen("language"), ["ru_tech"]);
        assert_eq!(chosen("cleanup"), ["local"]);
        assert_eq!(chosen("notes"), ["codex"]);
        let cleanup = turn.asks.iter().find(|ask| ask.id == "cleanup").unwrap();
        let server = cleanup.options.iter().find(|o| o.id == "custom").unwrap();
        assert_eq!(server.extra, "LM Studio");
        assert!(turn.now.contains("Orukeet"), "{}", turn.now);
        assert!(turn.now.contains("94"), "{}", turn.now);
    }
}
