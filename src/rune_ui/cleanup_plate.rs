//! Labs › Cleanup progress (macOS): while an AI cleanup will plainly take a
//! while, the HUD gives way to this plate in its colours, which shows how far
//! the cleanup is with Paste now: the part already cleaned, then the rest as
//! transcribed ([`crate::dictation::cleanup_pieces`]). Only this plate takes
//! a click, and it never takes focus, so the paste still lands in the app
//! being dictated into.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::{Cx, El, OverlayHandle, OverlayPlacement, OverlaySpec, ScreenEdge, col, row, text};

use super::controls::Paint;
use super::live_text::Spot;
use crate::desktop::i18n::tr;
use crate::dictation::cleanup_pieces::{self, Progress};

/// The overlay's size: the plate and room for its border.
pub(crate) const SIZE: (f32, f32) = (380.0, 44.0);
const PLATE_H: f32 = 30.0;
const PAD: f32 = (SIZE.1 - PLATE_H) / 2.0;
/// The dot field, as the HUD's Dot scope draws it: lit up to the progress.
const COLUMNS: usize = 40;
const ROWS: usize = 9;
const DOT: f32 = 1.6;
const DOT_GAP: f32 = 1.0;
const BUTTON_H: f32 = 22.0;

#[derive(Default)]
pub(crate) struct Plate {
    progress: Option<Progress>,
}

/// Where the plate goes: on the HUD's own spot while the HUD gives way to
/// it; just past the HUD's plate (`spot`) while the HUD shows, such as for a
/// new dictation, or when the HUD sits in a notch island or is hidden.
fn placement_now(spot: Spot) -> (bool, OverlayPlacement) {
    match crate::rune_hud::hud_center().filter(|_| !crate::rune_hud::hud_shown()) {
        Some((x, y)) => (
            true,
            OverlayPlacement::Rect {
                x: x - SIZE.0 / 2.0,
                y: y - SIZE.1 / 2.0,
                width: SIZE.0,
                height: SIZE.1,
            },
        ),
        None => (false, placement(spot)),
    }
}

/// The overlay's placement for the plate to start at `spot`, just past
/// where the HUD's plate sits.
fn placement(spot: Spot) -> OverlayPlacement {
    match spot {
        Spot::Edge(edge, margin) => OverlayPlacement::Edge {
            edge,
            margin: margin - PAD,
            align: rune::Align::Center,
            inset: 0.0,
        },
        Spot::Global { x, y, below } => OverlayPlacement::Rect {
            x: x - SIZE.0 / 2.0,
            y: if below { y - PAD } else { y - SIZE.1 + PAD },
            width: SIZE.0,
            height: SIZE.1,
        },
    }
}

/// Adds the plate to the app. It shows only while a long cleanup reports
/// its progress, which it does only with Labs › Cleanup progress on, and
/// hides the HUD meanwhile.
pub(crate) fn install<S: 'static>(app: &mut rune::App<S>, place: super::live_text::Place) {
    let spec = OverlaySpec {
        name: "Hex cleanup progress",
        size: SIZE,
        // Placed for real each time it appears.
        edge: ScreenEdge::Top,
        margin: 0.0,
        align: rune::Align::Center,
        inset: 0.0,
        interactive: true,
        view,
    };
    let overlay: OverlayHandle<Plate> = app.overlay(spec, Plate::default());
    let shown = Arc::new(AtomicBool::new(false));
    // Whether the plate sits on the HUD's spot, as last placed.
    let on_hud = Arc::new(std::sync::Mutex::new(None::<bool>));
    cleanup_pieces::watch(move |progress| {
        let handle = overlay.clone();
        let Some(progress) = progress else {
            if shown.swap(false, Ordering::AcqRel) {
                *on_hud.lock().unwrap_or_else(|error| error.into_inner()) = None;
                crate::rune_hud::give_way(false);
                overlay.update(move |plate, tx| {
                    plate.progress = None;
                    tx.instant();
                    handle.hide();
                });
            }
            return;
        };
        let appearing = !shown.swap(true, Ordering::AcqRel);
        if appearing {
            crate::rune_hud::give_way(true);
        }
        let on_hud = on_hud.clone();
        // On the main thread, after the HUD's own updates: moves to the HUD's
        // spot once it hides and below it while it shows.
        overlay.update(move |plate, tx| {
            plate.progress = Some(progress);
            tx.instant();
            let (at_hud, placement) = placement_now(place());
            let mut last = on_hud.lock().unwrap_or_else(|error| error.into_inner());
            if appearing || *last != Some(at_hud) {
                *last = Some(at_hud);
                handle.set_placement(Some(placement));
            }
            if appearing {
                handle.show();
            }
        });
    });
}

fn view(plate: &Plate, cx: &mut Cx<Plate>) -> El {
    // The HUD's colours in either appearance: dark, with light dots.
    let p = Paint::new(true, cx.env().scale);
    let root = row().key("cleanup-progress").w_full().h_full().center();
    let Some(progress) = plate.progress else {
        return root;
    };
    root.child(plate_row(p, progress))
}

/// The dot field lit from the left up to `fraction`, brightest along its
/// middle rows and at the leading column, as Dot scope's trace is.
fn dots(p: Paint, fraction: f32) -> El {
    let ink = p.k.c.text_base;
    let lit = (COLUMNS as f32 * fraction.clamp(0.0, 1.0)).round() as usize;
    let middle = (ROWS as f32 - 1.0) / 2.0;
    let rows = (0..ROWS).map(|r| {
        let centre = 1.0 - (r as f32 - middle).abs() / (middle + 1.0);
        row().gap(DOT_GAP).children((0..COLUMNS).map(|c| {
            let alpha = if c + 1 == lit {
                1.0
            } else if c < lit {
                0.35 + 0.55 * centre
            } else {
                0.10
            };
            rune::el()
                .size(DOT, DOT)
                .radius(DOT / 2.0)
                .fill(ink.alpha(alpha))
        }))
    });
    col().key("dots").gap(DOT_GAP).children(rows)
}

fn plate_row(p: Paint, progress: Progress) -> El {
    let c = &p.k.c;
    let percent = (progress.fraction.clamp(0.0, 1.0) * 100.0).floor() as u32;
    let paste_now = row()
        .key("paste-now")
        .role("button")
        .label(tr("Paste now"))
        .h(BUTTON_H)
        .px(10.0)
        .center_y()
        .radius(BUTTON_H / 2.0)
        .fill(c.text_base.alpha(0.12))
        .hover(c.text_base.alpha(0.2))
        .on_press(|_: &mut Plate| cleanup_pieces::paste_now())
        .child(
            text(tr("Paste now"))
                .font(p.k.ty.ui_medium)
                .color(c.text_base.alpha(0.9)),
        );
    row()
        .key("plate")
        .h(PLATE_H)
        .pl(14.0)
        .pr(4.0)
        .gap(10.0)
        .center_y()
        .radius(PLATE_H / 2.0)
        .fill(c.bg_base)
        .border(p.hair, c.text_base.alpha(0.12))
        .child(
            text(if progress.loading {
                tr("Loading the model…")
            } else {
                tr("Cleaning up")
            })
            .key("label")
            .font(p.k.ty.ui)
            .color(c.text_base.alpha(0.9)),
        )
        .child(dots(p, progress.fraction))
        .child(
            text(format!("{percent}%"))
                .key("percent")
                .font(p.k.ty.ui)
                .color(c.text_base.alpha(0.5)),
        )
        .child(paste_now)
}

#[cfg(test)]
mod tests {
    use rune::Runtime;
    use rune::text::TextSystem;

    use super::*;

    fn runtime(progress: Option<Progress>, dark: bool) -> Runtime<Plate> {
        let mut rt = crate::rune_ui::offscreen::runtime(
            Plate { progress },
            view,
            SIZE,
            rune::Env {
                system_dark: dark,
                scale: 2.0,
            },
            TextSystem::new(rune_theme_opencode::FONTS, rune_theme_opencode::FAMILIES),
        );
        rt.settle_all();
        rt.frame(0.0);
        rt
    }

    fn rect(rt: &Runtime<Plate>, key: &str) -> Option<[f32; 4]> {
        rt.elements()
            .into_iter()
            .find(|e| !e.exiting && e.key.as_deref() == Some(key))
            .map(|e| e.rect)
    }

    #[test]
    fn the_plate_fits_its_overlay_with_its_dots_and_paste_now() {
        for dark in [false, true] {
            let progress = Progress {
                done: 3,
                total: 8,
                fraction: 0.5,
                loading: false,
            };
            let rt = runtime(Some(progress), dark);
            let [x, _, w, h] = rect(&rt, "plate").expect("the plate draws");
            assert!(x >= 0.0 && x + w <= SIZE.0, "inside: {x} {w}");
            assert_eq!(h, PLATE_H);
            let [button_x, _, button_w, _] = rect(&rt, "paste-now").unwrap();
            assert!(button_x + button_w <= x + w, "Paste now fits the plate");
            assert!(rect(&rt, "dots").is_some());
            assert!(rect(&rt, "percent").is_some());
        }
    }

    #[test]
    fn no_progress_shows_nothing() {
        assert!(rect(&runtime(None, false), "plate").is_none());
    }

    /// Renders the plate to `HEX_CLEANUP_PLATE_SNAPS/cleanup-plate-*.png`
    /// in both appearances, to look at.
    #[test]
    #[ignore = "set HEX_CLEANUP_PLATE_SNAPS to a folder; needs a GPU"]
    fn snapshots() {
        let folder = std::path::PathBuf::from(std::env::var("HEX_CLEANUP_PLATE_SNAPS").unwrap());
        let gpu = rune::render::Gpu::try_new(rune::render::Gpu::instance(), None).unwrap();
        for (name, done, total, fraction, loading) in
            [("loading", 0, 1, 0.0, true), ("pieces", 3, 8, 0.44, false)]
        {
            for dark in [false, true] {
                let progress = Progress {
                    done,
                    total,
                    fraction,
                    loading,
                };
                let mut rt = runtime(Some(progress), dark);
                let (w, h) = ((SIZE.0 * 2.0) as u32, (SIZE.1 * 2.0) as u32);
                let mut renderer = crate::rune_ui::offscreen::renderer(&gpu.device);
                let target = crate::rune_ui::offscreen::target(&gpu.device, w, h);
                renderer.upload_images(&gpu.device, &gpu.queue, &mut rt.images);
                renderer.prepare(
                    &gpu.device,
                    &gpu.queue,
                    &mut rt.text.atlas,
                    &rt.instances,
                    (w, h),
                );
                let mut encoder = gpu.device.create_command_encoder(&Default::default());
                let backdrop = if dark { 0.08 } else { 0.62 };
                renderer.draw(
                    &mut encoder,
                    &target.view,
                    [backdrop, backdrop, backdrop, 1.0],
                );
                target.copy(&mut encoder);
                gpu.queue.submit([encoder.finish()]);
                let theme = if dark { "dark" } else { "light" };
                let path = folder.join(format!("cleanup-plate-{name}-{theme}.png"));
                rune::render::write_png(&path, w, h, &target.read(&gpu.device)).unwrap();
            }
        }
    }
}
