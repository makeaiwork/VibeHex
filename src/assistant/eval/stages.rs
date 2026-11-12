//! What the setup scenarios are built from: the installs they play on,
//! each stage as the pane's own calls make it (`rune_ui::assistant::ask`),
//! and the replies a model is played as writing.

use super::fixtures::{self, owner};
use super::harness::Fixture;
use crate::assistant::facts::State;
use crate::assistant::recordings::Step as Read;
use crate::assistant::setup::{self, Stage, follow, past, since};

/// Twenty days before the fixtures' now.
pub fn earlier() -> u64 {
    fixtures::now() - 20 * 86_400_000
}

/// The setup the returning fixture finished twenty days ago: round one,
/// and two follow-ups answered, the first Yes, the second No.
pub fn finished(fixture: &Fixture) -> past::Setup {
    let answers = fixtures::answers();
    let fresh = State {
        answered: Vec::new(),
        declined: Vec::new(),
        ..fixture.state.clone()
    };
    let mut asks = follow::candidates(&answers, &fixture.settings, &fresh);
    asks.truncate(2);
    for (index, ask) in asks.iter_mut().enumerate() {
        ask.done = true;
        ask.chosen = vec![if index == 0 { "yes" } else { "no" }.into()];
    }
    past::finished(&asks, &answers, None, &fixture.settings, earlier())
}

/// The owner's install, set up twenty days ago.
pub fn returning() -> Fixture {
    let mut fixture = owner();
    let setups = [finished(&fixture)];
    // No release is counted as having changed what was answered.
    fixture.state.answered = past::answered_before(&setups, &fixture.settings, |_, _| false);
    fixture.state.declined = past::declined(&setups);
    fixture
}

/// The owner's install on a speech model they picked themselves, which
/// the rules would change for someone who dictates code in Russian.
pub fn own_model() -> Fixture {
    let mut fixture = owner();
    fixture.settings.transcription.model =
        crate::transcription::models::TranscriptionModelId::GigaAmV3;
    fixture.state.installed =
        crate::assistant::facts::Fact::Known(vec!["gigaam_v3".into(), "parakeet_v3".into()]);
    fixture
}

pub fn follow_stage(fixture: &Fixture) -> (String, Stage) {
    let answers = fixtures::answers();
    let line = fixtures::answers_line(&answers, fixture.state.os);
    let stage = Stage::Follow {
        candidates: follow::candidates(&answers, &fixture.settings, &fixture.state),
        before: follow::before(&answers, &fixture.state),
        since: String::new(),
        line: line.clone(),
        answers,
    };
    (line, stage)
}

/// The terms the fixture's use shows Vocabulary lacks.
pub fn candidates(fixture: &Fixture) -> Vec<since::Candidate> {
    fixtures::used(fixture)
        .as_ref()
        .map(since::candidates)
        .unwrap_or_default()
}

pub fn plan_stage(fixture: &Fixture) -> (String, Stage) {
    let answers = fixtures::answers();
    let line = fixtures::answers_line(&answers, fixture.state.os);
    let stage = Stage::Plan {
        since: String::new(),
        terms: candidates(fixture),
        line: line.clone(),
        answers,
    };
    (line, stage)
}

/// A returning setup's opening on a remote model, as `ask::start` makes it.
pub fn look_stage(fixture: &Fixture) -> (String, Stage) {
    let since = fixtures::since(fixture, &[finished(fixture)]);
    let mut asks = setup::questions(fixture.state.os);
    setup::preselect(&mut asks, &fixture.settings, &fixture.state);
    since::preselect(&mut asks, &since);
    let usage = since.usage.clone().unwrap_or_default();
    let stage = Stage::Look {
        facts: since::prompt(&since),
        shown: since::paragraph(&since),
        asks,
        terms: since::candidates(&usage),
        step: Read::Looked {
            dictations: usage.dictations,
            meetings: usage.meetings,
        },
    };
    (String::new(), stage)
}

pub const ROUND_ONE: &str = "Давайте настроим VibeHex под вас.\n\n```hex-ask\n\
{\"id\":\"use\",\"q\":\"Для чего вам VibeHex?\",\"hint\":\"можно несколько\",\"multi\":true,\"options\":[{\"id\":\"dictation_any\",\"label\":\"Диктовка в любые приложения\"},{\"id\":\"dictation_code\",\"label\":\"Диктовка в код и терминал\"},{\"id\":\"meetings\",\"label\":\"Запись созвонов\"},{\"id\":\"files\",\"label\":\"Расшифровка файлов\"}]}\n\
{\"id\":\"language\",\"q\":\"На каком языке вы обычно говорите?\",\"options\":[{\"id\":\"ru\",\"label\":\"Русский\"},{\"id\":\"en\",\"label\":\"English\"},{\"id\":\"ru_tech\",\"label\":\"Русский с английскими терминами\"},{\"id\":\"mixed\",\"label\":\"Другой или вперемешку\"}]}\n\
{\"id\":\"cleanup\",\"q\":\"Где чистить диктовку?\",\"options\":[{\"id\":\"local\",\"label\":\"На этом Mac\"},{\"id\":\"codex\",\"label\":\"Codex\"},{\"id\":\"opencode\",\"label\":\"OpenCode\"},{\"id\":\"openrouter\",\"label\":\"OpenRouter\"},{\"id\":\"custom\",\"label\":\"Свой сервер\"},{\"id\":\"none\",\"label\":\"Без чистки\"}]}\n\
{\"id\":\"notes\",\"q\":\"Где писать конспекты и обрабатывать файлы?\",\"options\":[{\"id\":\"codex\",\"label\":\"Codex\"},{\"id\":\"opencode\",\"label\":\"OpenCode\"},{\"id\":\"openrouter\",\"label\":\"OpenRouter\"},{\"id\":\"custom\",\"label\":\"Свой сервер\"},{\"id\":\"local\",\"label\":\"На этом Mac\"},{\"id\":\"apple\",\"label\":\"Apple Intelligence\"}]}\n\
```\n\nSources: none";

/// Follow-ups a model is played as writing: the two this Mac mini can't
/// use, and one that fits (a feature every OS has, so the scenarios hold
/// wherever the checks run).
pub const FOLLOW_UNFIT: &str = "Ещё пара вопросов.\n\n```hex-ask\n\
{\"id\":\"labs.notch_indicator\",\"q\":\"Показывать индикатор у выреза экрана?\",\"options\":[{\"id\":\"yes\",\"label\":\"Да, включить\"},{\"id\":\"no\",\"label\":\"Нет\"}]}\n\
{\"id\":\"labs.mouse_shortcuts\",\"q\":\"Диктовать по боковой кнопке мыши?\",\"options\":[{\"id\":\"yes\",\"label\":\"Да, включить\"},{\"id\":\"no\",\"label\":\"Нет\"}]}\n\
{\"id\":\"labs.filler_words\",\"q\":\"Убирать «эээ» и «ну» из диктовки?\",\"options\":[{\"id\":\"yes\",\"label\":\"Да, включить\"},{\"id\":\"no\",\"label\":\"Нет\"}]}\n\
```\n\nSources: none";

pub const PLAN_UNFIT: &str = "Вот что подходит к тому, как вы работаете.\n\n```hex-plan\n\
{\"key\":\"labs.notch_indicator\",\"value\":true,\"why\":\"индикатор у выреза\"}\n\
{\"key\":\"labs.mouse_shortcuts\",\"value\":true,\"why\":\"диктовка с мыши\"}\n\
{\"key\":\"labs.filler_words\",\"value\":true,\"why\":\"без «эээ»\"}\n\
```\n\nSources: none";

/// A returning plan a model is played as writing: a setting, two terms on
/// one line each, and an ordinary word as a term.
pub const PLAN_TERMS: &str = "Вот что стоит поменять.\n\n```hex-plan\n\
{\"key\":\"labs.filler_words\",\"value\":true,\"why\":\"без «эээ»\"}\n\
{\"term\":\"Codex\",\"sounds_like\":[\"кодекс\"]}\n\
{\"term\":\"GitHub\",\"sounds_like\":[\"гитхаб\"],\"optional\":true}\n\
```\n\nSources: none";

pub const LOOK: &str = "- В **General › Vocabulary** стоит добавить Codex: очистка четыре раза исправляла «кодекс».\n\
- Диктовок в редакторы кода больше половины: проверьте **Labs › Russian vibe coding**.\n\
- GitHub и pull request тоже просятся в словарь.\n\
- Длинных диктовок мало, так что быстрая локальная очистка вам подходит.\n\nSources: none";
