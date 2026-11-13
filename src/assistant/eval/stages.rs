//! What the setup scenarios are built from: the installs they play on,
//! each stage as the setup machine (`setup::flow`) asks a model for it
//! when sent the events a person's presses send, and the replies a model
//! is played as writing.

use super::fixtures::{self, owner};
use super::harness::Fixture;
use crate::assistant::facts::State;
use crate::assistant::setup::flow::{self, Effect, Env, Event, Flow};
use crate::assistant::setup::{Ask, Stage, follow, past, since};
use crate::assistant::store::Turn;

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
    let setups = vec![finished(&fixture)];
    // No release is counted as having changed what was answered.
    fixture.state.answered = past::answered_before(&setups, &fixture.settings, |_, _| false);
    fixture.state.declined = past::declined(&setups);
    fixture.setups = setups;
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

/// The setup on a fixture as the machine (`setup::flow`) runs it for a
/// remote model: the events a person would send, and the model's turns
/// as the worker shapes them.
struct Run<'a> {
    fixture: &'a Fixture,
    flow: Flow,
}

impl<'a> Run<'a> {
    /// "Set VibeHex up for me" on `fixture`: a returning setup when it
    /// was set up before or has been used a while.
    fn begin(fixture: &'a Fixture) -> (Self, Vec<Effect>) {
        let mut run = Self {
            fixture,
            flow: Flow::new(1, Vec::new(), fixture.setups.clone()),
        };
        let used = fixtures::used(fixture).unwrap_or_default();
        let since = since::returning(&fixture.setups, used.dictations, used.meetings)
            .then(|| fixtures::since(fixture, &fixture.setups));
        let effects = run.send(Event::Begin { since });
        (run, effects)
    }

    fn send(&mut self, event: Event) -> Vec<Effect> {
        let context = self.fixture.context();
        let env = Env {
            context: &context,
            saved: &self.fixture.settings,
            now_ms: self.fixture.now_ms,
            on_device: false,
            busy: false,
            scene: false,
            // No release is counted as having changed what was answered.
            changed: &|_, _| false,
        };
        let (flow, effects) = std::mem::take(&mut self.flow).on(event, &env);
        self.flow = flow;
        effects
    }

    /// A model's turn for `stage`, asking `asks`, joins as the worker
    /// shapes it.
    fn reply(&mut self, stage: &Stage, asks: Vec<Ask>) {
        let mut turn = Turn {
            asks,
            created_ms: self.fixture.now_ms,
            ..Turn::default()
        };
        flow::replied(stage, &mut turn, true, Some(&self.fixture.context()));
        self.flow.turns.push(turn);
    }

    /// Round one answered as [`fixtures::answers`] and Continue pressed.
    fn round_one(&mut self) -> Vec<Effect> {
        let turn = self.flow.turns.len() - 1;
        for answer in fixtures::answers() {
            let asks = &self.flow.turns[turn].asks;
            let Some(number) = asks.iter().position(|ask| ask.id == answer.id) else {
                continue;
            };
            let ask = &asks[number];
            // A chip pressed for each option that isn't as wanted.
            let press: Vec<String> = ask
                .options
                .iter()
                .map(|option| option.id.clone())
                .filter(|id| {
                    let wanted = answer.chosen.contains(id);
                    wanted != ask.chosen.contains(id) && (ask.multi || wanted)
                })
                .collect();
            for id in press {
                self.send(Event::Pick { turn, number, id });
            }
        }
        self.send(Event::Answered { turn, skip: false })
    }
}

/// What a transition asked a model, and at which stage.
fn asked(effects: Vec<Effect>) -> Option<(String, Stage)> {
    effects.into_iter().find_map(|effect| match effect {
        Effect::Ask { line, stage } => Some((line, stage)),
        _ => None,
    })
}

/// The setup on `fixture` up to round one answered: what the model is
/// asked for the follow-ups.
fn to_follow(fixture: &Fixture) -> (Run<'_>, (String, Stage)) {
    // The answers go to the model in the app's own words.
    crate::desktop::i18n::speaking(Some("en"), || {
        let (mut run, effects) = Run::begin(fixture);
        // A returning setup's look comes back before round one shows.
        if let Some((_, look)) = asked(effects) {
            run.reply(&look, Vec::new());
        }
        let follow = asked(run.round_one()).expect("round one leads to the follow-ups");
        (run, follow)
    })
}

/// Round one answered: the follow-ups a model is asked to word.
pub fn follow_stage(fixture: &Fixture) -> (String, Stage) {
    to_follow(fixture).1
}

/// The terms the fixture's use shows Vocabulary lacks.
pub fn candidates(fixture: &Fixture) -> Vec<since::Candidate> {
    fixtures::used(fixture)
        .as_ref()
        .map(since::candidates)
        .unwrap_or_default()
}

/// The follow-ups skipped ("Skip to the plan"): the plan a model is asked
/// for, from round one's answers.
pub fn plan_stage(fixture: &Fixture) -> (String, Stage) {
    let (mut run, (_, follow)) = to_follow(fixture);
    crate::desktop::i18n::speaking(Some("en"), || {
        let Stage::Follow { candidates, .. } = &follow else {
            unreachable!("round one asks for follow-ups");
        };
        run.reply(&follow, candidates.clone());
        let turn = run.flow.turns.len() - 1;
        asked(run.send(Event::Answered { turn, skip: true })).expect("the answers lead to the plan")
    })
}

/// A returning setup's opening on a remote model: the look.
pub fn look_stage(fixture: &Fixture) -> (String, Stage) {
    let (_, effects) = Run::begin(fixture);
    asked(effects).expect("a returning setup looks first")
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
{\"term\":\"Web\",\"sounds_like\":[\"веб\"]}\n\
```\n\nSources: none";

pub const LOOK: &str = "Вот что из этого стоит поменять:\n\n\
- В **General › Vocabulary** стоит добавить Codex: очистка четыре раза исправляла «кодекс».\n\
- Диктовок в редакторы кода больше половины: проверьте **Labs › Russian vibe coding**.\n\
- GitHub и pull request тоже просятся в словарь.\n\
- Длинных диктовок мало, так что быстрая локальная очистка вам подходит.\n\nSources: none";
