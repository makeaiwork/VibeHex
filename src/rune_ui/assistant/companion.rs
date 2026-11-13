//! A switch's companion in its row (`changes::companion`): "Off → On,
//! after [15 min ▾]". The choice is Assistant's value chip with a
//! chevron, outlined while it is the value saved now and on the new
//! value's soft tint once another is picked; it opens the kit's menu of
//! its choices. Once the row is decided it is a plain chip.

use rune::{El, icon, row, text};
use rune_kit::{Menu as KitMenu, MenuItem, Placement};
use rune_theme_opencode::icons as theme;

use super::super::controls::Paint;
use super::super::{Menu, Ui};
use super::changes::{self as act, At};
use super::value::{self, Slot, ValueAt};
use crate::assistant::changes::companion::{self, Companion};
use crate::assistant::changes::names;
use crate::desktop::i18n::tr_window;

/// The companion of row `index` (`kept`: of "Already set"), with the
/// words around it; `open` while the row can still change.
#[allow(clippy::too_many_arguments)]
pub(super) fn view(
    ui: &Ui,
    p: Paint,
    at: At,
    index: usize,
    kept: bool,
    companion: &Companion,
    open: bool,
    layer: &mut Vec<El>,
) -> Option<El> {
    let pair = companion.pair()?;
    let (before, after) = companion::around(pair);
    let label = companion::label(&companion.key, companion.to());
    let words = |key: &str, words: String| {
        (!words.is_empty()).then(|| {
            text(words)
                .key(key.to_owned())
                .font(p.k.ty.small)
                .color(p.muted())
        })
    };
    let chip = match open {
        true => choice(
            ui,
            p,
            at.value(index, Slot::Companion(kept)),
            companion,
            &label,
            layer,
        ),
        false => value::short(p, "companion", &label, companion.changed(), false),
    };
    Some(
        row()
            .key("companion")
            .gap(5.0)
            .center_y()
            .children(words("before", before))
            .child(chip)
            .children(words("after", after)),
    )
}

/// The chip that opens the companion's choices.
fn choice(
    ui: &Ui,
    p: Paint,
    at: ValueAt,
    companion: &Companion,
    label: &str,
    layer: &mut Vec<El>,
) -> El {
    let c = &p.k.c;
    let menu = Menu::Companion(at);
    let open = ui.open == Some(menu);
    let (fill, border, ink) = value::looks(p, companion.changed(), false);
    let name = names::place(&companion.key, &ui.settings)
        .map(|place| tr_window(&place.name).to_owned())
        .unwrap_or_else(|| companion.key.clone());
    let trigger = row()
        .key("choice")
        .role("button")
        .label(name)
        .a11y_value(label.to_owned())
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .h(20.0)
        .pl(7.0)
        .pr(4.0)
        .gap(3.0)
        .center_y()
        .radius(p.k.radius.xs)
        .fill(fill)
        .border(p.hair, border)
        .hover(c.hover)
        .child(
            text(label.to_owned())
                .key("label")
                .font(p.k.ty.small)
                .color(ink)
                .max_lines(1),
        )
        .child(icon(&theme::CHEVRON_DOWN, 12.0).key("chevron").color(ink))
        .on_press(move |ui: &mut Ui| {
            ui.open = (ui.open != Some(menu)).then_some(menu);
        });
    let mut list = KitMenu::new(format!("companion{}-{}#choices", at.at.turn, at.row))
        .trigger(trigger)
        .placement(Placement::BottomStart)
        .gutter(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None);
    let (row, kept) = match at.slot {
        Slot::Companion(kept) => (at.row, kept),
        _ => (at.row, false),
    };
    for (index, option) in companion.options.iter().enumerate() {
        let card = at.at;
        list = list.item(
            MenuItem::new(companion::label(&companion.key, option))
                .checked(index == companion.chosen)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    act::pick_companion(ui, card, row, kept, index);
                }),
        );
    }
    list.build(p.k, layer)
}
