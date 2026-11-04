//! Labs › Cleanup progress (macOS): while AI cleanup runs past a moment,
//! a plate just past the HUD shows how far it is, with Paste now, which
//! pastes the part already cleaned and the rest as transcribed
//! ([`crate::dictation::cleanup_pieces`]). The HUD stays click-through; only
//! this plate takes a click, and it never takes focus, so the paste still
//! lands in the app being dictated into.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::{Cx, El, OverlayHandle, OverlayPlacement, OverlaySpec, ScreenEdge, Tx, row, text};
use rune_kit::{Button, ButtonVariant};

use super::controls::{Paint, PushSize};
use super::live_text::Spot;
use crate::desktop::i18n::tr;
use crate::dictation::cleanup_pieces::{self, Progress};

/// The overlay's size: the plate and room for its border.
pub(crate) const SIZE: (f32, f32) = (420.0, 48.0);
const PLATE_H: f32 = 36.0;
const PAD: f32 = (SIZE.1 - PLATE_H) / 2.0;
const BAR_W: f32 = 150.0;
const BAR_H: f32 = 6.0;

#[derive(Default)]
pub(crate) struct Plate {
    progress: Option<Progress>,
}

/// The overlay's placement for the plate to start at `spot`, where the
/// live text plate would.
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
/// its progress, which it does only with Labs › Cleanup progress on.
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
    cleanup_pieces::watch(move |progress| {
        let handle = overlay.clone();
        let Some(progress) = progress else {
            if shown.swap(false, Ordering::AcqRel) {
                overlay.update(move |plate, tx| {
                    plate.progress = None;
                    tx.instant();
                    handle.hide();
                });
            }
            return;
        };
        let appearing = !shown.swap(true, Ordering::AcqRel);
        overlay.update(move |plate, tx| {
            plate.progress = Some(progress);
            tx.instant();
            if appearing {
                handle.set_placement(Some(placement(place())));
                handle.show();
            }
        });
    });
}

fn view(plate: &Plate, cx: &mut Cx<Plate>) -> El {
    let p = Paint::new(cx.system_dark(), cx.env().scale);
    let root = row().key("cleanup-progress").w_full().h_full().center();
    let Some(progress) = plate.progress else {
        return root;
    };
    root.child(plate_row(p, progress))
}

fn plate_row(p: Paint, progress: Progress) -> El {
    let c = &p.k.c;
    let fill = (BAR_W * progress.fraction).clamp(0.0, BAR_W);
    let bar = row()
        .key("bar")
        .w(BAR_W)
        .h(BAR_H)
        .radius(BAR_H / 2.0)
        .fill(c.layer_03)
        .child(
            row()
                .key("bar-fill")
                .w(fill)
                .h(BAR_H)
                .radius(BAR_H / 2.0)
                .fill(c.icon_accent),
        );
    let mut line = row()
        .key("plate")
        .h(PLATE_H)
        .pl(14.0)
        .pr(6.0)
        .gap(10.0)
        .center_y()
        .radius(PLATE_H / 2.0)
        .fill(c.bg_base)
        .border(p.hair, c.border_base)
        .child(
            text(tr("AI cleanup"))
                .key("label")
                .font(p.k.ty.ui)
                .color(p.text()),
        )
        .child(bar);
    // A count of pieces only means something when there are several.
    if progress.total > 1 {
        line = line.child(
            text(format!(
                "{}/{}",
                progress.done.min(progress.total),
                progress.total
            ))
            .key("count")
            .font(p.k.ty.ui)
            .color(p.muted()),
        );
    }
    line.child(
        Button::new(tr("Paste now"))
            .variant(ButtonVariant::Neutral)
            .on_press_tx(|_: &mut Plate, _: &mut Tx| cleanup_pieces::paste_now())
            .build_small_push(p.k)
            .key("paste-now"),
    )
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
    fn the_plate_fits_its_overlay_with_its_bar_and_paste_now() {
        for dark in [false, true] {
            let progress = Progress {
                done: 3,
                total: 8,
                fraction: 0.5,
            };
            let rt = runtime(Some(progress), dark);
            let [x, _, w, h] = rect(&rt, "plate").expect("the plate draws");
            assert!(x >= 0.0 && x + w <= SIZE.0, "inside: {x} {w}");
            assert_eq!(h, PLATE_H);
            let [button_x, _, button_w, _] = rect(&rt, "paste-now").unwrap();
            assert!(button_x + button_w <= x + w, "Paste now fits the plate");
            let [_, _, fill, _] = rect(&rt, "bar-fill").unwrap();
            assert!((fill - BAR_W / 2.0).abs() < 0.5, "half the bar: {fill}");
            assert!(rect(&rt, "count").is_some());
        }
    }

    #[test]
    fn one_piece_shows_no_count_and_no_progress_shows_nothing() {
        let one = Progress {
            done: 0,
            total: 1,
            fraction: 0.3,
        };
        assert!(rect(&runtime(Some(one), false), "count").is_none());
        assert!(rect(&runtime(None, false), "plate").is_none());
    }

    /// Renders the plate to `HEX_CLEANUP_PLATE_SNAPS/cleanup-plate-*.png`
    /// in both appearances, to look at.
    #[test]
    #[ignore = "set HEX_CLEANUP_PLATE_SNAPS to a folder; needs a GPU"]
    fn snapshots() {
        let folder = std::path::PathBuf::from(std::env::var("HEX_CLEANUP_PLATE_SNAPS").unwrap());
        let gpu = rune::render::Gpu::try_new(rune::render::Gpu::instance(), None).unwrap();
        for (name, done, total, fraction) in [("one", 0, 1, 0.35), ("pieces", 3, 8, 0.44)] {
            for dark in [false, true] {
                let progress = Progress {
                    done,
                    total,
                    fraction,
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
