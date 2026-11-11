//! The Assistant's setting-change cards as scenes: every state a card
//! takes, the applied card with Undo beside a change still to decide, a
//! change with what it turns on beside, and the placeholder while a change
//! is read. The window fits four cards or so, so the states spread over
//! four scenes. A state this computer can't take on in a scene (an
//! organization's lock, Codex signed out) is set by `shown_as`.

use std::time::Instant;

use serde_json::{Map, Value, json};

use super::super::Ui;
use super::changes::At;
use super::fixture::{AT, MINUTE, open, turn};
use crate::app_settings::CleanupRunsOn;
use crate::assistant::changes::catalog::Class;
use crate::assistant::changes::state::Shown;
use crate::assistant::changes::{Card, CardKind, Decision, Item, Refused, Why};
use crate::assistant::store::Turn;
use crate::assistant::worker::Answering;
use crate::transcription::models::{ModelPreparationStage, TranscriptionModelId};

fn item(key: &str, from: Value, to: Value, why: &str) -> Item {
    Item {
        key: key.into(),
        from,
        to,
        why: why.into(),
        included: true,
        ..Item::default()
    }
}

fn single(key: &str, from: Value, to: Value, why: &str) -> Card {
    Card {
        items: vec![item(key, from, to, why)],
        ..Card::default()
    }
}

/// `card`, saved: its key went from its old value to its new one.
fn saved(mut card: Card) -> Card {
    let first = &card.items[0];
    card.decision = Decision::Applied {
        before: Map::from_iter([(first.key.clone(), first.from.clone())]),
        after: Map::from_iter([(first.key.clone(), first.to.clone())]),
        at_ms: AT + MINUTE,
    };
    card
}

fn declined(mut card: Card) -> Card {
    card.decision = Decision::Declined;
    card
}

/// A conversation of `turns`, open, with Undo for the cards at `undo`.
/// The editors take the scene's settings too, or their next draft save
/// would write the fixture's back.
fn conversation(ui: &mut Ui, name: &str, turns: Vec<Turn>, undo: &[(usize, usize)]) -> u32 {
    let settings = ui.settings.clone();
    ui.modes.reload(&settings);
    ui.vocabulary.reload(&settings.vocabulary);
    let store = &mut ui.assistant.store;
    let id = store.start(name, None, None);
    store.date(id, AT);
    for turn in turns {
        store.push(id, turn);
    }
    ui.assistant.open = Some(id);
    ui.assistant.undo_for = Some(id);
    ui.assistant.undo = undo
        .iter()
        .map(|&(turn, card)| At {
            conversation: id,
            turn,
            card,
        })
        .collect();
    id
}

fn with_cards(mut turn: Turn, cards: Vec<Card>) -> Turn {
    turn.cards = cards;
    turn
}

/// States 1 to 4: suggested, applied with Undo, declined, and a value
/// moved since it was suggested.
pub(in super::super) fn cards(ui: &mut Ui) {
    open(ui);
    let labs = &mut ui.settings.labs;
    labs.smart_spacing = false;
    labs.vocabulary_quick_add = true;
    labs.terminal_paste_shortcut = false;
    labs.ai_cleanup_server = true;
    ui.settings.dictation_processing.server.runs_on = CleanupRunsOn::Openrouter;
    let question = "What should I turn on so dictation comes out cleaner?";
    let answer = turn(
        question,
        "Two things help most. **Smart spacing** puts in the space you keep typing by hand, and AI cleanup on **Codex** fixes punctuation on your ChatGPT plan.",
        &["labs/smart-spacing", "topics/ai-cleanup"],
        1,
    );
    let cards = vec![
        single(
            "labs.smart_spacing",
            json!(false),
            json!(true),
            "Adds a space before dictation pasted right after a word",
        ),
        saved(single(
            "labs.vocabulary_quick_add",
            json!(false),
            json!(true),
            "",
        )),
        declined(single(
            "labs.terminal_paste_shortcut",
            json!(false),
            json!(true),
            "",
        )),
        single(
            "dictation_processing.server.runs_on",
            json!("opencode"),
            json!("codex"),
            "Your ChatGPT plan covers it",
        ),
    ];
    conversation(ui, question, vec![with_cards(answer, cards)], &[(0, 1)]);
}

/// States 5 and 6: a choice, and a speech model downloading.
pub(in super::super) fn cards_more(ui: &mut Ui) {
    open(ui);
    ui.settings.meetings.enabled = true;
    ui.settings.meetings.notes.writer = None;
    ui.settings.transcription.model = TranscriptionModelId::ParakeetV3;
    if let Some(view) = ui.view.as_mut() {
        let model = TranscriptionModelId::GigaAmV3;
        let bytes = crate::transcription::models::definition(model)
            .download_bytes()
            .unwrap_or(0);
        view.transcription.preparing = Some(model);
        view.transcription.downloaded_bytes = bytes * 42 / 100;
        view.transcription.preparation_stage = Some(ModelPreparationStage::Downloading);
        ui.preparing = Some((model, Instant::now()));
    }
    let meetings = "Who should write my meeting notes in Russian?";
    let mut first = turn(
        meetings,
        "**Meetings** writes notes when a call ends. For Russian, Codex writes better notes than Apple Intelligence.",
        &["topics/meetings"],
        1,
    );
    let writer = "meetings.notes.writer";
    let now = crate::assistant::changes::current(&ui.settings, writer).unwrap_or(Value::Null);
    first.cards.push(Card {
        kind: CardKind::Choice {
            options: vec![now.clone(), json!("codex:gpt-6.1-sol"), json!("openrouter")],
            chosen: 1,
            suggested: 1,
        },
        items: vec![item(
            writer,
            now,
            json!("codex:gpt-6.1-sol"),
            "Writes Russian well, and it's connected already",
        )],
        ..Card::default()
    });
    let model = "Which speech model is best for Russian?";
    let mut download = saved(single(
        "transcription.model",
        json!("parakeet_v3"),
        json!("gigaam_v3"),
        "",
    ));
    download.items[0].class = Class::Download;
    let second = with_cards(
        turn(
            model,
            "**GigaAM v3** is made for Russian and runs on this Mac.",
            &["topics/speech-models"],
            2,
        ),
        vec![download],
    );
    conversation(ui, meetings, vec![first, second], &[]);
}

/// States 7 to 10: a permission only macOS grants, a sign-in only Codex
/// does, a setting an organization manages, and a setting this VibeHex
/// doesn't have.
pub(in super::super) fn cards_held(ui: &mut Ui) {
    open(ui);
    ui.settings.meetings.enabled = true;
    ui.settings.labs.files = false;
    ui.platform.system_audio = false;
    ui.settings.labs.ai_cleanup_server = true;
    ui.settings.dictation_processing.server.runs_on = CleanupRunsOn::Opencode;
    let question = "Record my calls, clean up on my ChatGPT plan, and transcribe files";
    let mut answer = with_cards(
        turn(
            question,
            "**Meetings** records calls, AI cleanup runs on **Codex** with your ChatGPT plan, and **Files** transcribes recordings you drop in.",
            &["labs/meetings", "topics/codex"],
            1,
        ),
        vec![
            saved(single("meetings.enabled", json!(false), json!(true), "")),
            single(
                "dictation_processing.server.runs_on",
                json!("opencode"),
                json!("codex"),
                "",
            ),
            single("labs.files", json!(false), json!(true), ""),
        ],
    );
    answer.refused = vec![Refused {
        line: r#"{"key":"meetings.auto_record","value":true}"#.into(),
        key: Some("meetings.auto_record".into()),
        why: Why::Unknown,
    }];
    let id = conversation(ui, question, vec![answer], &[]);
    let at = |card| At {
        conversation: id,
        turn: 0,
        card,
    };
    ui.assistant.shown_as = vec![
        (at(1), Shown::SignIn),
        (
            at(2),
            Shown::Blocked {
                managed: true,
                why: String::new(),
            },
        ),
    ];
}

const INSTRUCTIONS: &str = "Fix punctuation and obvious recognition errors.\nKeep English terms as the speaker said them.\nDo not add anything that was not said.";

/// State 11 and a shorter retention: instructions line by line, and older
/// meetings named before they go.
pub(in super::super) fn cards_prompt(ui: &mut Ui) {
    open(ui);
    ui.settings.meetings.enabled = true;
    ui.settings
        .dictation_processing
        .default_mode
        .post_processing
        .prompt = INSTRUCTIONS.into();
    let from = crate::assistant::changes::current(&ui.settings, "meetings.retention")
        .unwrap_or(Value::Null);
    let keep = "Keep meetings for a month only";
    let first = with_cards(
        turn(
            keep,
            "**Meetings › Settings › Keep recordings** sets it. Older ones move to the Trash, so they can still come back from there.",
            &["topics/meetings"],
            1,
        ),
        vec![single(
            "meetings.retention",
            from,
            json!("month"),
            "Keeps the last month of calls",
        )],
    );
    let terms = "Keep Kubernetes and kubectl in Latin letters, and never translate code";
    let prompt = "dictation_processing.default_mode.prompt";
    let second = with_cards(
        turn(
            terms,
            "That's for the Global mode's instructions in **Output › AI cleanup**. Here is the change:",
            &["topics/output-and-instructions"],
            2,
        ),
        vec![Card {
            kind: CardKind::Prompt,
            items: vec![item(
                prompt,
                json!(INSTRUCTIONS),
                json!(
                    "Fix punctuation and obvious recognition errors.\nKeep English terms in Latin script: Kubernetes, kubectl, pull request.\nNever translate code, commands, or file names.\nDo not add anything that was not said."
                ),
                "",
            )],
            ..Card::default()
        }],
    );
    conversation(ui, keep, vec![first, second], &[]);
}

/// The Main mockup: Meetings turned on, with Undo, and Zoom's Always
/// record still to decide; then a follow-up.
pub(in super::super) fn applied(ui: &mut Ui) {
    open(ui);
    ui.settings.meetings.enabled = true;
    ui.settings.meetings.providers.clear();
    let question = "Хочу, чтобы созвоны в Zoom записывались сами, без вопросов. Так можно?";
    let first = with_cards(
        turn(
            question,
            "Да. Это делает раздел **Meetings**, сейчас он выключен в Labs. Нужны два шага:\n\n1. Включить Meetings в Labs.\n2. Для Zoom поставить **Always record** вместо Ask.\n\nЗаписи и заметки остаются на этом Mac. При первой записи macOS один раз спросит разрешение на запись системного звука.",
            &["labs/meetings", "topics/meetings"],
            1,
        ),
        vec![
            saved(single("meetings.enabled", json!(false), json!(true), "")),
            single(
                "meetings.providers.zoom.policy",
                json!("ask"),
                json!("always_record"),
                "Каждый созвон в Zoom начнёт записываться сам. Остальные приложения остаются на Ask.",
            ),
        ],
    );
    let follow = "А кто будет писать конспект? И на русском нормально?";
    let second = turn(
        follow,
        "Конспект пишет модель из **Meetings › Settings › Notes**. Сейчас там Apple Intelligence: он работает без интернета, но по-русски пишет слабо. Для русских встреч лучше Codex, он у вас уже подключён.",
        &["topics/meetings"],
        2,
    );
    conversation(ui, question, vec![first, second], &[(0, 0)]);
}

/// The Web mockup's card: a server's URL, which needs a Labs feature on
/// too, saved together with Change both.
pub(in super::super) fn change_both(ui: &mut Ui) {
    open(ui);
    ui.settings.labs.ai_cleanup_server = false;
    ui.settings.dictation_processing.server.url = String::new();
    let question = "У меня стоит LM Studio. Как сделать, чтобы VibeHex чистил диктовку через него?";
    let mut card = single(
        "dictation_processing.server.url",
        json!(""),
        json!("http://localhost:1234/v1"),
        "",
    );
    card.items[0].requires = vec![("labs.ai_cleanup_server".into(), json!(true))];
    let answer = with_cards(
        turn(
            question,
            "Со стороны VibeHex всё есть, нужен только адрес сервера. Со стороны LM Studio:\n\n1. Откройте вкладку **Developer** и включите **Start server**.\n2. Загрузите модель, например Qwen3 4B Instruct.\n3. Сервер будет слушать `localhost:1234`.",
            &["topics/ai-cleanup", "labs/ai-cleanup-server"],
            1,
        ),
        vec![card],
    );
    conversation(ui, question, vec![answer], &[]);
}

/// A change block started streaming: the answer so far, then "Preparing a
/// change…" where its card will be, never the raw lines.
pub(in super::super) fn change_coming(ui: &mut Ui) {
    open(ui);
    ui.settings.labs.smart_spacing = false;
    let question = "Some words come out glued to the one before. Can VibeHex fix that?";
    let id = conversation(ui, question, Vec::new(), &[]);
    ui.assistant.answering = Some(Answering {
        conversation: id,
        question: question.into(),
        draft: "Yes. **Smart spacing** in Labs adds a space when you dictate right after a word, so it doesn't stick to it.".into(),
        started: Instant::now(),
        steps: Vec::new(),
        searching: false,
        cards_coming: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    type Scene = (&'static str, fn(&mut Ui));

    /// A scene's settings are what its editors hold, so no draft save
    /// changes them a moment in.
    #[test]
    fn every_scene_has_nothing_left_to_save() {
        let scenes: [Scene; 7] = [
            ("cards", cards),
            ("cards-more", cards_more),
            ("cards-held", cards_held),
            ("cards-prompt", cards_prompt),
            ("applied", applied),
            ("change-both", change_both),
            ("change-coming", change_coming),
        ];
        for (name, scene) in scenes {
            let mut ui = super::super::super::scenes::mac_fixture();
            scene(&mut ui);
            assert!(ui.drafted() == ui.settings, "{name} has a draft to save");
        }
    }
}
