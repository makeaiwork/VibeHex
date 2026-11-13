//! Round one's language question and the plan it makes, for a person whose
//! language is neither Russian nor English, and for Other or mixed.

use serde_json::json;

use super::*;
use crate::assistant::changes::{Card, Item};
use crate::assistant::facts::{Fact, State};
use crate::assistant::setup::{Answer, plan, preselect, questions, questions_for};
use crate::transcription::models::{self, TranscriptionModelId as Model};

fn mac(system: &str) -> State {
    State {
        os: "macos",
        codex_signed_in: Fact::Known(true),
        opencode_running: Fact::Known(true),
        openrouter_key: Fact::Known(true),
        system_language: system.into(),
        ..State::default()
    }
}

fn on(model: Model, language: &str) -> AppSettings {
    let mut settings = AppSettings::default();
    settings.transcription.model = model;
    settings.transcription.language = language.into();
    settings
}

fn ids(settings: &AppSettings, state: &State) -> Vec<String> {
    let asks = questions_for("macos", Spoken::of(settings, state));
    asks[1]
        .options
        .iter()
        .map(|option| option.id.clone())
        .collect()
}

fn answers(language: &str) -> Vec<Answer> {
    [("use", "dictation_code"), ("language", language)]
        .map(|(id, chosen)| Answer {
            id: id.into(),
            chosen: vec![chosen.into()],
            changes: Vec::new(),
        })
        .to_vec()
}

fn plan_for(settings: &AppSettings, language: &str) -> Option<Card> {
    plan::plan(Vec::new(), &answers(language), settings, &mac(""))
}

fn row<'a>(card: &'a Card, key: &str) -> Option<&'a Item> {
    card.items.iter().find(|item| item.key == key)
}

fn kept(card: &Card, key: &str) -> bool {
    card.kept.iter().any(|item| item.key == key)
}

#[test]
fn their_language_is_the_recognition_then_the_system_then_the_window() {
    let own = |recognition, system, window| Spoken::from(recognition, system, window).own;
    assert_eq!(own(Some("es"), "en", "en"), Some("es"));
    assert_eq!(own(None, "es-MX", "en"), Some("es"));
    assert_eq!(own(None, "", "de"), Some("de"));
    assert_eq!(own(None, "", "zh-Hant"), Some("zh"));
    assert_eq!(own(None, "nb", "en"), Some("no"));
    // Russian or English: the usual question.
    assert_eq!(own(Some("ru"), "es", "es"), None);
    assert_eq!(own(Some("en"), "es", "es"), None);
    assert_eq!(own(None, "en", "es"), None);
    // A language VibeHex doesn't name: the usual question too.
    assert_eq!(own(Some("ca"), "es", "es"), None);
    assert_eq!(own(None, "", ""), None);
}

#[test]
fn russian_shows_where_any_of_the_three_is_russian() {
    assert!(Spoken::from(Some("es"), "en", "ru").russian);
    assert!(Spoken::from(Some("es"), "ru-RU", "en").russian);
    assert!(Spoken::from(Some("ru"), "en", "en").russian);
    assert!(!Spoken::from(Some("es"), "en", "en").russian);
}

#[test]
fn an_option_id_names_its_language() {
    assert_eq!(id("es"), "lang:es");
    assert_eq!(code("lang:es"), Some("es"));
    assert_eq!(code("mixed"), None);
    assert_eq!(code("lang:auto"), None);
    assert_eq!(name("es"), "Spanish");
}

#[test]
fn a_name_inside_a_sentence_is_lower_case_where_the_language_writes_it_so() {
    let in_ = |code: &str, language| i18n::speaking(Some(code), || in_sentence(language));
    assert_eq!(in_("ru", "es"), "испанский");
    assert_eq!(in_("es", "es"), "español");
    assert_eq!(in_("de", "es"), "Spanisch");
    assert_eq!(in_("uk", "es"), "Іспанська");
    assert_eq!(in_("en", "he"), "Hebrew");
}

/// Spanish as the recognition language, or the system's under Auto: the
/// question starts with Spanish, then English and Other or mixed, with no
/// Russian; Russian beside it when the window is Russian.
#[test]
fn their_own_language_leads_the_question() {
    crate::desktop::i18n::apply(Some("en"));
    let spanish = on(Model::ParakeetV3, "es");
    assert_eq!(ids(&spanish, &mac("")), ["lang:es", "en", "mixed"]);
    let auto = on(Model::ParakeetV3, "auto");
    assert_eq!(ids(&auto, &mac("es-ES")), ["lang:es", "en", "mixed"]);
    // Auto with no system language: the window's.
    crate::desktop::i18n::apply(Some("de"));
    assert_eq!(ids(&auto, &mac("")), ["lang:de", "en", "mixed"]);
    crate::desktop::i18n::apply(Some("ru"));
    assert_eq!(
        ids(&spanish, &mac("")),
        ["lang:es", "en", "ru", "ru_tech", "mixed"]
    );
    crate::desktop::i18n::apply(Some("en"));
    // Its name in Assistant's language, a capital on the chip.
    let asks = questions_for("macos", Spoken::of(&spanish, &mac("")));
    let label = |code| i18n::speaking(Some(code), || asks[1].label(&asks[1].options[0]));
    assert_eq!(label("en"), "Spanish");
    assert_eq!(label("ru"), "Испанский");
    assert_eq!(label("es"), "Español");
}

/// Russian or English: the question is the one it always was, whatever the
/// system and the window say.
#[test]
fn russian_and_english_keep_the_usual_question() {
    let usual: Vec<String> = questions("macos")[1]
        .options
        .iter()
        .map(|option| option.id.clone())
        .collect();
    assert_eq!(usual, ["ru", "en", "ru_tech", "mixed"]);
    crate::desktop::i18n::apply(Some("es"));
    let mut owners = AppSettings::default();
    crate::assistant::setup::sample(&mut owners);
    assert_eq!(ids(&owners, &mac("es")), usual);
    assert_eq!(ids(&on(Model::GigaAmV3, "auto"), &mac("es")), usual);
    assert_eq!(ids(&AppSettings::default(), &mac("es")), usual);
    assert_eq!(ids(&on(Model::ParakeetV3, "auto"), &mac("en-US")), usual);
    crate::desktop::i18n::apply(Some("en"));
    // The same questions, word for word.
    assert_eq!(
        questions_for("macos", Spoken::of(&owners, &mac("es"))),
        questions("macos")
    );
}

/// Their language preselects its option; Auto preselects Other or mixed,
/// and so does their language where the question doesn't offer it.
#[test]
fn their_language_is_preselected() {
    let chosen = |settings: &AppSettings, state: &State, spoken: Spoken| {
        let mut asks = questions_for("macos", spoken);
        preselect(&mut asks, settings, state);
        asks[1].chosen.clone()
    };
    let spanish = on(Model::ParakeetV3, "es");
    let state = mac("");
    assert_eq!(
        chosen(&spanish, &state, Spoken::of(&spanish, &state)),
        ["lang:es"]
    );
    assert_eq!(chosen(&spanish, &state, Spoken::default()), ["mixed"]);
    let auto = on(Model::ParakeetV3, "auto");
    let state = mac("es");
    assert_eq!(chosen(&auto, &state, Spoken::of(&auto, &state)), ["mixed"]);
    // Arabic's own model names its language.
    let arabic = on(Model::QwenCleoAsr17B, "ar");
    assert_eq!(
        chosen(&arabic, &state, Spoken::of(&arabic, &state)),
        ["lang:ar"]
    );
}

/// Spanish on the English model: Parakeet v3, which hears it, checked
/// since Spanish needs it, then the language. A model that hears Spanish
/// stays; Spanish already set stays too. No Russian vibe coding.
#[test]
fn their_language_sets_the_language_and_a_model_that_hears_it() {
    let card = plan_for(&AppSettings::default(), "lang:es").unwrap();
    let model = row(&card, "transcription.model").unwrap();
    assert_eq!(model.to, json!("parakeet_v3"));
    assert!(model.included && !model.yours);
    assert_eq!(model.why, "Hears Spanish well");
    let language = row(&card, "transcription.language").unwrap();
    assert_eq!((&language.from, &language.to), (&json!("en"), &json!("es")));
    assert!(language.included);
    let at = |key| card.items.iter().position(|item| item.key == key);
    assert!(at("transcription.model") < at("transcription.language"));
    assert!(row(&card, "labs.russian_vibe_coding").is_none());
    // In Russian, the name reads inside the sentence.
    let russian = i18n::speaking(Some("ru"), || plan_for(&AppSettings::default(), "lang:es"));
    let why = &row(russian.as_ref().unwrap(), "transcription.model")
        .unwrap()
        .why;
    assert_eq!(why, "Хорошо понимает испанский");

    let card = plan_for(&on(Model::WhisperLargeV3Turbo, "en"), "lang:es").unwrap();
    assert!(row(&card, "transcription.model").is_none());
    assert!(kept(&card, "transcription.model"));
    assert_eq!(
        row(&card, "transcription.language").unwrap().to,
        json!("es")
    );
    let card = plan_for(&on(Model::ParakeetV3, "es"), "lang:es").unwrap();
    assert!(row(&card, "transcription.model").is_none());
    assert!(row(&card, "transcription.language").is_none());
    assert!(kept(&card, "transcription.model") && kept(&card, "transcription.language"));

    // Arabic, on a model that doesn't hear it: the one VibeHex recommends.
    let card = plan_for(&AppSettings::default(), "lang:ar").unwrap();
    let model = row(&card, "transcription.model").unwrap();
    let suggested: Model = model.to.as_str().unwrap().parse().unwrap();
    assert!(models::definition(suggested).supports_language("ar"));
    assert_eq!(
        row(&card, "transcription.language").unwrap().to,
        json!("ar")
    );
}

/// Their language asks what English asks, never the Russian follow-ups.
#[test]
fn their_language_asks_no_russian_follow_ups() {
    let settings = AppSettings::default();
    let asked = |language: &str| -> Vec<String> {
        crate::assistant::setup::follow::candidates(&answers(language), &settings, &mac(""))
            .into_iter()
            .map(|ask| ask.id)
            .collect()
    };
    assert_eq!(asked("lang:es"), asked("en"));
}

/// Other or mixed on a model of one language brings a model that finds
/// the language itself and still hears the one spoken now: never Parakeet
/// v3 for Arabic, Hebrew, or Norwegian. Recognition goes to Auto.
#[test]
fn other_or_mixed_never_brings_a_model_that_doesnt_hear_their_language() {
    for (model, language) in [
        (Model::QwenCleoAsr17B, "ar"),
        (Model::IvritWhisperLargeV3Turbo, "he"),
        (Model::NbWhisperMedium, "no"),
    ] {
        let card = plan_for(&on(model, language), "mixed").unwrap();
        let row = row(&card, "transcription.model").unwrap();
        assert_ne!(row.to, json!("parakeet_v3"), "{model:?}");
        let suggested: Model = row.to.as_str().unwrap().parse().unwrap();
        let definition = models::definition(suggested);
        assert!(definition.supports_language(language), "{model:?}");
        assert!(definition.supports_language("auto"), "{model:?}");
        assert_eq!(
            card.items
                .iter()
                .find(|item| item.key == "transcription.language")
                .map(|item| item.to.clone()),
            Some(json!("auto")),
            "{model:?}"
        );
    }
    // A model that finds the language itself stays: SenseVoice's Chinese.
    let card = plan_for(&on(Model::SenseVoiceSmall, "zh"), "mixed").unwrap();
    assert!(row(&card, "transcription.model").is_none());
    assert_eq!(
        row(&card, "transcription.language").unwrap().to,
        json!("auto")
    );
    // GigaAM's Russian: Parakeet v3, as before; Auto already set stays.
    let card = plan_for(&on(Model::GigaAmV3, "ru"), "mixed").unwrap();
    assert_eq!(
        row(&card, "transcription.model").unwrap().to,
        json!("parakeet_v3")
    );
    let card = plan_for(&on(Model::ParakeetV3, "auto"), "mixed").unwrap();
    assert!(row(&card, "transcription.model").is_none());
    assert!(kept(&card, "transcription.language"));
}

/// Over every catalog model and every language it hears, Other or mixed
/// suggests only a model that hears that language and finds it itself.
#[test]
fn other_or_mixed_keeps_every_language_heard() {
    for model in models::MODELS.iter().filter(|model| model.available()) {
        for language in model.language_choices() {
            if language == "auto" {
                continue;
            }
            let settings = on(model.id, language);
            let Some(card) = plan_for(&settings, "mixed") else {
                continue;
            };
            if let Some(row) = row(&card, "transcription.model") {
                let suggested: Model = row.to.as_str().unwrap().parse().unwrap();
                let definition = models::definition(suggested);
                assert!(
                    definition.supports_language(language) && definition.supports_language("auto"),
                    "{:?} in {language}: {suggested:?}",
                    model.id
                );
            }
        }
    }
}
