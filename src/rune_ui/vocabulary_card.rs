//! Labs › Add to vocabulary from a selection (macOS): a small card that the
//! vocabulary shortcut opens over any app, just below the selected text (or
//! above it near the screen's bottom; at the pointer when the app doesn't
//! say where its selection is). It holds
//! the selected, misheard text ("Heard") and takes the right spelling
//! ("Should be"); Add saves the pair to Vocabulary and puts the spelling in
//! place of the selection where that text can be edited. Escape, Cancel,
//! or a click into another app close it with nothing saved.
//!
//! The card takes keys without activating Hex, so the app with the
//! selection stays in front and keeps its selection.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use rune::{
    Cx, El, KeyPolicy, OverlayHandle, OverlayInput, OverlayOptions, OverlayPlacement, OverlaySpec,
    ScreenEdge, TextEdit, Tx, col, el, row, text,
};
use rune_kit::{Button, ButtonVariant, TextInput};

use super::controls::{DESCRIPTION, Paint, PushSize};
use crate::accessibility::Selection;
use crate::desktop::i18n::{tr, tr_fill};
use crate::dictation::vocabulary::{self, VocabularyEntry};

/// The overlay's size: the card and room for its shadow.
const SIZE: (f32, f32) = (460.0, 210.0);
/// The card's distance below the top of the screen's work area, until it
/// is first placed by its selection.
const MARGIN: f32 = 56.0;
/// The room between the selection and the card.
const GAP: f64 = 6.0;
/// The pointer's own height below its hot spot, which the card stays clear of.
const POINTER_HEIGHT: f64 = 20.0;
const LABEL_WIDTH: f32 = 96.0;

/// Saves `term` with `heard` as one of its spellings; the error says why
/// it couldn't.
pub(crate) type Save = Arc<dyn Fn(String, String) -> Result<(), String> + Send + Sync>;

pub(crate) struct Card {
    heard: TextEdit,
    term: TextEdit,
    /// Where the selection came from, to replace it once the term is in.
    selection: Option<Selection>,
    /// Why the last Add didn't save.
    error: Option<String>,
    save: Option<Save>,
    overlay: Option<OverlayHandle<Card>>,
    /// The card sits above its selection, so it hangs from the overlay's
    /// bottom instead of its top.
    above: bool,
}

static CARD: OnceLock<OverlayHandle<Card>> = OnceLock::new();

/// Saves `term`, with `heard` among its spellings, as `hex vocabulary add`
/// does, then `reload`s so the runtime and an open window adopt it.
pub(crate) fn save_word(term: &str, heard: &str, reload: impl Fn()) -> Result<(), String> {
    let mut settings =
        crate::app_settings::AppSettings::load().map_err(|error| format!("{error:#}"))?;
    let heard: Vec<String> = (!heard.trim().is_empty())
        .then(|| heard.to_string())
        .into_iter()
        .collect();
    if vocabulary::add(&mut settings.vocabulary, term, &heard)? {
        settings.save().map_err(|error| format!("{error:#}"))?;
        reload();
    }
    Ok(())
}

/// Adds the card to the app, hidden; `save` writes Vocabulary.
pub(crate) fn install<S: 'static>(app: &mut rune::App<S>, save: Save) {
    let spec = OverlaySpec {
        name: "Hex add to vocabulary",
        size: SIZE,
        edge: ScreenEdge::Top,
        margin: MARGIN,
        align: rune::Align::Center,
        inset: 0.0,
        interactive: true,
        view,
    }
    .options(OverlayOptions {
        placement: Some(OverlayPlacement::Edge {
            edge: ScreenEdge::Top,
            margin: MARGIN,
            align: rune::Align::Center,
            inset: 0.0,
        }),
        // Placed in global points by `place`, from the primary display.
        display: rune::DisplayTarget::Primary,
        keys: KeyPolicy::OnDemand,
        input: OverlayInput::All,
        ..Default::default()
    })
    .on_key(on_key)
    .on_focus(on_focus);
    let card = Card {
        heard: TextEdit::default(),
        term: TextEdit::default(),
        selection: None,
        error: None,
        save: Some(save),
        overlay: None,
        above: false,
    };
    let overlay = app.overlay(spec, card);
    let handle = overlay.clone();
    overlay.update(move |card, _| card.overlay = Some(handle));
    let _ = CARD.set(overlay);
}

/// Opens the card for `heard`, the selected text (empty when no app said
/// what was selected), from any thread. A term that already lists `heard`
/// as a spelling, or is `heard` itself, starts in Should be.
pub(crate) fn open(selection: Option<Selection>, heard: String) {
    let Some(overlay) = CARD.get() else {
        return;
    };
    let known = known_term(&vocabulary::entries(), &heard);
    overlay.update(move |card, tx| {
        let empty = heard.is_empty();
        card.heard = TextEdit::new(heard);
        card.term = TextEdit::new(known.unwrap_or_default());
        // With nothing selected, Heard is typed first.
        card.heard.focused = empty;
        card.term.focused = !empty;
        let bounds = selection.as_ref().and_then(|selection| selection.bounds);
        card.selection = selection;
        card.error = None;
        tx.instant();
        if let Some(overlay) = &card.overlay {
            if let Some((x, y, above)) = spot(bounds) {
                card.above = above;
                overlay.set_placement(Some(OverlayPlacement::Rect {
                    x: x as f32,
                    y: y as f32,
                    width: SIZE.0,
                    height: SIZE.1,
                }));
            }
            overlay.show();
            overlay.request_keys();
        }
    });
}

/// Where the overlay goes, in global top-left points, and whether above
/// its anchor: the selection's rectangle, else the pointer. Read on the
/// main thread, where AppKit's screens are.
fn spot(selection: Option<[f64; 4]>) -> Option<(f64, f64, bool)> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen};
    let mtm = MainThreadMarker::new()?;
    let screens = NSScreen::screens(mtm);
    let primary = screens.iter().next()?.frame();
    // AppKit counts up from the primary display's bottom; global points
    // count down from its top.
    let top = primary.origin.y + primary.size.height;
    let anchor = selection.unwrap_or_else(|| {
        let pointer = NSEvent::mouseLocation();
        [pointer.x, top - pointer.y, 0.0, POINTER_HEIGHT]
    });
    let (center_x, center_y) = (anchor[0] + anchor[2] / 2.0, anchor[1] + anchor[3] / 2.0);
    let visible = screens
        .iter()
        .map(|screen| {
            let frame = screen.visibleFrame();
            [
                frame.origin.x,
                top - (frame.origin.y + frame.size.height),
                frame.size.width,
                frame.size.height,
            ]
        })
        .find(|v| {
            center_x >= v[0] && center_x < v[0] + v[2] && center_y >= v[1] && center_y < v[1] + v[3]
        })
        .or_else(|| {
            let frame = NSScreen::mainScreen(mtm)?.visibleFrame();
            Some([
                frame.origin.x,
                top - (frame.origin.y + frame.size.height),
                frame.size.width,
                frame.size.height,
            ])
        })?;
    Some(place(anchor, visible))
}

/// The overlay's top-left for `anchor` (`[x, y, width, height]`) inside the
/// screen's `visible` area, and whether it sits above the anchor: centred
/// under it when it fits, else over it, and kept on the screen.
fn place(anchor: [f64; 4], visible: [f64; 4]) -> (f64, f64, bool) {
    let (width, height) = (f64::from(SIZE.0), f64::from(SIZE.1));
    let left = visible[0];
    let right = (visible[0] + visible[2] - width).max(left);
    let x = (anchor[0] + anchor[2] / 2.0 - width / 2.0).clamp(left, right);
    let below = anchor[1] + anchor[3] + GAP;
    let bottom = visible[1] + visible[3];
    if below + height <= bottom {
        return (x, below, false);
    }
    let above = (anchor[1] - GAP - height).max(visible[1]);
    (x, above, true)
}

/// The vocabulary shortcut: reads what the app in front has selected,
/// through Accessibility or else a ⌘C that puts the clipboard back, and
/// opens the card for it. Off the listener's thread, which must not wait.
pub(crate) fn open_for_selection() {
    let spawned = std::thread::Builder::new()
        .name("vocabulary-selection".into())
        .spawn(|| {
            let selection = crate::dictation::context::frontmost_pid()
                .and_then(crate::accessibility::selection);
            let heard = match &selection {
                Some(selection) => selection.text.clone(),
                None => crate::paste::copy_selection(COPY_WAIT).unwrap_or_default(),
            };
            open(selection, heard);
        });
    if let Err(error) = spawned {
        tracing::warn!(%error, "couldn't read the selection for Vocabulary");
    }
}

/// How long a ⌘C may take to reach the clipboard.
const COPY_WAIT: Duration = Duration::from_millis(400);

/// The listed term `heard` already spells, if any: a term with it among
/// its spellings, or the term itself (any case).
fn known_term(entries: &[VocabularyEntry], heard: &str) -> Option<String> {
    let heard = heard.trim().to_lowercase();
    if heard.is_empty() {
        return None;
    }
    entries
        .iter()
        .find(|entry| {
            entry.term.to_lowercase() == heard
                || entry
                    .sounds_like
                    .iter()
                    .any(|spelling| spelling.to_lowercase() == heard)
        })
        .map(|entry| entry.term.clone())
}

fn close(card: &mut Card) {
    card.selection = None;
    card.error = None;
    card.heard.focused = false;
    card.term.focused = false;
    if let Some(overlay) = &card.overlay {
        overlay.hide();
    }
}

/// Add: saves the pair, closes, then replaces the selection with the term
/// where the app lets it be edited. A message already sent keeps its text;
/// the term is in Vocabulary either way.
fn add(card: &mut Card) {
    let term = card.term.text.trim().to_string();
    let heard = card.heard.text.trim().to_string();
    if term.is_empty() {
        card.term.focused = true;
        return;
    }
    let Some(save) = card.save.clone() else {
        return;
    };
    if let Err(error) = save(term.clone(), heard.clone()) {
        card.error = Some(error);
        return;
    }
    let selection = card.selection.take();
    close(card);
    if let Some(selection) = selection
        && term != heard
    {
        std::thread::Builder::new()
            .name("vocabulary-replace".into())
            .spawn(move || {
                // After the card gives the keys back to the app.
                std::thread::sleep(Duration::from_millis(60));
                if !selection.replace(&term) {
                    tracing::info!(
                        replaceable = selection.replaceable,
                        "the selection stays as it was; the term is in Vocabulary"
                    );
                }
            })
            .ok();
    }
}

fn on_key(card: &mut Card, event: &rune::KeyEvent, _: &mut Tx) -> bool {
    if event.key == rune::Key::Escape {
        close(card);
        return true;
    }
    false
}

/// A click into another app closes the card, as Cancel does.
fn on_focus(card: &mut Card, focused: bool, _: &mut Tx) {
    if !focused && card.overlay.is_some() {
        close(card);
    }
}

fn field(
    p: Paint,
    key: &'static str,
    label: &str,
    edit: &TextEdit,
    placeholder: &str,
    access: fn(&mut Card) -> &mut TextEdit,
) -> El {
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
                .on_edit(access)
                .on_submit(|card: &mut Card, _: &mut Tx| add(card))
                .build(p.k)
                .key("input")
                .grow(1.0)
                .shrink(),
        )
}

fn heard_field(card: &mut Card) -> &mut TextEdit {
    &mut card.heard
}

fn term_field(card: &mut Card) -> &mut TextEdit {
    &mut card.term
}

fn view(card: &Card, cx: &mut Cx<Card>) -> El {
    let p = Paint::new(cx.system_dark(), cx.env().scale);
    let c = &p.k.c;
    let note = match (&card.error, &card.selection) {
        (Some(error), _) => Some((tr_fill("Couldn't save: {}", error), p.danger())),
        (None, Some(selection)) if !selection.replaceable => Some((
            tr("This text can't be edited here; the word still goes to Vocabulary").to_string(),
            p.muted(),
        )),
        _ => None,
    };
    let mut panel = col()
        .key("card")
        .role("dialog")
        .label(tr("Add to vocabulary"))
        .w(SIZE.0 - 24.0)
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
        .child(field(
            p,
            "heard",
            tr("Heard"),
            &card.heard,
            tr("What was transcribed"),
            heard_field,
        ))
        .child(field(
            p,
            "term",
            tr("Should be"),
            &card.term,
            tr("The right spelling"),
            term_field,
        ));
    if let Some((note, color)) = note {
        panel = panel.child(text(note).key("note").font(DESCRIPTION).color(color).wrap());
    }
    panel = panel.child(
        row()
            .key("footer")
            .gap(10.0)
            .center_y()
            .child(el().key("spacer").grow(1.0))
            .child(
                Button::new(tr("Cancel"))
                    .variant(ButtonVariant::Neutral)
                    .on_press_tx(|card: &mut Card, _: &mut Tx| close(card))
                    .build_push(p.k)
                    .key("cancel"),
            )
            .child(
                Button::new(tr("Add"))
                    .variant(ButtonVariant::Contrast)
                    .on_press_tx(|card: &mut Card, _: &mut Tx| add(card))
                    .build_push(p.k)
                    .key("add"),
            ),
    );
    for shadow in &p.k.elevation.overlay {
        panel = panel.box_shadow(*shadow);
    }
    let root = col()
        .key("vocabulary-card")
        .w_full()
        .h_full()
        .items_center();
    if card.above {
        root.justify_end().pb(4.0).child(panel)
    } else {
        root.pt(4.0).child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Under the selection, centred and kept on screen; over it when the
    /// screen ends first.
    #[test]
    fn the_card_sits_by_its_selection() {
        let screen = [0.0, 25.0, 1440.0, 875.0];
        let (x, y, above) = place([600.0, 300.0, 80.0, 18.0], screen);
        assert_eq!((x, y, above), (640.0 - 230.0, 324.0, false));
        let (x, _, _) = place([5.0, 300.0, 40.0, 18.0], screen);
        assert_eq!(x, 0.0);
        let (x, _, _) = place([1430.0, 300.0, 10.0, 18.0], screen);
        assert_eq!(x, 1440.0 - 460.0);
        let (_, y, above) = place([600.0, 820.0, 80.0, 18.0], screen);
        assert!(above);
        assert_eq!(y, 820.0 - 6.0 - 210.0);
    }

    #[test]
    fn a_listed_spelling_or_term_finds_its_term() {
        let entries = [VocabularyEntry {
            term: "DeepSeek".into(),
            sounds_like: vec!["дипсик".into(), "дип сик".into()],
        }];
        assert_eq!(known_term(&entries, "Дипсик").as_deref(), Some("DeepSeek"));
        assert_eq!(
            known_term(&entries, "deepseek").as_deref(),
            Some("DeepSeek")
        );
        assert_eq!(known_term(&entries, "кансел"), None);
        assert_eq!(known_term(&entries, "  "), None);
    }
}
