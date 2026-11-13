//! Cleanup on this computer in a card or a plan row
//! (`changes::built_in`): "Runs on Your server → Built in" with the
//! built-in models under it to pick from, each as Output › AI cleanup
//! shows it (its speed and quality meters), its download, and what it
//! takes while it runs. The recommended one and one already downloaded
//! are tagged; one too big for this computer says so in the warning tone.
//! The picked option wears a soft tint with its radio, never a side bar.
//! Before Apply the row says "Needs a download 2.5 GB"; after it, the
//! model downloads in the row with its progress and Cancel.

use rune::{El, col, row, text};
use rune_kit::RadioItem;

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint};
use super::super::lists::{Tone, apart, tone_tag};
use super::super::output::local_cleanup;
use super::card::{Look, push, warned};
use super::changes::{self as act, At};
use crate::assistant::changes::Item;
use crate::assistant::changes::built_in::{self, Machine};
use crate::desktop::i18n::{tr, tr_fill, tr_named};
use crate::dictation::cleanup_server::local_models::{LocalModel, MODELS, gigabytes};

/// What this computer has: a scene's stand-ins, else read now.
pub(super) fn machine(ui: &Ui) -> Machine {
    match ui.assistant.fixture {
        true => ui.assistant.machine.clone(),
        false => Machine::here(),
    }
}

/// The model `item` brings that isn't on disk yet, with its size: what
/// "Needs a download" names before Apply.
pub(super) fn download_size(ui: &Ui, item: &Item) -> Option<String> {
    let model = built_in::target(item, &ui.settings)?;
    (!machine(ui).has(model)).then(|| model.size_label())
}

/// "Needs a download 2.5 GB": what a row takes before Apply.
pub(super) fn needs_download(p: Paint, size: String) -> El {
    row()
        .key("download")
        .gap(6.0)
        .center_y()
        .child(
            text(tr("Needs a download"))
                .key("words")
                .font(DESCRIPTION)
                .color(p.muted()),
        )
        .child(text(size).key("size").font(DESCRIPTION).color(p.warning()))
}

/// A card's lines before Apply: the download it takes, and the models.
pub(super) fn proposed(ui: &Ui, p: Paint, at: At, index: usize, item: &Item) -> Vec<El> {
    let mut lines: Vec<El> = download_size(ui, item)
        .map(|size| needs_download(p, size))
        .into_iter()
        .collect();
    lines.extend(choices(ui, p, at, index, item));
    lines
}

/// Whether the words say Mac: a scene plays one.
fn mac(ui: &Ui) -> bool {
    super::ask::os(ui) == "macos"
}

/// The models to pick from, under the row's words while it is open.
pub(super) fn choices(ui: &Ui, p: Paint, at: At, index: usize, item: &Item) -> Option<El> {
    let companion = built_in::choice(item)?;
    let machine = machine(ui);
    let mut list = col()
        .key("models")
        .role("radiogroup")
        .label(tr("Model"))
        .gap(2.0)
        .mt(4.0);
    for (option, model) in MODELS.iter().enumerate() {
        let on = option == companion.chosen;
        list = list.child(choice(ui, p, (at, index), (option, model), on, &machine));
    }
    if built_in::none_fits(&machine) {
        let words = match mac(ui) {
            true => tr(
                "None of these fits this Mac's memory well: AI cleanup with Codex suits it better",
            ),
            false => tr(
                "None of these fits this computer's memory well: AI cleanup with Codex suits it better",
            ),
        };
        list = list.child(warned(p, words).key("none-fits").mt(4.0));
    }
    Some(list)
}

/// One model: its radio and name, its tags, its meters, what it costs,
/// and a warning when it is too much for this computer.
fn choice(
    ui: &Ui,
    p: Paint,
    (at, row_index): (At, usize),
    (option, model): (usize, &'static LocalModel),
    on: bool,
    machine: &Machine,
) -> El {
    let c = &p.k.c;
    let fit = built_in::fit(model, machine);
    let tags = row()
        .key("tags")
        .gap(6.0)
        .center_y()
        .flex_wrap()
        .children(
            (option == 0).then(|| tone_tag(p, "recommended", tr("Recommended"), Tone::Accent)),
        )
        .children(
            machine
                .has(model)
                .then(|| tone_tag(p, "downloaded", tr("Downloaded"), Tone::Success)),
        );
    let head = row()
        .key("head")
        .gap(8.0)
        .center_y()
        .flex_wrap()
        .child(
            RadioItem::new(model.name, on)
                .on_select(move |ui: &mut Ui| act::pick_companion(ui, at, row_index, false, option))
                .build(p.k),
        )
        .child(tags);
    let meters = row()
        .key("meters")
        .gap(16.0)
        .center_y()
        .flex_wrap()
        .child(local_cleanup::meter(
            p,
            "speed",
            tr("Speed"),
            model.speed,
            local_cleanup::speed_label(model.speed),
            p.k.c.success.fg,
        ))
        .child(local_cleanup::meter(
            p,
            "quality",
            tr("Quality"),
            model.quality,
            local_cleanup::quality_label(model.quality),
            p.k.c.info.fg,
        ));
    let download = match machine.has(model) {
        true => String::new(),
        false => tr_fill("{} download", &model.size_label()),
    };
    let memory = tr_fill(
        "about {} of memory while it runs",
        &gigabytes(model.memory()),
    );
    let costs = text(apart(&[download, memory]))
        .key("costs")
        .font(DESCRIPTION)
        .color(p.muted())
        .wrap();
    let mut details = col()
        .key("details")
        .pl(24.0)
        .gap(3.0)
        .child(meters)
        .child(costs);
    if let Some((needs, total)) = fit.memory {
        let (need, total) = (gigabytes(needs), memory_label(total));
        let values = [("need", need.as_str()), ("total", total.as_str())];
        let words = match mac(ui) {
            true => tr_named(
                "May slow this Mac: needs about {need} of {total} memory",
                &values,
            ),
            false => tr_named(
                "May slow this computer: needs about {need} of {total} memory",
                &values,
            ),
        };
        details = details.child(warned(p, &words).key("slow"));
    }
    if let Some((needs, free)) = fit.disk {
        let words = tr_named(
            "Not enough disk space: needs {need}, {free} free",
            &[("need", &gigabytes(needs)), ("free", &gigabytes(free))],
        );
        details = details.child(warned(p, &words).key("disk"));
    }
    let mut line = col()
        .key(format!("model{option}"))
        .px(8.0)
        .py(6.0)
        .gap(4.0)
        .radius(p.k.radius.sm)
        .child(head)
        .child(details);
    line = match on {
        true => line
            .fill(p.k.alpha(p.accent(), 0.08))
            .border(p.hair, p.k.alpha(p.accent(), 0.32)),
        false => line.hover(c.hover),
    };
    line
}

/// The memory installed as the computer's maker counts it: "8 GB", "16 GB".
fn memory_label(bytes: u64) -> String {
    let gib = bytes as f64 / (1u64 << 30) as f64;
    match gib >= 10.0 || (gib - gib.round()).abs() < 0.05 {
        true => format!("{} GB", gib.round() as u64),
        false => format!("{gib:.1} GB"),
    }
}

/// A saved row's model downloading: its progress, how far, and Cancel.
pub(super) fn downloading(
    p: Paint,
    at: At,
    model: &'static LocalModel,
    done: u64,
    total: u64,
) -> Vec<El> {
    let fraction = match total {
        0 => 0.0,
        total => (done as f32 / total as f32).clamp(0.0, 1.0),
    };
    let words = tr_named(
        "Downloading {model}, {done} of {total}. AI cleanup uses it once it is in",
        &[
            ("model", model.name),
            ("done", &gigabytes(done)),
            ("total", &gigabytes(total)),
        ],
    );
    vec![
        row()
            .key("progress")
            .pt(4.0)
            .child(rune_kit::Progress::new(Some(fraction)).build(p.k)),
        row()
            .key("downloading")
            .gap(8.0)
            .center_y()
            .child(
                text(words)
                    .key("words")
                    .font(DESCRIPTION)
                    .color(p.muted())
                    .wrap_anywhere()
                    .grow(1.0)
                    .shrink()
                    .min_w(0.0),
            )
            .child(push(
                p,
                "cancel",
                tr("Cancel"),
                Look::Ghost,
                move |ui, _| act::cancel_download(ui, at),
            )),
    ]
}

/// A saved row's model that couldn't download: why, and Try again.
pub(super) fn failed(ui: &Ui, p: Paint, item: &Item) -> Option<El> {
    let model = built_in::target(item, &ui.settings)?;
    let why = local_cleanup::failed(ui, model.id)?;
    if machine(ui).has(model) {
        return None;
    }
    Some(
        row()
            .key("download-failed")
            .gap(8.0)
            .center_y()
            .child(
                text(why)
                    .key("why")
                    .font(DESCRIPTION)
                    .color(p.danger())
                    .wrap_anywhere()
                    .grow(1.0)
                    .shrink()
                    .min_w(0.0),
            )
            .child(push(
                p,
                "retry-download",
                tr("Try again"),
                Look::Secondary,
                move |ui, _| local_cleanup::start_download(ui, model),
            )),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_reads_as_its_maker_counts_it() {
        assert_eq!(memory_label(8 << 30), "8 GB");
        assert_eq!(memory_label(48 << 30), "48 GB");
        assert_eq!(memory_label(16_303_488 * 1024), "16 GB");
        assert_eq!(memory_label(3 << 29), "1.5 GB");
    }
}
