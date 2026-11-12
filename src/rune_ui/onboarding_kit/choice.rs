//! The first run's model step, where a first run with no model begins: the
//! dictation language (the system's, and changeable here), the models worth
//! offering for it from the picker's own recommendations, the recommended
//! one first and selected, then any compatible model already on this
//! computer. Selecting a row starts nothing. One button starts the selected
//! row (a download, or a file used in place), "All models…" opens the full
//! picker, and "Continue without a model" is there from the start.
//!
//! The choice itself is the tour's ([`ModelChoice`]); this module turns it
//! into rows and sends the person's presses back.

use rune::{El, Tx, col, icon, row, text};
use rune_kit::{
    Badge, Button, ButtonVariant, ControlSize, IconButton, IconButtonVariant, Placement, RadioItem,
    Select, SelectAppearance, glyphs,
};

use super::super::controls::{CardRow, DESCRIPTION, METRICS, Paint, PushSize, card};
use super::super::{Ui, onboarding, picker};
use super::landing::{LEAD, arrives};
use super::live::{self, Live};
use super::stories::Onboard;
use crate::desktop::i18n::{tr, tr_fill};
use crate::desktop::onboarding::{Input, ItemStatus, ModelChoice, Pick, SetupItem, View};
use crate::transcription::models::{
    self as models, LANGUAGES, ModelPreparationStage, Recommendation,
};

/// The list, as wide as the tour's column.
const LIST_W: f32 = 540.0;
const LANGUAGE_W: f32 = 200.0;
const ACTION_H: f32 = 32.0;
/// The offers, then whatever else fits the smallest window.
const MAX_ROWS: usize = 5;

/// How a row's start is going.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowState {
    /// Not started: selecting it starts nothing.
    Idle,
    /// The person started it: downloading, or being checked.
    Working,
    Ready,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Row {
    pub(crate) pick: Pick,
    pub(crate) name: String,
    /// One line: why you'd pick it, or how its start is going.
    pub(crate) line: String,
    /// The download's size, or that the file is already here.
    pub(crate) size: String,
    /// The picker's badge for it, in this language.
    pub(crate) tag: Option<&'static str>,
    /// Already on this computer: starting it downloads nothing.
    pub(crate) here: bool,
    pub(crate) selected: bool,
    pub(crate) state: RowState,
}

/// What the one main button does with the selected row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Primary {
    Download,
    /// A model already on this computer.
    Use,
    /// The selected row is already under way, or ready.
    Continue,
    Retry,
}

/// The model step as it shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Shown {
    pub(crate) language: &'static str,
    pub(crate) rows: Vec<Row>,
    pub(crate) primary: Primary,
}

impl Shown {
    fn selected(&self) -> Option<&Row> {
        self.rows.iter().find(|row| row.selected)
    }
}

/// The rows' picks with the picker's badge, before the disk is looked at:
/// the offers, a model chosen from the full list (or a model file), then
/// the compatible models found on this computer that speak the language.
fn candidates(ui: &Ui, choice: &ModelChoice) -> Vec<(Pick, Option<Recommendation>)> {
    let mut picks: Vec<(Pick, Option<Recommendation>)> = choice
        .offered()
        .iter()
        .map(|&(model, recommendation)| (Pick::Model(model), recommendation))
        .collect();
    let kept = [choice.selected(), choice.started()];
    for pick in kept.into_iter().flatten() {
        if matches!(pick, Pick::Model(_)) && !picks.iter().any(|(listed, _)| *listed == pick) {
            picks.push((pick, None));
        }
    }
    let language = choice.language();
    for (index, found) in ui.found.models.iter().enumerate() {
        let pick = found.catalog.map_or(Pick::Found(index), Pick::Model);
        let speaks = match found.catalog {
            Some(model) => models::definition(model).supports_language(language),
            None => {
                language == models::AUTO_LANGUAGE
                    || found
                        .languages
                        .iter()
                        .any(|code| code == language || code == "*")
            }
        };
        let fits = picks.len() < MAX_ROWS || kept.contains(&Some(pick));
        if speaks && fits && !picks.iter().any(|(listed, _)| *listed == pick) {
            picks.push((pick, None));
        }
    }
    picks
}

/// The picks already on this computer: installed, or another app's exact
/// file. This reads the disk, so the first run asks a few times a second.
pub(in crate::rune_ui) fn here(ui: &Ui, choice: &ModelChoice) -> Vec<Pick> {
    candidates(ui, choice)
        .into_iter()
        .map(|(pick, _)| pick)
        .filter(|pick| match *pick {
            Pick::Found(_) => true,
            Pick::Model(id) => {
                let model = models::definition(id);
                ui.picker
                    .installed(model, model.language_for(choice.language()))
                    || (!id.is_custom()
                        && ui.found.models.iter().any(|found| {
                            found.catalog == Some(id)
                                && !crate::transcription::found_models::is_forgotten(
                                    model,
                                    &found.path,
                                )
                        }))
            }
        })
        .collect()
}

/// The model step for the tour's `view`, with `here` from [`here`].
pub(in crate::rune_ui) fn shown(ui: &Ui, view: &View, here: &[Pick]) -> Shown {
    let choice = &view.choice;
    let rows: Vec<Row> = candidates(ui, choice)
        .into_iter()
        .filter_map(|(pick, recommendation)| {
            row_for(ui, view, pick, recommendation, here.contains(&pick))
        })
        .collect();
    let primary = match rows.iter().find(|row| row.selected) {
        Some(row) => match row.state {
            RowState::Failed => Primary::Retry,
            RowState::Working | RowState::Ready => Primary::Continue,
            RowState::Idle if row.here => Primary::Use,
            RowState::Idle => Primary::Download,
        },
        None => Primary::Download,
    };
    Shown {
        language: choice.language(),
        rows,
        primary,
    }
}

fn row_for(
    ui: &Ui,
    view: &View,
    pick: Pick,
    recommendation: Option<Recommendation>,
    here: bool,
) -> Option<Row> {
    let choice = &view.choice;
    let (name, blurb, size) = match pick {
        Pick::Model(id) => {
            let model = models::definition(id);
            let blurb = match picker::blurb(model) {
                "" => tr("Not tested by Hex"),
                blurb => blurb,
            };
            (model.name.to_owned(), blurb, picker::size_text(model))
        }
        Pick::Found(index) => {
            let found = ui.found.models.get(index)?;
            (found.name.clone(), tr("Not tested by Hex"), String::new())
        }
    };
    let state = if choice.started() != Some(pick) {
        RowState::Idle
    } else {
        match view.model {
            ItemStatus::Failed => RowState::Failed,
            ItemStatus::Ready if !ui.found.adopting() => RowState::Ready,
            _ => RowState::Working,
        }
    };
    let checking = ui.found.adopting()
        || ui.view.as_ref().is_some_and(|view| {
            matches!(
                view.transcription.preparation_stage,
                Some(ModelPreparationStage::Verifying | ModelPreparationStage::Loading)
            )
        });
    let line = match (state, view.model) {
        (RowState::Idle, _) => blurb.to_owned(),
        (RowState::Failed, _) => tr("Couldn't prepare this model").to_owned(),
        (RowState::Ready, _) => tr("Ready").to_owned(),
        (RowState::Working, ItemStatus::Downloading(progress)) if !here && !checking => tr_fill(
            "Downloading… {}%",
            &((progress * 100.0).round() as u32).to_string(),
        ),
        (RowState::Working, _) => tr("Checking…").to_owned(),
    };
    Some(Row {
        pick,
        name,
        line,
        size: if here {
            tr("On this computer").to_owned()
        } else {
            size
        },
        tag: recommendation.map(picker::recommendation_label),
        here,
        selected: choice.selected() == Some(pick),
        state,
    })
}

/// The name of what the person started, for the download pill.
pub(in crate::rune_ui) fn started_name(ui: &Ui, choice: &ModelChoice) -> Option<String> {
    Some(match choice.started()? {
        Pick::Model(id) => models::definition(id).name.to_owned(),
        Pick::Found(index) => ui.found.models.get(index)?.name.clone(),
    })
}

/// The catalog model the person started, whose bytes the pill counts.
pub(in crate::rune_ui) fn started_model(
    ui: &Ui,
    choice: &ModelChoice,
) -> Option<&'static models::ModelDefinition> {
    match choice.started()? {
        Pick::Model(id) => Some(models::definition(id)),
        Pick::Found(index) => ui.found.models.get(index)?.catalog.map(models::definition),
    }
}

fn select(pick: Pick) -> impl Fn(&mut Ui, &mut Tx) + 'static {
    move |ui: &mut Ui, tx: &mut Tx| {
        tx.instant();
        onboarding::input(ui, Input::Pick(pick));
        live::refresh_choice(ui);
    }
}

/// The main button: starts the selected row unless it is already under
/// way, and the story goes on while it downloads.
pub(in crate::rune_ui) fn press_primary(ui: &mut Ui) {
    live::refresh_choice(ui);
    let Some((primary, here)) =
        step(ui).map(|shown| (shown.primary, shown.selected().is_some_and(|row| row.here)))
    else {
        return;
    };
    if primary != Primary::Continue {
        onboarding::input(ui, Input::Grant(SetupItem::Model));
    }
    live::model_chosen(ui, here);
}

/// The × on the row the person started: its download stops, and nothing is
/// chosen again.
pub(in crate::rune_ui) fn cancel(ui: &mut Ui) {
    onboarding::input(ui, Input::CancelModel);
    live::refresh_choice(ui);
}

/// "All models…": the full picker, for the language chosen here. A model
/// chosen there comes back as the started row ([`onboarding::choose`]).
pub(in crate::rune_ui) fn all_models(ui: &mut Ui) {
    let Some(language) = step(ui).map(|shown| shown.language) else {
        return;
    };
    set_language_open(ui, false);
    picker::open_for_setup(ui, language);
}

/// The dictation language chosen on the model step.
pub(in crate::rune_ui) fn choose_language(ui: &mut Ui, language: &'static str) {
    set_language_open(ui, false);
    onboarding::input(ui, Input::Language(language));
    live::refresh_choice(ui);
}

fn step(ui: &Ui) -> Option<&Shown> {
    ui.first_run.as_ref()?.live.as_ref()?.choice.as_ref()
}

fn set_language_open(ui: &mut Ui, open: bool) {
    if let Some(live) = ui.first_run.as_mut().and_then(|s| s.live.as_mut()) {
        live.language_open = open;
    }
}

fn toggle_language(ui: &mut Ui) {
    let open = ui
        .first_run
        .as_ref()
        .and_then(|s| s.live.as_ref())
        .is_some_and(|live| live.language_open);
    set_language_open(ui, !open);
}

/// The language field: the app's own recognition languages, Auto included,
/// by the names General's menu uses.
fn language_select(shown: &Shown, open: bool) -> Select {
    let labels: Vec<&'static str> = LANGUAGES.iter().map(|(_, name)| *name).collect();
    Select::new("setup-language", &labels)
        .appearance(SelectAppearance::Field)
        .width(LANGUAGE_W)
        .label(tr("Dictation language"))
        .current(Some(models::language_name(shown.language)))
        .open(open)
        .placement(Placement::BottomStart)
        .on_toggle(toggle_language)
        .on_close(|ui: &mut Ui| set_language_open(ui, false))
        .on_choose(|ui: &mut Ui, value: &str, _: &mut Tx| {
            if let Some((code, _)) = LANGUAGES.iter().find(|(_, name)| *name == value) {
                choose_language(ui, code);
            }
        })
}

/// The open language menu, above the first run's own layer.
pub(in crate::rune_ui) fn language_menu(p: Paint, s: &Onboard, layer: &mut Vec<El>) {
    let Some(live) = s.live.as_ref().filter(|live| live.language_open) else {
        return;
    };
    if let Some(shown) = &live.choice {
        let _ = language_select(shown, true).build(p.k, layer);
    }
}

/// The page: the language, the list, the one main button, and the ways
/// round it.
pub(super) fn page(p: Paint, s: &Onboard, live: &Live) -> El {
    let Some(shown) = &live.choice else {
        return col().key("models");
    };
    let language = row()
        .key("language")
        .gap(10.0)
        .center_y()
        .child(
            text(tr("Dictation language"))
                .key("label")
                .font(LEAD)
                .color(p.muted())
                .a11y_hidden(),
        )
        .child(language_select(shown, live.language_open).trigger(p.k));
    let rows: Vec<CardRow> = shown
        .rows
        .iter()
        .enumerate()
        .map(|(index, model)| model_row(p, index, model).into())
        .collect();
    let list = card(p, "model-list", rows).w(LIST_W);
    let actions = row()
        .key("action")
        .h(ACTION_H)
        .gap(10.0)
        .center()
        .child(
            Button::new(tr("All models…"))
                .variant(ButtonVariant::Neutral)
                .on_press(all_models)
                .build_push(p.k)
                .key("all-models"),
        )
        .child(primary_button(p, shown.primary));
    col()
        .key("models")
        .gap(18.0)
        .items_center()
        .child(super::first_run::heading(
            p,
            s,
            "models-head",
            tr("Choose a speech model"),
            language,
        ))
        .child(s.motion.staggered(list, 1.0, 8.0))
        .child(actions)
        .child(without_model(p))
}

fn model_row(p: Paint, index: usize, model: &Row) -> El {
    let pick = model.pick;
    let radio = RadioItem::new(&model.name, model.selected)
        .on_select_tx(select(pick))
        .build(p.k)
        .key("radio")
        .description(model.line.clone());
    let tag = model
        .tag
        .map(|tag| Badge::new(tag).build(p.k).key("tag").a11y_hidden());
    let line_ink = if model.state == RowState::Failed {
        p.danger()
    } else {
        p.muted()
    };
    let words = col()
        .key("words")
        .grow(1.0)
        .shrink()
        .gap(3.0)
        .child(
            row()
                .key("title")
                .gap(8.0)
                .center_y()
                .child(radio)
                .children(tag),
        )
        .child(
            text(model.line.clone())
                .key("line")
                .font(DESCRIPTION)
                .color(line_ink)
                .pl(24.0)
                .wrap()
                .a11y_hidden(),
        );
    let trailing = if model.state == RowState::Working {
        IconButton::new(p.k.glyph.close)
            .variant(IconButtonVariant::Ghost)
            .control_size(ControlSize::Small)
            .label(tr("Cancel"))
            .on_press(cancel)
            .build(p.k)
            .key("cancel-model")
    } else {
        text(model.size.clone())
            .key("size")
            .font(p.k.ty.small.features("'tnum' 1"))
            .color(p.faint())
    };
    // The radio inside is the choice assistive technology reads; the row
    // only widens where a press lands.
    row()
        .key(format!("model{index}"))
        .role("none")
        .min_h(METRICS.row_h_described - 6.0)
        .px(METRICS.row_px)
        .py(8.0)
        .gap(12.0)
        .center_y()
        .radius(p.k.radius.md)
        .hover(p.k.c.hover)
        .on_press_tx(select(pick))
        .child(words)
        .child(trailing)
}

fn primary_button(p: Paint, primary: Primary) -> El {
    let label = match primary {
        Primary::Download => tr("Download"),
        Primary::Use => tr("Use this model"),
        Primary::Continue => tr("Continue"),
        Primary::Retry => tr("Try again"),
    };
    let onward = primary == Primary::Continue;
    Button::new(label)
        .variant(ButtonVariant::Contrast)
        .label_with(move |font, ink| {
            row()
                .gap(6.0)
                .center_y()
                .child(text(label).key("label").font(font).color(ink))
                .children(onward.then(|| icon(&glyphs::ARROW_RIGHT, 15.0).key("arrow").color(ink)))
        })
        .on_press(press_primary)
        .build_push(p.k)
        .key("continue")
        .default_button()
        .transition(arrives())
}

/// Round the model altogether, from the start: nothing downloads, and
/// Settings says how to add one later.
fn without_model(p: Paint) -> El {
    col()
        .key("without-model")
        .gap(4.0)
        .items_center()
        .child(
            Button::new(tr("Continue without a model"))
                .variant(ButtonVariant::Ghost)
                .on_press(live::continue_without_model)
                .build_push(p.k)
                .key("continue-without-model"),
        )
        .child(
            text(tr(
                "Dictation won't work until you choose a model in Settings.",
            ))
            .key("without-model-why")
            .font(DESCRIPTION)
            .color(p.muted())
            .align(rune::Align::Center)
            .wrap()
            .w(LIST_W),
        )
}
