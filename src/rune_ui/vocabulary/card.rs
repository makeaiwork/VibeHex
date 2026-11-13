//! The Add to vocabulary card's panel (Labs › Add to vocabulary from a
//! selection), built for any state: the macOS overlay
//! (`super::super::vocabulary_card`) holds it over another app, and the
//! `vocabulary-card` scenes hold it in the window, so every OS renders it.

use rune::{Color, El, TextEdit, Tx, col, el, row, text};
use rune_kit::{Button, ButtonVariant, TextInput};

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint, PushSize};
use crate::desktop::i18n::{tr, tr_fill};

/// The panel's width; the overlay adds room for its shadow.
pub(crate) const WIDTH: f32 = 436.0;
const LABEL_WIDTH: f32 = 96.0;

/// What the panel shows, and what its controls reach in state `S`.
pub(crate) struct Panel<'a, S> {
    pub(crate) heard: &'a TextEdit,
    pub(crate) term: &'a TextEdit,
    /// Lines under the fields, each in its colour.
    pub(crate) notes: Vec<(String, Color)>,
    /// Heard's field, or with `true` Should be's.
    pub(crate) field: fn(&mut S, bool) -> &mut TextEdit,
    pub(crate) add: fn(&mut S),
    pub(crate) cancel: fn(&mut S),
}

/// The note that says AI cleanup wrote the selected word, not
/// transcription.
pub(crate) fn cleaned_note(p: Paint, cleaned: &str) -> (String, Color) {
    (tr_fill("AI cleanup wrote it as {}", cleaned), p.muted())
}

pub(crate) fn panel<S: 'static>(p: Paint, panel: Panel<'_, S>) -> El {
    let c = &p.k.c;
    let (add, cancel) = (panel.add, panel.cancel);
    let mut card = col()
        .key("card")
        .role("dialog")
        .label(tr("Add to vocabulary"))
        .w(WIDTH)
        .padding(16.0)
        .gap(10.0)
        .radius(p.k.radius.xl)
        .fill(c.bg_base)
        .border(p.hair, c.border_weak)
        .child(
            text(tr("Add to vocabulary"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(field(p, &panel, false))
        .child(field(p, &panel, true));
    for (index, (note, color)) in panel.notes.into_iter().enumerate() {
        card = card.child(
            text(note)
                .key(format!("note-{index}"))
                .font(DESCRIPTION)
                .color(color)
                .wrap(),
        );
    }
    card = card.child(
        row()
            .key("footer")
            .gap(10.0)
            .center_y()
            .child(el().key("spacer").grow(1.0))
            .child(
                Button::new(tr("Cancel"))
                    .variant(ButtonVariant::Neutral)
                    .on_press_tx(move |state: &mut S, _: &mut Tx| cancel(state))
                    .build_push(p.k)
                    .key("cancel"),
            )
            .child(
                Button::new(tr("Add"))
                    .variant(ButtonVariant::Contrast)
                    .on_press_tx(move |state: &mut S, _: &mut Tx| add(state))
                    .build_push(p.k)
                    .key("add"),
            ),
    );
    for shadow in &p.k.elevation.overlay {
        card = card.box_shadow(*shadow);
    }
    card
}

/// Heard's row, or with `term` Should be's.
fn field<S: 'static>(p: Paint, panel: &Panel<'_, S>, term: bool) -> El {
    let (key, label, edit, placeholder) = if term {
        (
            "term",
            tr("Should be"),
            panel.term,
            tr("The right spelling"),
        )
    } else {
        (
            "heard",
            tr("Heard"),
            panel.heard,
            tr("What was transcribed"),
        )
    };
    let (access, add) = (panel.field, panel.add);
    row()
        .key(key)
        .gap(10.0)
        .center_y()
        .child(
            text(label.to_string())
                .key("label")
                .w(LABEL_WIDTH)
                .font(p.k.ty.ui)
                .color(p.muted()),
        )
        .child(
            TextInput::new(edit)
                .placeholder(placeholder)
                .width(None)
                .on_edit(move |state: &mut S| access(state, term))
                .on_submit(move |state: &mut S, _: &mut Tx| add(state))
                .build(p.k)
                .key("input")
                .grow(1.0)
                .shrink(),
        )
}

/// The card in the window, for the `vocabulary-card` scenes: what the
/// shortcut opens over another app, on the window's backdrop. Add and
/// Cancel close it; nothing is saved.
#[derive(Default)]
pub(in super::super) struct Scene {
    heard: TextEdit,
    term: TextEdit,
    cleaned: Option<String>,
    shown: bool,
}

/// Opens the scene's card over "cloud code", Should be focused and holding
/// `term`; `cleaned` is what AI cleanup wrote there.
pub(in super::super) fn fixture(ui: &mut Ui, term: &str, cleaned: Option<&str>) {
    let mut term = TextEdit::new(term);
    term.focused = true;
    ui.vocabulary.card = Some(Scene {
        heard: TextEdit::new("cloud code"),
        term,
        cleaned: cleaned.map(str::to_string),
        shown: true,
    });
}

fn scene_field(ui: &mut Ui, term: bool) -> &mut TextEdit {
    let scene = ui.vocabulary.card.get_or_insert_with(Scene::default);
    if term {
        &mut scene.term
    } else {
        &mut scene.heard
    }
}

fn scene_close(ui: &mut Ui) {
    if let Some(scene) = &mut ui.vocabulary.card {
        scene.shown = false;
        scene.heard.focused = false;
        scene.term.focused = false;
    }
}

/// The scene's layers: the window's backdrop, then the card near its top.
pub(super) fn scene(ui: &Ui, p: Paint) -> Option<[El; 2]> {
    let scene = ui.vocabulary.card.as_ref()?;
    let layer = |key: &str| {
        el().key(key)
            .role("none")
            .absolute(0.0, 0.0)
            .w_full()
            .h_full()
    };
    let mut front = layer("vocabulary-card");
    if scene.shown {
        let notes = scene
            .cleaned
            .as_deref()
            .map(|cleaned| cleaned_note(p, cleaned))
            .into_iter()
            .collect();
        front = front.child(
            col()
                .key("vocabulary-card-spot")
                .w_full()
                .items_center()
                .pt(16.0)
                .child(panel(
                    p,
                    Panel {
                        heard: &scene.heard,
                        term: &scene.term,
                        notes,
                        field: scene_field,
                        add: scene_close,
                        cancel: scene_close,
                    },
                )),
        );
    }
    Some([layer("vocabulary-card-backdrop").fill(p.k.c.bg_deep), front])
}
