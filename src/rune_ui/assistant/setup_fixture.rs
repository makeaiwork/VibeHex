//! Guided setup as scenes (the Setup board): a typed Russian request with
//! its first question open, the plan after the three answers, and the plan
//! applied with one row failed and the speech model downloading.

use std::time::Instant;

use serde_json::{Map, Value, json};

use super::super::Ui;
use super::fixture::{AT, MINUTE, open, turn};
use super::fixture_cards::{conversation, item};
use crate::app_settings::CleanupRunsOn;
use crate::assistant::changes::catalog::Class;
use crate::assistant::changes::{Card, CardKind, Decision, Item, current};
use crate::assistant::setup::{Answer, Ask, Choice};
use crate::assistant::store::Turn;
use crate::transcription::models::{ModelPreparationStage, TranscriptionModelId};

const REQUEST: &str = "Помоги мне настроить VibeHex под меня";

fn choice(id: &str, label: &str) -> Choice {
    Choice {
        id: id.into(),
        label: label.into(),
    }
}

/// The three questions as a model wrote them in Russian, answered as the
/// board shows, or not yet.
fn asks(answered: bool) -> Vec<Ask> {
    let ask = |id: &str, q: &str, hint: &str, multi, options, chosen: &[&str]| Ask {
        id: id.into(),
        q: q.into(),
        hint: hint.into(),
        multi,
        options,
        chosen: if answered {
            chosen.iter().map(|id| (*id).into()).collect()
        } else {
            Vec::new()
        },
        done: answered,
        own: false,
    };
    vec![
        ask(
            "use",
            "Для чего вам VibeHex?",
            "можно несколько",
            true,
            vec![
                choice("dictation_any", "Диктовка в любые приложения"),
                choice("dictation_code", "Диктовка в код и терминал"),
                choice("meetings", "Запись созвонов"),
                choice("files", "Расшифровка файлов"),
            ],
            &["dictation_any", "dictation_code", "meetings"],
        ),
        ask(
            "language",
            "На каком языке вы обычно говорите?",
            "",
            false,
            vec![
                choice("ru", "Русский"),
                choice("en", "English"),
                choice("mixed", "Оба, вперемешку"),
            ],
            &["ru"],
        ),
        ask(
            "cloud",
            "Можно отправлять текст в облачную модель?",
            "для чистки диктовки и конспектов",
            false,
            vec![
                choice("chatgpt", "Да, через мой ChatGPT"),
                choice("this_mac", "Только на этом Mac"),
                choice("no_cleanup", "Пока без чистки"),
            ],
            &["chatgpt"],
        ),
    ]
}

/// The request and the model's questions.
fn request(asks: Vec<Ask>) -> Turn {
    let mut first = turn(
        REQUEST,
        "Давайте настроим VibeHex под вас. Три коротких вопроса, потом один список изменений, и вы сами решите, что применить.",
        &[],
        1,
    );
    first.asks = asks;
    first
}

/// The first question open, two of its options chosen, and Continue.
pub(in super::super) fn setup_ask(ui: &mut Ui) {
    open(ui);
    let mut asks = asks(false);
    asks[0].chosen = vec!["dictation_code".into(), "meetings".into()];
    conversation(ui, REQUEST, vec![request(asks)], &[]);
}

/// The plan's rows as the board lists them, against the scene's settings.
fn rows(ui: &mut Ui) -> Vec<Item> {
    let settings = &mut ui.settings;
    settings.transcription.model = TranscriptionModelId::ParakeetV3;
    settings.labs.russian_vibe_coding = false;
    settings.labs.ai_cleanup_server = false;
    settings.dictation_processing.server.runs_on = CleanupRunsOn::Opencode;
    settings.meetings.enabled = false;
    settings.meetings.notes.writer = None;
    settings.labs.vocabulary_quick_add = false;
    let writer = current(settings, "meetings.notes.writer").unwrap_or(Value::Null);
    let mut model = item(
        "transcription.model",
        json!("parakeet_v3"),
        json!("gigaam_v3"),
        "Лучшая точность на русском",
    );
    model.class = Class::Download;
    let mut runs_on = item(
        "dictation_processing.server.runs_on",
        json!("opencode"),
        json!("codex"),
        "Чистка диктовки на вашем тарифе ChatGPT",
    );
    runs_on.requires = vec![("labs.ai_cleanup_server".into(), json!(true))];
    let mut meetings = item(
        "meetings.enabled",
        json!(false),
        json!(true),
        "Запись созвонов, транскрипт и конспект",
    );
    meetings.class = Class::Permission;
    let mut quick = item(
        "labs.vocabulary_quick_add",
        json!(false),
        json!(true),
        "Выделить ошибочное слово и добавить его в словарь одной клавишей. Пригодится, если ошибки повторяются",
    );
    quick.included = false;
    vec![
        model,
        item(
            "labs.russian_vibe_coding",
            json!(false),
            json!(true),
            "Словарь и правила для русской речи с английскими терминами. Выключение вернёт всё как было",
        ),
        runs_on,
        meetings,
        item(
            "meetings.notes.writer",
            writer,
            json!("codex"),
            "Конспекты на русском заметно лучше, чем у Apple Intelligence",
        ),
        quick,
    ]
}

/// The answers turn: no bubble, the model's two lines, and the plan.
fn answers(plan: Card) -> Turn {
    let mut answered = turn(
        "use: dictation_any, dictation_code, meetings; language: ru; cloud: chatgpt",
        "Готово. Вот что я предлагаю под русскую диктовку в код и записи созвонов. Снимите галочку с того, что не нужно.",
        &[],
        2,
    );
    answered.answers = asks(true)
        .into_iter()
        .map(|ask| Answer {
            id: ask.id,
            chosen: ask.chosen,
        })
        .collect();
    answered.cards = vec![plan];
    answered
}

/// The Setup board: three answered questions, then six changes, five
/// selected, the permission named before Apply.
pub(in super::super) fn setup_plan(ui: &mut Ui) {
    open(ui);
    ui.platform.system_audio = false;
    let plan = Card {
        kind: CardKind::Plan,
        items: rows(ui),
        ..Card::default()
    };
    conversation(ui, REQUEST, vec![request(asks(true)), answers(plan)], &[]);
}

/// Applied: four of five rows saved, Meetings' writer refused, and GigaAM
/// downloading; Undo while the conversation stays open.
pub(in super::super) fn setup_result(ui: &mut Ui) {
    open(ui);
    ui.platform.system_audio = true;
    let mut items = rows(ui);
    items[4].failed = Some("Codex isn't installed on this Mac".into());
    // What the save changed.
    let settings = &mut ui.settings;
    settings.labs.russian_vibe_coding = true;
    settings.labs.ai_cleanup_server = true;
    settings.dictation_processing.server.runs_on = CleanupRunsOn::Codex;
    settings.meetings.enabled = true;
    let mut before = Map::new();
    let mut after = Map::new();
    for (key, old, new) in [
        ("labs.russian_vibe_coding", json!(false), json!(true)),
        ("labs.ai_cleanup_server", json!(false), json!(true)),
        (
            "dictation_processing.server.runs_on",
            json!("opencode"),
            json!("codex"),
        ),
        ("meetings.enabled", json!(false), json!(true)),
    ] {
        before.insert(key.into(), old);
        after.insert(key.into(), new);
    }
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
    conversation(
        ui,
        REQUEST,
        vec![request(asks(true)), answers(plan)],
        &[(1, 0)],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    type Scene = (&'static str, fn(&mut Ui));

    #[test]
    fn every_setup_scene_has_nothing_left_to_save() {
        let scenes: [Scene; 3] = [
            ("setup-ask", setup_ask),
            ("setup-plan", setup_plan),
            ("setup-result", setup_result),
        ];
        for (name, scene) in scenes {
            let mut ui = super::super::super::scenes::mac_fixture();
            scene(&mut ui);
            assert!(ui.drafted() == ui.settings, "{name} has a draft to save");
        }
    }
}
