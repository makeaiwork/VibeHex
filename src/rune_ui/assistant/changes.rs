//! What a card's buttons do. A change saves only through
//! `control::settings::check`, the path `hex settings set` takes, then the
//! window's own save (`Ui::update`), so the host adopts it as it would a
//! switch flipped by hand. A speech model goes through the service's
//! ChooseModel, which downloads it first. Undo puts back what the save
//! changed while the conversation stays open this session (owner decision
//! 2); a switch to another conversation, or a quit, ends it.

use rune::Tx;
use serde_json::{Map, Value};

use super::super::sidebar::Pane;
use super::super::{Ui, history, labs, picker, platform};
use crate::assistant::changes::catalog::Class;
use crate::assistant::changes::names::{self, Home};
use crate::assistant::changes::state::{self, Codex, Live, Shown};
use crate::assistant::changes::{self, Card, CardKind, Context, Decision, vocab};
use crate::assistant::facts;
use crate::control::settings::{self, Effect};
use crate::desktop::i18n::tr;
use crate::transcription::models::{self as transcription_models, TranscriptionModelId};

/// Where a card is: its conversation, turn, and place among the turn's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) struct At {
    pub(super) conversation: u32,
    pub(super) turn: usize,
    pub(super) card: usize,
}

/// What a question's suggestions are offered and checked against: the
/// settings now, with what is being typed, and what `facts::gather` reads
/// off this computer. The pane passes only what lives in its own state:
/// the setups its store keeps, what Output and the pane last heard of
/// Codex and OpenCode, and a scene's stand-ins.
pub(super) fn context(ui: &Ui) -> Context {
    let settings = ui.drafted();
    let state = facts::gather(facts::Seen {
        settings: &settings,
        setups: ui.assistant.store.setups(),
        codex_signed_in: super::super::output::codex_signed_in(ui),
        opencode_running: ui.assistant.reach.opencode,
        scene: scene(ui),
    });
    Context { settings, state }
}

/// A scene's stand-ins for this computer; `None` in the real window.
fn scene(ui: &Ui) -> Option<facts::Scene> {
    ui.assistant.fixture.then(|| facts::Scene {
        installed: ui
            .picker
            .installed
            .as_ref()
            .map(|installed| installed.iter().map(|id| id.as_str().to_owned()).collect()),
        hardware: ui.assistant.hardware.clone(),
    })
}

/// The card at `at`.
pub(super) fn card(ui: &Ui, at: At) -> Option<&Card> {
    ui.assistant
        .store
        .get(at.conversation)?
        .turns
        .get(at.turn)?
        .cards
        .get(at.card)
}

/// What is true now for the card at `at`.
pub(super) fn live(ui: &Ui, at: At) -> Live<'_> {
    let transcription = ui.view.as_ref().map(|view| &view.transcription);
    let preparing = transcription.and_then(|transcription| {
        let model = transcription.preparing?;
        let total = transcription_models::definition(model)
            .download_bytes()
            .unwrap_or(0);
        Some((
            model.as_str().to_owned(),
            transcription.downloaded_bytes,
            total,
        ))
    });
    Live {
        settings: &ui.settings,
        preparing,
        codex: codex(ui),
        // Only macOS asks; a scene plays a Mac.
        system_audio: ui.platform.system_audio
            || (!cfg!(target_os = "macos") && !ui.assistant.fixture),
        undoable: ui.assistant.undo.contains(&at) && ui.assistant.open == Some(at.conversation),
        here: !ui.assistant.fixture,
    }
}

/// Where Codex stands for a card running on it: the CLI found (by the
/// pane's look or Output's), then its sign-in. A look that failed for
/// another reason doesn't hold a card back. A scene plays what it sets.
pub(super) fn codex(ui: &Ui) -> Codex {
    if ui.assistant.fixture {
        return ui.assistant.codex_as;
    }
    use super::super::output;
    if ui.assistant.reach.codex == Some(false) || output::codex_missing(ui) {
        return Codex::Missing;
    }
    match output::codex_signed_in(ui) {
        Some(true) => Codex::Ready,
        Some(false) => Codex::SignedOut,
        None if output::codex_failed(ui) => Codex::Ready,
        None => Codex::Checking,
    }
}

/// `card` as a save would take it: a Codex row shows unchecked while
/// Codex isn't ready, and is counted and saved so.
pub(super) fn as_saved(ui: &Ui, card: &Card) -> Card {
    let codex = codex(ui);
    let mut card = card.clone();
    for item in &mut card.items {
        if state::codex_need(&item.key, &item.to, codex).is_some() {
            item.included = false;
        }
    }
    card
}

/// What the card at `at` shows now.
pub(super) fn shown(ui: &Ui, at: At) -> Option<Shown> {
    let card = card(ui, at)?;
    if ui.assistant.fixture
        && let Some((_, shown)) = ui.assistant.shown_as.iter().find(|(card, _)| *card == at)
    {
        return Some(shown.clone());
    }
    Some(state::shown(card, &live(ui, at)))
}

/// Whether a card still waits on the person: its main button may be the
/// screen's Primary.
pub(super) fn undecided(shown: &Shown) -> bool {
    matches!(
        shown,
        Shown::Proposed { .. }
            | Shown::SignIn
            | Shown::InstallCodex
            | Shown::CheckingCodex
            | Shown::Permission(_)
    )
}

/// The newest card in conversation `id` still waiting on the person: only
/// its main button is Primary (owner decision 3).
pub(super) fn newest(ui: &Ui, id: u32) -> Option<At> {
    let conversation = ui.assistant.store.get(id)?;
    conversation
        .turns
        .iter()
        .enumerate()
        .flat_map(|(turn, cards)| {
            (0..cards.cards.len()).map(move |card| At {
                conversation: id,
                turn,
                card,
            })
        })
        .filter(|at| shown(ui, *at).as_ref().is_some_and(undecided))
        .last()
}

pub(super) fn set(ui: &mut Ui, at: At, change: impl FnOnce(&mut Card)) {
    let Some(mut card) = card(ui, at).cloned() else {
        return;
    };
    change(&mut card);
    ui.assistant
        .store
        .set_card(at.conversation, at.turn, at.card, card);
}

/// A speech model, which the service downloads and then switches to.
fn is_download(item: &changes::Item) -> bool {
    item.class == Class::Download && item.key.starts_with("transcription.")
}

/// Saves the card's change: what it needs first and the change itself, in
/// one save, or nothing.
pub(super) fn apply(ui: &mut Ui, at: At) {
    let _speaking = crate::assistant::language::speak(&ui.settings);
    // A term still being edited is kept first.
    super::vocab::done(ui);
    let Some(card) = card(ui, at).cloned() else {
        return;
    };
    if card.kind == CardKind::Plan {
        return apply_plan(ui, at, card);
    }
    if card.kind == CardKind::Vocabulary {
        return super::vocab::apply(ui, at, card);
    }
    if card.kind == CardKind::VocabularyEdit {
        return super::vocab_edit::apply(ui, at, card);
    }
    let decision = match card.items.first() {
        Some(item) if is_download(item) => {
            let to = chosen(&card).cloned().unwrap_or(Value::Null);
            prepare(ui, &item.key, &to).map(|()| Decision::Applied {
                before: Map::from_iter([(item.key.clone(), item.from.clone())]),
                after: Map::from_iter([(item.key.clone(), to)]),
                at_ms: ui.assistant.now_ms(),
            })
        }
        Some(_) => changes::values(&card, &ui.drafted()).and_then(|values| save(ui, &values)),
        None => return,
    };
    let applied = decision.is_ok();
    set(ui, at, |card| {
        card.decision = decision.unwrap_or_else(|why| Decision::Failed { why });
        card.later = false;
    });
    if applied && !ui.assistant.undo.contains(&at) {
        ui.assistant.undo.push(at);
    }
}

/// Saves a plan's checked rows. Each is checked on its own against what
/// the rows before it make, so a failing one is left out with why and the
/// rest go in one save; the speech model downloads after it, and a failed
/// download takes nothing back.
fn apply_plan(ui: &mut Ui, at: At, card: Card) {
    let mut card = as_saved(ui, &card);
    let before = ui.drafted();
    let rules = super::super::shortcut_rules();
    let mut candidate = before.clone();
    let mut effects = Vec::new();
    let mut terms = Vec::new();
    for item in &mut card.items {
        item.failed = None;
        if !item.included || is_download(item) {
            continue;
        }
        if item.key == vocab::KEY {
            terms.extend(super::vocab::add(&mut candidate.vocabulary, item));
            continue;
        }
        let one = Card {
            items: vec![item.clone()],
            ..Card::default()
        };
        let checked = changes::values(&one, &candidate).and_then(|values| {
            settings::check(&candidate, &values, &rules).map_err(|error| error.message)
        });
        match checked {
            Ok(change) => {
                candidate = change.settings;
                effects.extend(change.effects);
            }
            Err(why) => item.failed = Some(why),
        }
    }
    // A row kept as it is, its companion picked anew: a change of its own.
    for item in &mut card.kept {
        item.failed = None;
        let Some(companion) = item.companion_changed() else {
            continue;
        };
        let values = Map::from_iter([(companion.key.clone(), companion.to().clone())]);
        match settings::check(&candidate, &values, &rules) {
            Ok(change) => {
                candidate = change.settings;
                effects.extend(change.effects);
            }
            Err(error) => item.failed = Some(error.message),
        }
    }
    let mut decision = commit(ui, &before, candidate, effects);
    if decision.is_ok()
        && let Err((rows, why)) = choose_speech(ui, &card.items, false)
    {
        for index in rows {
            card.items[index].failed = Some(why.clone());
        }
    }
    if let Ok(Decision::Applied { after, .. }) = &mut decision
        && !terms.is_empty()
    {
        after.extend(super::vocab::added_map(terms));
    }
    let (done, _) = state::applied_count(&card);
    if done == 0 && decision.is_ok() {
        let why = card.items.iter().find_map(|item| item.failed.clone());
        decision = Err(why.unwrap_or_else(|| tr("Nothing was selected").to_owned()));
    }
    decide(ui, at, card, decision);
}

/// Keeps what a save of `card` made of it, and offers Undo for it.
pub(super) fn decide(ui: &mut Ui, at: At, mut card: Card, decision: Result<Decision, String>) {
    let applied = decision.is_ok();
    card.decision = decision.unwrap_or_else(|why| Decision::Failed { why });
    ui.assistant
        .store
        .set_card(at.conversation, at.turn, at.card, card);
    if applied && !ui.assistant.undo.contains(&at) {
        ui.assistant.undo.push(at);
    }
    super::ask::decided(ui, at);
}

/// Checks or unchecks row `index` of a plan.
pub(super) fn toggle(ui: &mut Ui, at: At, index: usize) {
    // A Codex row waits for Codex: installed and signed in first.
    let codex = codex(ui);
    if card(ui, at)
        .and_then(|card| card.items.get(index))
        .is_some_and(|item| state::codex_need(&item.key, &item.to, codex).is_some())
    {
        return;
    }
    set(ui, at, |card| {
        if let Some(item) = card.items.get_mut(index) {
            item.included = !item.included;
        }
    });
}

/// The value a card's first item goes to: a choice's chosen option.
fn chosen(card: &Card) -> Option<&Value> {
    let item = card.items.first()?;
    Some(match &card.kind {
        CardKind::Choice {
            options, chosen, ..
        } => options.get(*chosen).unwrap_or(&item.to),
        _ => &item.to,
    })
}

/// A plan's speech model and recognition language rows, checked, as one
/// choice for the service: the language goes with the model checked beside
/// it, never with the one saved now, which a second request would put back.
/// `back` asks for what they were before (Undo), of the rows that went in.
/// A failure names the rows it leaves out.
fn choose_speech(
    ui: &mut Ui,
    items: &[changes::Item],
    back: bool,
) -> Result<(), (Vec<usize>, String)> {
    let rows: Vec<usize> = (0..items.len())
        .filter(|&index| {
            let item = &items[index];
            item.included && is_download(item) && (!back || item.failed.is_none())
        })
        .collect();
    let value = |index: usize| match back {
        true => items[index].from.clone(),
        false => items[index].to.clone(),
    };
    let find = |key: &str| rows.iter().copied().find(|&index| items[index].key == key);
    let language = find("transcription.language").map(value);
    let done = match (find("transcription.model"), &language) {
        (Some(model), language) => {
            let language = language.as_ref().and_then(Value::as_str);
            prepare_model(ui, &value(model), language)
        }
        (None, Some(language)) => prepare(ui, "transcription.language", language),
        (None, None) => return Ok(()),
    };
    done.map_err(|why| (rows, why))
}

/// Asks the service for speech model `to`, in `language` when one is given.
fn prepare_model(ui: &mut Ui, to: &Value, language: Option<&str>) -> Result<(), String> {
    let model: TranscriptionModelId =
        serde_json::from_value(to.clone()).map_err(|error| error.to_string())?;
    picker::choose_in(ui, transcription_models::definition(model), language);
    Ok(())
}

/// Asks the service for a speech model, or the saved model in a language:
/// it downloads what it needs and saves the choice when ready.
fn prepare(ui: &mut Ui, key: &str, to: &Value) -> Result<(), String> {
    match key {
        "transcription.model" => prepare_model(ui, to, None)?,
        "transcription.language" => {
            let language = to.as_str().ok_or("a language is text")?;
            picker::choose_language(ui, language);
        }
        _ => return Err(format!("{key} isn't a speech model")),
    }
    Ok(())
}

/// Checks `values` against the settings with every text field's typing,
/// then saves them as the window saves a switch, with their side effects.
/// The decision names every key the save changed.
fn save(ui: &mut Ui, values: &Map<String, Value>) -> Result<Decision, String> {
    let before = ui.drafted();
    let change = settings::check(&before, values, &super::super::shortcut_rules())
        .map_err(|error| error.message)?;
    commit(ui, &before, change.settings, change.effects)
}

/// Saves `after`, checked already, as the window saves a switch, and
/// carries out `effects`.
fn commit(
    ui: &mut Ui,
    before: &crate::app_settings::AppSettings,
    after: crate::app_settings::AppSettings,
    effects: Vec<Effect>,
) -> Result<Decision, String> {
    let saved = after.clone();
    if !ui.update(|settings| *settings = saved) {
        return Err(ui.error.clone().unwrap_or_default());
    }
    for effect in effects {
        match effect {
            Effect::HistoryRetention(retention) => history::adopt_retention(ui, retention),
            // The host shows or hides the Dock icon as it saves; a card
            // never changes the update channel.
            Effect::DockIcon | Effect::CheckForUpdates => {}
        }
    }
    Ok(changes::applied(
        settings::diff(before, &after),
        ui.assistant.now_ms(),
    ))
}

/// Edit first: the instructions card's text opens in the card itself,
/// never in Output's editor.
pub(super) fn edit(ui: &mut Ui, at: At) {
    let Some(text) = card(ui, at)
        .and_then(|card| card.items.first())
        .and_then(|item| item.to.as_str())
        .map(str::to_owned)
    else {
        return;
    };
    ui.assistant.edit = rune::TextEdit::new(text);
    ui.assistant.editing = Some(at);
}

/// Cancel edit: back to the lines the model wrote.
pub(super) fn cancel_edit(ui: &mut Ui) {
    ui.assistant.editing = None;
}

/// Saves the edited text through the same check as any card, so Undo and
/// a later hand change behave as for the model's own text.
pub(super) fn save_edit(ui: &mut Ui, at: At) {
    if ui.assistant.editing != Some(at) {
        return apply(ui, at);
    }
    let text = ui.assistant.edit.text.trim().to_owned();
    set(ui, at, |card| {
        if let Some(item) = card.items.first_mut() {
            item.to = Value::String(text);
        }
    });
    ui.assistant.editing = None;
    apply(ui, at);
}

/// Not now: nothing changes.
pub(super) fn decline(ui: &mut Ui, at: At) {
    if ui.assistant.editing == Some(at) {
        ui.assistant.editing = None;
    }
    set(ui, at, |card| card.decision = Decision::Declined);
    super::ask::decided(ui, at);
}

/// Puts back what the card's save changed, unless something changed it
/// since.
pub(super) fn undo(ui: &mut Ui, at: At) {
    let Some(card) = card(ui, at).cloned() else {
        return;
    };
    if state::shown(&card, &live(ui, at)) != (Shown::Applied { undo: true }) {
        return;
    }
    let Decision::Applied { before, after, .. } = &card.decision else {
        return;
    };
    let done = match card.items.first() {
        Some(item) if card.kind != CardKind::Plan && is_download(item) => {
            prepare(ui, &item.key, &item.from)
        }
        _ if card.kind == CardKind::Vocabulary => super::vocab::take_back(ui, after),
        _ if card.kind == CardKind::VocabularyEdit => super::vocab_edit::take_back(ui, after),
        _ if card.kind == CardKind::Plan => {
            let saved = if before.is_empty() {
                Ok(())
            } else {
                save(ui, before).map(|_| ())
            }
            .and_then(|()| super::vocab::take_back(ui, after));
            // The speech model it downloaded goes back to the one before.
            let _ = choose_speech(ui, &card.items, true);
            saved
        }
        _ => save(ui, before).map(|_| ()),
    };
    ui.assistant.undo.retain(|undoable| *undoable != at);
    set(ui, at, |card| {
        card.decision = match done {
            Ok(()) => Decision::Undone,
            Err(why) => Decision::Failed { why },
        };
    });
    // A setup's record follows its plan: undone, it no longer says applied.
    super::ask::decided(ui, at);
}

/// Picks option `index` of the companion of row `row` (`kept`: a row of
/// "Already set").
pub(super) fn pick_companion(ui: &mut Ui, at: At, row: usize, kept: bool, index: usize) {
    set(ui, at, |card| {
        let rows = if kept {
            &mut card.kept
        } else {
            &mut card.items
        };
        if let Some(companion) = rows.get_mut(row).and_then(|item| item.companion.as_mut())
            && index < companion.options.len()
        {
            companion.chosen = index;
        }
    });
}

/// Picks a choice's option.
pub(super) fn pick(ui: &mut Ui, at: At, index: usize) {
    set(ui, at, |card| {
        if let CardKind::Choice { chosen, .. } = &mut card.kind {
            *chosen = index;
        }
    });
}

/// Later: the permission waits; the card shows as applied.
pub(super) fn later(ui: &mut Ui, at: At) {
    set(ui, at, |card| card.later = true);
}

/// Asks macOS for Screen & System Audio Recording.
pub(super) fn ask_macos(ui: &mut Ui) {
    ui.ask(platform::PlatformRequest::SystemAudio);
}

/// Stops the speech model downloading.
pub(super) fn cancel_download(ui: &mut Ui) {
    picker::cancel(ui);
}

/// Opens where the card's setting lives in the window: its Labs row, or
/// its pane.
pub(super) fn show(ui: &mut Ui, key: &str, tx: &mut Tx) {
    let Some(place) = names::place(key, &ui.settings) else {
        return;
    };
    let pane = match place.home {
        Home::Labs(lab) => {
            let link = labs::LabsLink {
                lab: Some(lab.to_owned()),
                ..labs::LabsLink::default()
            };
            labs::reveal(ui, &link, tx);
            return;
        }
        Home::General => Pane::General,
        Home::Output => Pane::Output,
        Home::History => Pane::History,
        Home::Meetings => Pane::Meetings,
        Home::Files => Pane::Files,
        #[cfg(hex_capture)]
        Home::Annotate => Pane::Annotate,
        #[cfg(not(hex_capture))]
        Home::Annotate => return,
    };
    ui.select_pane(pane, tx);
}

/// Opens Output, where Codex's sign-in lives.
pub(super) fn open_output(ui: &mut Ui, tx: &mut Tx) {
    ui.select_pane(Pane::Output, tx);
}

/// Starts Codex's sign-in, leaving Assistant's own model as it is.
pub(super) fn sign_in(ui: &mut Ui) {
    super::connect::start_sign_in(ui);
}

/// Asks the card's question again, in the same conversation.
pub(super) fn ask_again(ui: &mut Ui, at: At) {
    let question = ui
        .assistant
        .store
        .get(at.conversation)
        .and_then(|conversation| conversation.turns.get(at.turn))
        .map(|turn| turn.question.clone());
    if let Some(question) = question {
        super::composer::ask(ui, question);
    }
}

/// Undo ends when another conversation opens (owner decision 2).
pub(super) fn follow_open(ui: &mut Ui) {
    let pane = &mut ui.assistant;
    if pane.undo_for != pane.open {
        pane.undo.retain(|at| Some(at.conversation) == pane.open);
        pane.undo_for = pane.open;
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::Request;
    use super::*;
    use crate::assistant::changes::Item;
    use crate::assistant::store::Turn;
    use serde_json::json;

    /// A live window has no scene's list: the models on disk are read,
    /// and the model reads them by name, the one in use marked.
    #[test]
    fn a_live_window_reports_the_models_on_disk() {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.picker.installed = None;
        assert!(
            context(&ui).state.installed.known().is_some(),
            "read from disk"
        );

        ui.assistant.fixture = true;
        ui.picker.installed = Some(vec![TranscriptionModelId::ParakeetV3]);
        let mut context = context(&ui);
        context.settings.transcription.model = TranscriptionModelId::ParakeetV3;
        let told = facts::text::told(&context.facts());
        assert!(
            told.contains("Speech models downloaded: Parakeet v3 (parakeet_v3, in use)\n"),
            "{told}"
        );
        context.state.installed = facts::Fact::Unknown;
        let told = facts::text::told(&context.facts());
        assert!(
            told.contains("Speech models downloaded: unknown\n"),
            "{told}"
        );
    }

    /// A scene's pane with one card in an open conversation.
    fn with_card(key: &str, from: Value, to: Value) -> (Ui, At) {
        let mut ui = super::super::super::scenes::mac_fixture();
        ui.assistant.fixture = true;
        let id = ui.assistant.store.start("q", None, None);
        ui.assistant.store.push(
            id,
            Turn {
                question: "q".into(),
                answer: "a".into(),
                cards: vec![Card {
                    items: vec![Item {
                        key: key.into(),
                        from,
                        to,
                        included: true,
                        ..Item::default()
                    }],
                    ..Card::default()
                }],
                ..Turn::default()
            },
        );
        ui.assistant.open = Some(id);
        follow_open(&mut ui);
        let at = At {
            conversation: id,
            turn: 0,
            card: 0,
        };
        (ui, at)
    }

    #[test]
    fn apply_saves_through_check_and_undo_puts_it_back() {
        let (mut ui, at) = with_card("labs.spoken_formatting", json!(false), json!(true));
        ui.settings.labs.spoken_formatting = false;
        assert_eq!(newest(&ui, at.conversation), Some(at));
        apply(&mut ui, at);
        assert!(ui.settings.labs.spoken_formatting);
        assert_eq!(shown(&ui, at), Some(Shown::Applied { undo: true }));
        assert_eq!(newest(&ui, at.conversation), None);
        undo(&mut ui, at);
        assert!(!ui.settings.labs.spoken_formatting);
        assert_eq!(card(&ui, at).unwrap().decision, Decision::Undone);
    }

    #[test]
    fn a_change_brings_what_it_needs_and_undo_takes_both_back() {
        let (mut ui, at) = with_card(
            "dictation_processing.server.url",
            json!(""),
            json!("http://localhost:1234/v1"),
        );
        ui.settings.labs.ai_cleanup_server = false;
        ui.settings.dictation_processing.server.url = String::new();
        set(&mut ui, at, |card| {
            card.items[0].requires = vec![("labs.ai_cleanup_server".into(), json!(true))];
        });
        apply(&mut ui, at);
        assert!(ui.settings.labs.ai_cleanup_server);
        assert_eq!(
            ui.settings.dictation_processing.server.url,
            "http://localhost:1234/v1"
        );
        undo(&mut ui, at);
        assert!(!ui.settings.labs.ai_cleanup_server);
        assert!(ui.settings.dictation_processing.server.url.is_empty());
    }

    #[test]
    fn undo_is_refused_after_a_hand_change_and_ends_with_the_conversation() {
        let (mut ui, at) = with_card("labs.spoken_formatting", json!(false), json!(true));
        ui.settings.labs.spoken_formatting = false;
        apply(&mut ui, at);
        // Turned off again by hand: the card says so and Undo does nothing.
        ui.settings.labs.spoken_formatting = false;
        undo(&mut ui, at);
        assert!(matches!(
            card(&ui, at).unwrap().decision,
            Decision::Applied { .. }
        ));
        ui.settings.labs.spoken_formatting = true;
        assert_eq!(shown(&ui, at), Some(Shown::Applied { undo: true }));
        // Another conversation opens: Undo is over for this one.
        ui.assistant.open = None;
        follow_open(&mut ui);
        ui.assistant.open = Some(at.conversation);
        follow_open(&mut ui);
        assert_eq!(shown(&ui, at), Some(Shown::Applied { undo: false }));
    }

    #[test]
    fn a_refused_save_fails_the_card_and_changes_nothing() {
        let (mut ui, at) = with_card("recording_duck_volume", json!(0.2), json!(1.4));
        let before = ui.settings.clone();
        apply(&mut ui, at);
        assert_eq!(ui.settings, before);
        assert!(matches!(
            card(&ui, at).unwrap().decision,
            Decision::Failed { .. }
        ));
    }

    #[test]
    fn a_shorter_history_retention_reaches_the_history_store() {
        let (mut ui, at) = with_card("history_retention", json!("forever"), json!("week"));
        ui.settings.history_retention = crate::history::HistoryRetention::Forever;
        apply(&mut ui, at);
        assert_eq!(
            ui.settings.history_retention,
            crate::history::HistoryRetention::Week
        );
    }

    /// A switch's companion saves with it: picked in the row, its value
    /// goes in the same save; on a row kept as it is, picked anew, it is
    /// a change of its own, counted and saved; untouched, it is none.
    /// Undo takes them back.
    #[test]
    fn a_switchs_companion_saves_with_it() {
        use crate::assistant::changes::{companion, counts};
        let (mut ui, at) = with_card("labs.unload_idle", json!(false), json!(true));
        ui.settings.labs.unload_idle = false;
        ui.settings.labs.unload_idle_minutes = 15;
        ui.settings.meetings.enabled = true;
        ui.settings.meetings.calendar.remind = true;
        ui.settings.meetings.calendar.lead_minutes = 1;
        let settings = ui.settings.clone();
        set(&mut ui, at, |card| {
            card.kind = CardKind::Plan;
            card.items[0].companion = companion::of(&settings, "labs.unload_idle", &json!(true));
            card.kept.push(Item {
                key: "meetings.calendar.remind".into(),
                from: json!(true),
                to: json!(true),
                companion: companion::of(&settings, "meetings.calendar.remind", &json!(true)),
                ..Item::default()
            });
        });
        let plan = card(&ui, at).unwrap().clone();
        assert_eq!(plan.items[0].companion.as_ref().unwrap().from, json!("15"));
        // Calendar reminders are macOS's: elsewhere the kept row has none.
        let reminds = plan.kept[0].companion.is_some();
        assert_eq!(
            reminds,
            crate::control::settings::unavailable("meetings.calendar.lead_minutes").is_none()
        );
        let one = counts::Counts {
            changes: 1,
            terms: 0,
        };
        assert_eq!(
            counts::held(&plan),
            (one, one),
            "an untouched companion is no change"
        );
        // Picked: 60 minutes on the row, 5 on the kept reminder.
        pick_companion(&mut ui, at, 0, false, 3);
        pick_companion(&mut ui, at, 0, true, 3);
        let plan = card(&ui, at).unwrap().clone();
        let two = counts::Counts {
            changes: 1 + usize::from(reminds),
            terms: 0,
        };
        assert_eq!(counts::held(&plan), (two, two));
        apply(&mut ui, at);
        assert!(ui.settings.labs.unload_idle);
        assert_eq!(ui.settings.labs.unload_idle_minutes, 60);
        let lead = if reminds { 5 } else { 1 };
        assert_eq!(ui.settings.meetings.calendar.lead_minutes, lead);
        let plan = card(&ui, at).unwrap().clone();
        assert_eq!(counts::applied(&plan), (two, two));
        undo(&mut ui, at);
        assert!(!ui.settings.labs.unload_idle);
        assert_eq!(ui.settings.labs.unload_idle_minutes, 15);
        assert_eq!(ui.settings.meetings.calendar.lead_minutes, 1);
        // A card from an answer carries it the same way.
        let (mut ui, at) = with_card("labs.unload_idle", json!(false), json!(true));
        ui.settings.labs.unload_idle = false;
        ui.settings.labs.unload_idle_minutes = 30;
        let settings = ui.settings.clone();
        set(&mut ui, at, |card| {
            card.items[0].companion = companion::of(&settings, "labs.unload_idle", &json!(true));
        });
        pick_companion(&mut ui, at, 0, false, 0);
        apply(&mut ui, at);
        assert!(ui.settings.labs.unload_idle);
        assert_eq!(ui.settings.labs.unload_idle_minutes, 5);
    }

    /// A Codex row waits for Codex: while the CLI is missing it can't be
    /// checked, a save leaves it out, and the rest go in; once Codex is
    /// ready it shows checked as it was made.
    #[test]
    fn a_codex_row_waits_for_codex_and_the_rest_save() {
        let (mut ui, at) = with_card(
            "meetings.notes.writer",
            json!(""),
            json!("codex:gpt-6.1-terra"),
        );
        ui.settings.labs.filler_words = false;
        set(&mut ui, at, |card| {
            card.kind = CardKind::Plan;
            card.items.push(Item {
                key: "labs.filler_words".into(),
                from: json!(false),
                to: json!(true),
                included: true,
                ..Item::default()
            });
        });
        ui.assistant.codex_as = Codex::Missing;
        let open = |ui: &Ui| state::row(card(ui, at).unwrap(), 0, &live(ui, at));
        assert!(matches!(
            open(&ui),
            state::Row::Open {
                checked: false,
                codex: Some(Codex::Missing),
                ..
            }
        ));
        toggle(&mut ui, at, 0);
        assert!(card(&ui, at).unwrap().items[0].included, "the toggle waits");
        ui.assistant.codex_as = Codex::Ready;
        assert!(matches!(
            open(&ui),
            state::Row::Open {
                checked: true,
                codex: None,
                ..
            }
        ));
        ui.assistant.codex_as = Codex::Checking;
        let writer = ui.settings.meetings.notes.writer.clone();
        apply(&mut ui, at);
        assert_eq!(
            ui.settings.meetings.notes.writer, writer,
            "Codex row left out"
        );
        assert!(ui.settings.labs.filler_words, "the rest saved");
        let plan = card(&ui, at).unwrap();
        assert!(!plan.items[0].included);
        assert_eq!(state::row(plan, 0, &live(&ui, at)), state::Row::Left);
    }

    /// A plan's rows are checked one by one: a failing row is left out
    /// with why, the rest save together, and Undo takes them back.
    #[test]
    fn a_plan_saves_what_passes_and_names_what_failed() {
        let (mut ui, at) = with_card("labs.spoken_formatting", json!(false), json!(true));
        ui.settings.labs.spoken_formatting = false;
        ui.settings.labs.filler_words = false;
        set(&mut ui, at, |card| {
            card.kind = CardKind::Plan;
            card.items.push(Item {
                key: "recording_duck_volume".into(),
                from: json!(0.2),
                to: json!(1.4),
                included: true,
                ..Item::default()
            });
            card.items.push(Item {
                key: "labs.filler_words".into(),
                from: json!(false),
                to: json!(true),
                included: true,
                ..Item::default()
            });
            card.items.push(Item {
                key: "labs.spoken_symbols".into(),
                from: json!(false),
                to: json!(true),
                included: false,
                ..Item::default()
            });
        });
        let files = ui.settings.labs.spoken_symbols;
        apply(&mut ui, at);
        assert!(ui.settings.labs.spoken_formatting && ui.settings.labs.filler_words);
        assert_eq!(
            ui.settings.labs.spoken_symbols, files,
            "an unchecked row stays"
        );
        let plan = card(&ui, at).unwrap().clone();
        assert!(matches!(plan.decision, Decision::Applied { .. }));
        assert!(plan.items[1].failed.is_some());
        assert_eq!(state::applied_count(&plan), (2, 3));
        assert_eq!(shown(&ui, at), Some(Shown::Applied { undo: true }));
        undo(&mut ui, at);
        assert!(!ui.settings.labs.spoken_formatting && !ui.settings.labs.filler_words);
        // Nothing checked: nothing saved, and the plan says so.
        let (mut ui, at) = with_card("labs.spoken_formatting", json!(false), json!(true));
        set(&mut ui, at, |card| {
            card.kind = CardKind::Plan;
            card.items[0].included = false;
        });
        apply(&mut ui, at);
        assert!(matches!(
            card(&ui, at).unwrap().decision,
            Decision::Failed { .. }
        ));
    }

    /// Edit first: the card's own text box, saved through the same check;
    /// Cancel edit leaves the model's text, and Undo puts back the old.
    #[test]
    fn edited_instructions_save_and_undo_like_the_models() {
        let key = changes::parts::PROMPT;
        let before = ui_prompt(&with_card(key, json!(""), json!("")).0);
        let (mut ui, at) = with_card(key, json!(before), json!("Fix punctuation."));
        set(&mut ui, at, |card| card.kind = CardKind::Prompt);
        edit(&mut ui, at);
        assert_eq!(ui.assistant.editing, Some(at));
        assert_eq!(ui.assistant.edit.text, "Fix punctuation.");
        ui.assistant.edit = rune::TextEdit::new("Never translate code.".to_owned());
        cancel_edit(&mut ui);
        assert_eq!(
            card(&ui, at).unwrap().items[0].to,
            json!("Fix punctuation.")
        );
        edit(&mut ui, at);
        ui.assistant.edit = rune::TextEdit::new("  Never translate code.\n".to_owned());
        save_edit(&mut ui, at);
        assert!(ui.assistant.editing.is_none());
        assert_eq!(ui_prompt(&ui), "Never translate code.");
        assert_eq!(shown(&ui, at), Some(Shown::Applied { undo: true }));
        undo(&mut ui, at);
        assert_eq!(ui_prompt(&ui), before);
    }

    fn ui_prompt(ui: &Ui) -> String {
        let mode = &ui.settings.dictation_processing.default_mode;
        mode.post_processing.prompt.clone()
    }

    #[test]
    fn a_speech_model_goes_through_the_service() {
        let (mut ui, at) = with_card(
            "transcription.model",
            json!("parakeet_v3"),
            json!("gigaam_v3"),
        );
        set(&mut ui, at, |card| card.items[0].class = Class::Download);
        apply(&mut ui, at);
        let super::super::super::Service::Fixture(sent) = &ui.service else {
            panic!("not a scene");
        };
        assert!(
            sent.iter()
                .any(|request| matches!(request, Request::ChooseModel { .. })),
            "{sent:?}"
        );
    }

    /// A plan's speech model and language rows go to the service as one
    /// choice, the language with the model it comes with: a second
    /// request would ask for the language on the model saved now.
    #[test]
    fn a_plans_model_and_language_are_one_choice() {
        let (mut ui, at) = with_card(
            "transcription.model",
            json!("parakeet_unified_en"),
            json!("parakeet_v3"),
        );
        set(&mut ui, at, |card| {
            card.kind = CardKind::Plan;
            card.items[0].class = Class::Download;
            card.items.push(Item {
                key: "transcription.language".into(),
                from: json!("en"),
                to: json!("es"),
                included: true,
                class: Class::Download,
                ..Item::default()
            });
        });
        apply(&mut ui, at);
        let super::super::super::Service::Fixture(sent) = &ui.service else {
            panic!("not a scene");
        };
        let chosen: Vec<_> = sent
            .iter()
            .filter_map(|request| match request {
                Request::ChooseModel { model, language } => Some((*model, language.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(
            chosen,
            [(TranscriptionModelId::ParakeetV3, "es".to_owned())],
            "{sent:?}"
        );
    }
}
