//! The card's actions: Copy, the … menu, Transcribe again with its language,
//! Clean up again, and the request area with its chips and Recent.

use rune::{El, TextEdit, Tx, col, icon, row, text};
use rune_kit::{
    ButtonVariant, IconButton, IconButtonVariant, Menu as KitMenu, MenuItem, Placement,
};

use super::super::controls::Paint;
use super::super::list_detail as ld;
use super::super::{Menu, Pane, Ui, icons};
use super::detail::{current, tab_label, texts};
use super::{Again, Armed, Tab, submit};
use crate::desktop::i18n::tr;
use crate::files::store::{self, FileRecord, Version};
use crate::files::worker::Job;
use crate::transcription::models::TranscriptionModelId;

static MORE_ICON: rune::Icon = rune::Icon {
    name: "hex-files-more",
    view: 16.0,
    body: r#"<circle cx="3.5" cy="8" r="1.25" fill="currentColor"/><circle cx="8" cy="8" r="1.25" fill="currentColor"/><circle cx="12.5" cy="8" r="1.25" fill="currentColor"/>"#,
};

static SAVE_ICON: rune::Icon = rune::Icon {
    name: "hex-files-save",
    view: 16.0,
    body: r#"<path d="M8 10.5V2.5M5 7.5l3 3 3-3M2.5 10v3.5h11V10" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>"#,
};

pub(super) fn menu_open(ui: &Ui, key: &'static str) -> bool {
    ui.open == Some(Menu::Pick(key))
}

pub(super) fn toggle(ui: &mut Ui, key: &'static str) {
    ui.open = (ui.open != Some(Menu::Pick(key))).then_some(Menu::Pick(key));
}

/// A quiet small button that opens a menu, as the mockups draw it: an
/// optional icon, the label, and a chevron that turns up while it is open.
#[allow(clippy::too_many_arguments)]
fn menu_button(
    p: Paint,
    key: &str,
    glyph: Option<&'static rune::Icon>,
    label: &str,
    open: bool,
    disabled: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    let chevron = if open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    quiet_button(p, key, glyph, label, Some(chevron), disabled, press)
}

/// The mockups' small button: a step above the card, a visible hairline,
/// an optional icon before the label and chevron after it.
pub(super) fn quiet_button(
    p: Paint,
    key: &str,
    glyph: Option<&'static rune::Icon>,
    label: &str,
    chevron: Option<&'static rune::Icon>,
    disabled: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    quiet(p, key, glyph, (label, true), chevron, disabled, press)
}

/// The small button with or without its words (`label.1`); without them
/// the label is only its accessible name, and a tooltip says it.
fn quiet(
    p: Paint,
    key: &str,
    glyph: Option<&'static rune::Icon>,
    (label, words): (&str, bool),
    chevron: Option<&'static rune::Icon>,
    disabled: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    let c = &p.k.c;
    let mut button = row()
        .key(key)
        .role("button")
        .label(label)
        .h(22.0)
        .pl(8.0)
        .pr(if chevron.is_some() { 6.0 } else { 8.0 })
        .gap(5.0)
        .center_y()
        .radius(5.0)
        .fill(c.layer_01)
        .box_shadow(rune::Shadow {
            spread: 0.5,
            color: c.border_strong,
            ..Default::default()
        })
        .children(glyph.map(|glyph| icon(glyph, 12.0).key("icon").color(c.icon_base)))
        .children(words.then(|| {
            text(label.to_string())
                .key("words")
                .font(p.k.ty.small_medium)
                .color(c.text_base)
        }))
        .children(chevron.map(|glyph| icon(glyph, 11.0).key("chevron").color(c.icon_muted)));
    if disabled {
        button = button.dim(0.45);
    } else {
        button = button
            .focusable(true)
            .focus_ring(2.0, 0.0, c.border_focus)
            .hover(c.hover)
            .press(c.pressed)
            .on_press(press);
    }
    button
}

/// A quick request above the field: a lightly filled pill in the body's
/// weight, with a chevron when it opens a menu.
pub(in super::super) fn chip_button(
    p: Paint,
    key: &str,
    label: &str,
    chevron: Option<&'static rune::Icon>,
    live: bool,
    press: impl Fn(&mut Ui) + 'static,
) -> El {
    let c = &p.k.c;
    // An opaque fill a step off the card, as the mockups draw it (#1f1f1f
    // on dark), so text scrolling under a chip never shows through it.
    // Dark needs less: the same steps read brighter on a dark card.
    let (fill, outline, ink) = if p.k.dark {
        (
            c.bg_base.mix(c.text_base, 0.012).alpha(1.0),
            c.border_base.mix(c.border_strong, 0.5),
            c.text_base.mix(c.text_weak, 0.45),
        )
    } else {
        (
            c.bg_base.mix(c.text_base, 0.04).alpha(1.0),
            c.border_strong,
            c.text_base.mix(c.text_weak, 0.35),
        )
    };
    let chip = row()
        .key(key)
        .role("button")
        .label(label)
        .h(24.0)
        .pl(9.0)
        .pr(if chevron.is_some() { 7.0 } else { 9.0 })
        .gap(4.0)
        .center_y()
        .radius(12.0)
        .fill(fill)
        .box_shadow(rune::Shadow {
            spread: 0.5,
            color: outline,
            ..Default::default()
        })
        .child(
            text(label.to_string())
                .key("label")
                .font(rune::TextStyle {
                    size: 12.0,
                    ..p.k.ty.ui
                })
                .color(ink),
        )
        .children(chevron.map(|glyph| icon(glyph, 12.0).key("chevron").color(c.icon_muted)));
    if !live {
        return chip;
    }
    chip.focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .hover(c.hover)
        .press(c.pressed)
        .on_press(press)
}

/// Copy as Markdown or as plain text.
pub(super) fn copy_menu(ui: &Ui, p: Paint, version: &Version, layer: &mut Vec<El>) -> El {
    let (markdown, plain) = texts(ui, version);
    let copied = |what| ui.files.copied == Some((version.id, what));
    let id = version.id;
    let item = |label: &str, what: &'static str, copy: String| {
        MenuItem::new(if copied(what) { tr("Copied") } else { label }).on_select_tx(
            move |ui: &mut Ui, tx: &mut Tx| {
                tx.copy(copy.clone());
                ui.files.copied = Some((id, what));
                ui.open = Some(Menu::Pick("files-copy"));
            },
        )
    };
    let trigger = IconButton::new(ld::COPY_ICON)
        .variant(IconButtonVariant::Ghost)
        .control_size(rune_kit::ControlSize::Small)
        .label(tr("Copy"))
        .on_press(|ui: &mut Ui| toggle(ui, "files-copy"))
        .build(p.k)
        .key("file-copy");
    KitMenu::new("files-copy-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(200.0)
        .open(menu_open(ui, "files-copy"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .item(item(tr("Copy as Markdown"), "markdown", markdown))
        .item(item(tr("Copy as plain text"), "plain", plain))
        .trigger(trigger)
        .build(p.k, layer)
}

/// Where the file is, from its folder: `…/Clients/Interview.m4a`, a long
/// name shortened in its middle so the extension stays.
fn path_tail(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let chars: Vec<char> = name.chars().collect();
    let name = if chars.len() > 32 {
        let head: String = chars[..18].iter().collect();
        let tail: String = chars[chars.len() - 12..].iter().collect();
        format!("{head}…{tail}")
    } else {
        name
    };
    match path.parent().and_then(|parent| parent.file_name()) {
        Some(folder) => format!("…/{}/{name}", folder.to_string_lossy()),
        None => name,
    }
}

/// Show in Finder and Open original (with the app that opens it), Save as…,
/// the two deletes, and where the file is.
pub(super) fn more_menu(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    version: Option<&Version>,
    layer: &mut Vec<El>,
) -> El {
    let found = ui.files.fixture || record.path.exists();
    let (path, shown) = (record.path.clone(), record.path.clone());
    let mut open_original = MenuItem::new(tr("Open original"))
        .icon(&icons::PLAY)
        .disabled(!found)
        .on_select(move |ui: &mut Ui| {
            ui.open = None;
            if !ui.files.fixture {
                super::open_file(&shown);
            }
        });
    if let Some(opener) = ui.files.opener.as_ref().filter(|_| found) {
        open_original = open_original.shortcut(opener.clone());
    }
    let mut menu = KitMenu::new("files-more-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(240.0)
        .open(menu_open(ui, "files-more"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .item(
            MenuItem::new(tr("Show in Finder"))
                .icon(&icons::FOLDER)
                .disabled(!found)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    if !ui.files.fixture {
                        super::reveal(&path);
                    }
                }),
        )
        .item(open_original);
    if let Some(version) = version {
        menu = menu.separator().item(save_as(ui, record, version));
    }
    menu = menu.separator();
    if let Some(version) = version
        && record.versions.first().map(|first| first.id) != Some(version.id)
    {
        let armed = Armed::Version(record.id.clone(), version.id);
        menu = menu.item(
            MenuItem::new(tr("Delete this version…"))
                .icon(ld::TRASH_ICON)
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    ui.files.armed = Some(armed.clone());
                }),
        );
    }
    let armed = Armed::File(record.id.clone());
    menu = menu
        .item(
            MenuItem::new(tr("Delete file from list…"))
                .lead(icon(&icons::CIRCLE_X, 16.0).color(p.danger()))
                .on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    ui.files.armed = Some(armed.clone());
                }),
        )
        .separator()
        .group_label(if found {
            path_tail(&record.path)
        } else {
            tr("File not found").into()
        });
    let opened = record.path.clone();
    let trigger = IconButton::new(&MORE_ICON)
        .variant(IconButtonVariant::Ghost)
        .control_size(rune_kit::ControlSize::Small)
        .label(tr("More"))
        .on_press(move |ui: &mut Ui| {
            ui.files.opener = if ui.files.fixture {
                Some("QuickTime Player".into())
            } else {
                super::opener(&opened)
            };
            toggle(ui, "files-more");
        })
        .build(p.k)
        .key("file-more");
    menu.trigger(trigger).build(p.k, layer)
}

/// Save as… a Markdown or plain-text file beside the original.
fn save_as(ui: &Ui, record: &FileRecord, version: &Version) -> MenuItem {
    let (markdown, plain) = texts(ui, version);
    let stem = record
        .path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| record.name.clone());
    let file_name = format!("{stem} — {}.md", tab_label(&version.name));
    let directory = record.path.parent().map(std::path::Path::to_path_buf);
    MenuItem::new(tr("Save as…"))
        .icon(&SAVE_ICON)
        .on_select_tx(move |ui: &mut Ui, tx: &mut Tx| {
            ui.open = None;
            let options = rune::FileDialogOptions {
                title: tr("Save the text").into(),
                directory: directory.clone(),
                file_name: Some(file_name.clone()),
                ..Default::default()
            }
            .filter("Markdown", &["md"])
            .filter(tr("Plain text"), &["txt"]);
            let (markdown, plain) = (markdown.clone(), plain.clone());
            tx.save_file(options, move |ui: &mut Ui, chosen, _| match chosen {
                Ok(Some(path)) => {
                    let plain_text = path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"));
                    let body = if plain_text { &plain } else { &markdown };
                    if let Err(error) = std::fs::write(&path, body) {
                        ui.files.error = Some(error.to_string());
                    }
                }
                Ok(None) => {}
                Err(error) => ui.files.error = Some(error.to_string()),
            });
        })
}

/// Transcribe again with another downloaded model, as a new Original.
pub(super) fn again_menu(ui: &Ui, p: Paint, record: &FileRecord, layer: &mut Vec<El>) -> El {
    let gone = !ui.files.fixture && !record.path.is_file();
    let open = !gone && menu_open(ui, "files-again");
    let same = record.versions.first().map(|first| first.model.as_str());
    let mut menu = KitMenu::new("files-again-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(260.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .group_label(tr("Transcribe the audio again with"));
    for &(model, name) in &ui.files.models {
        let id = record.id.clone();
        let mut item =
            MenuItem::new(name).on_select(move |ui: &mut Ui| choose_model(ui, &id, model, name));
        if same == Some(name) {
            item = item.badge(tr("Same model"));
        }
        menu = menu.item(item);
    }
    menu = menu
        .separator()
        .item(MenuItem::new(tr("Adds a version to Original")).disabled(true))
        .item(
            MenuItem::new(tr("More models in General")).on_select(|ui: &mut Ui| {
                ui.open = None;
                ui.pane = Pane::General;
            }),
        );
    let label = if gone {
        tr("The file is gone")
    } else {
        tr("Transcribe again")
    };
    // Just the icon and chevron, so the Original's line keeps to one row;
    // the tooltip says what it does.
    let chevron = if open {
        p.k.glyph.chevron_up
    } else {
        p.k.glyph.chevron_down
    };
    let trigger = quiet(
        p,
        "file-again",
        Some(&icons::REFRESH),
        (label, false),
        Some(chevron),
        gone,
        |ui: &mut Ui| {
            ui.files.models = super::super::history::installed_models(ui);
            toggle(ui, "files-again");
        },
    );
    let trigger = hover_tip(ui, p, "again", trigger, label.to_owned(), layer);
    menu.trigger(trigger).build(p.k, layer)
}

/// `trigger` with a tooltip saying `words` while the pointer is on it and
/// no menu is open; `key` names it in `files.hover`.
pub(super) fn hover_tip(
    ui: &Ui,
    p: Paint,
    key: &'static str,
    trigger: El,
    words: String,
    layer: &mut Vec<El>,
) -> El {
    let trigger = trigger.on_hover_change(move |ui: &mut Ui, inside, _| {
        if inside {
            ui.files.hover = Some(key);
        } else if ui.files.hover == Some(key) {
            ui.files.hover = None;
        }
    });
    rune_kit::Tooltip::new(format!("files-{key}-tip"))
        .trigger(trigger)
        .text(p.k, words)
        .placement(Placement::BottomStart)
        .open(ui.files.hover == Some(key) && ui.open.is_none())
        .build(p.k, layer)
}

/// A model chosen from Transcribe again: one that takes a language asks
/// for it first, starting at the saved language; any other runs at once.
fn choose_model(ui: &mut Ui, record: &str, model: TranscriptionModelId, name: &'static str) {
    ui.open = None;
    let Some(choices) = super::super::history::language_choices(model) else {
        ui.files.again = None;
        transcribe_again(ui, record, model, name, None);
        return;
    };
    let saved = &ui.settings.transcription.language;
    let language = if choices.contains(&saved.as_str()) {
        saved.clone()
    } else {
        ui.settings.transcription_for_model(model).language
    };
    ui.files.again = Some(Again {
        record: record.into(),
        model,
        name,
        language,
    });
}

/// Queues Transcribe again with `model`, in `language` when given.
fn transcribe_again(
    ui: &mut Ui,
    record: &str,
    model: TranscriptionModelId,
    name: &'static str,
    language: Option<String>,
) {
    let mut selection = ui.settings.transcription_for_model(model);
    if let Some(language) = language {
        selection.language = language;
    }
    let job = Job::Transcribe {
        selection: Some(selection),
        model_name: name.into(),
        again: true,
    };
    if let Some(task) = submit(ui, record, job) {
        ui.files.tab = Some(Tab::Task(task));
    }
}

/// The language for Transcribe again's model, with Transcribe and Cancel,
/// in a bordered row under the version line.
pub(super) fn again_row(ui: &Ui, p: Paint, again: &Again, layer: &mut Vec<El>) -> El {
    use crate::transcription::models::language_name;
    let choices = super::super::history::language_choices(again.model).unwrap_or_default();
    let labels: Vec<&str> = choices.iter().map(|code| language_name(code)).collect();
    let languages = rune_kit::Select::new("files-again-language", &labels)
        .appearance(rune_kit::SelectAppearance::Inline)
        .current(Some(language_name(&again.language)))
        .open(menu_open(ui, "files-again-language"))
        .on_toggle(|ui: &mut Ui| toggle(ui, "files-again-language"))
        .on_close(|ui: &mut Ui| ui.open = None)
        .on_choose(|ui: &mut Ui, choice: &str, _: &mut Tx| {
            ui.open = None;
            if let Some(again) = &mut ui.files.again
                && let Some(code) =
                    super::super::history::language_choices(again.model).and_then(|choices| {
                        choices
                            .into_iter()
                            .find(|code| language_name(code) == choice)
                    })
            {
                again.language = code.into();
            }
        })
        .build(p.k, layer);
    row()
        .key("again-pending")
        .gap(6.0)
        .pl(12.0)
        .pr(6.0)
        .py(6.0)
        .center_y()
        .radius(p.k.radius.md)
        .fill(p.k.c.layer_01)
        .border(p.hair, p.k.c.border_weak)
        .child(
            text(again.name)
                .key("model")
                .font(p.k.ty.ui)
                .color(p.text())
                .grow(1.0)
                .shrink()
                .truncate(),
        )
        .child(languages)
        .child(quiet_button(
            p,
            "again-start",
            None,
            tr("Transcribe"),
            None,
            false,
            |ui: &mut Ui| {
                if let Some(again) = ui.files.again.take() {
                    transcribe_again(
                        ui,
                        &again.record,
                        again.model,
                        again.name,
                        Some(again.language),
                    );
                }
            },
        ))
        .child(
            rune_kit::Button::new(tr("Cancel"))
                .variant(ButtonVariant::Ghost)
                .control_size(rune_kit::ControlSize::Small)
                .on_press(|ui: &mut Ui| ui.files.again = None)
                .build(p.k)
                .key("again-cancel"),
        )
}

/// Clean up again on another engine, as a new Cleanup version.
pub(super) fn redo_menu(
    ui: &Ui,
    p: Paint,
    record: &FileRecord,
    version: &Version,
    layer: &mut Vec<El>,
) -> El {
    let from = version
        .from
        .or(record.versions.first().map(|first| first.id))
        .unwrap_or(version.id);
    let open = menu_open(ui, "files-redo");
    let mut menu = KitMenu::new("files-redo-menu")
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .min_width(240.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None);
    let engines = &ui.files.engines;
    for (local, label) in [
        (true, tr("Built in, downloaded")),
        (false, tr("Set up in Output › AI cleanup")),
    ] {
        let group: Vec<_> = engines.iter().filter(|e| e.local == local).collect();
        if group.is_empty() {
            continue;
        }
        menu = menu.group_label(label);
        for engine in group {
            let (id, value) = (record.id.clone(), engine.value.clone());
            menu = menu.item(
                MenuItem::new(engine.label.clone()).on_select(move |ui: &mut Ui| {
                    ui.open = None;
                    let job = Job::Cleanup {
                        from,
                        engine: Some(value.clone()),
                    };
                    if let Some(task) = submit(ui, &id, job) {
                        ui.files.tab = Some(Tab::Task(task));
                    }
                }),
            );
        }
    }
    menu = menu.separator().item(
        MenuItem::new(tr("More models in Output › AI cleanup")).on_select(|ui: &mut Ui| {
            ui.open = None;
            ui.pane = Pane::Output;
        }),
    );
    let trigger = menu_button(
        p,
        "file-redo",
        Some(&icons::REFRESH),
        tr("Clean up again"),
        open,
        false,
        |ui: &mut Ui| {
            ui.files.engines = super::settings::engine_choices(ui);
            toggle(ui, "files-redo");
        },
    );
    menu.trigger(trigger).build(p.k, layer)
}

/// The quick requests as outlined chips, Recent, and the request field,
/// under a rule.
///
/// While `waiting` names a job running on this file, everything shows but
/// rests, faded, and the field says to wait for it or stop it.
pub(super) fn ask_area(ui: &Ui, p: Paint, waiting: Option<&str>, layer: &mut Vec<El>) -> El {
    let live = waiting.is_none();
    let listened = ui
        .files
        .selected
        .clone()
        .map(super::super::playback::Of::File);
    let mut chips = row().key("chips").gap(6.0).center_y().children(
        listened
            .as_ref()
            .and_then(|of| super::super::playback::lead(ui, p, of)),
    );
    let (language, _) = ui
        .settings
        .files
        .prompt_language(&ui.settings.transcription);
    for (index, chip) in ui.settings.files.quick_prompts.iter().enumerate() {
        let prompt = chip.resolved(language);
        let label = chip.label.clone();
        chips = chips.child(chip_button(
            p,
            &format!("chip{index}"),
            &chip.label,
            None,
            live,
            move |ui: &mut Ui| {
                let Some(record) = ui.files.selected().cloned() else {
                    return;
                };
                if let Some(Tab::Version(from)) = current(ui, &record) {
                    super::composer::start(ui, &record, from, prompt.clone(), Some(label.clone()));
                }
            },
        ));
    }
    if !ui.files.recent.is_empty() && !live {
        chips = chips.child(chip_button(
            p,
            "recent",
            tr("Recent"),
            Some(p.k.glyph.chevron_down),
            false,
            |_: &mut Ui| {},
        ));
    } else if !ui.files.recent.is_empty() {
        let open = menu_open(ui, "files-recent");
        let mut menu = KitMenu::new("files-recent-menu")
            .placement(Placement::TopStart)
            .gutter(4.0)
            .min_width(280.0)
            .open(open)
            .on_close(|ui: &mut Ui| ui.open = None)
            .group_label(tr("Recent requests"));
        for prompt in &ui.files.recent {
            let chosen = prompt.clone();
            menu = menu.item(MenuItem::new(prompt.clone()).on_select(move |ui: &mut Ui| {
                ui.open = None;
                ui.files.ask = TextEdit::new(chosen.clone());
            }));
        }
        if !ui.files.fixture {
            menu = menu
                .separator()
                .item(MenuItem::new(tr("Clear recent")).on_select(|ui: &mut Ui| {
                    ui.open = None;
                    if let Err(error) = store::clear_recent() {
                        tracing::warn!(%error, "couldn't clear recent requests");
                    }
                    ui.files.recent.clear();
                }));
        }
        let chevron = if open {
            p.k.glyph.chevron_up
        } else {
            p.k.glyph.chevron_down
        };
        let trigger = chip_button(
            p,
            "recent",
            tr("Recent"),
            Some(chevron),
            true,
            |ui: &mut Ui| toggle(ui, "files-recent"),
        );
        chips = chips.child(menu.trigger(trigger).build(p.k, layer));
    }
    let field = super::composer::field(
        ui,
        p,
        super::composer::Target::New,
        tr("Ask about this text"),
        None,
        waiting,
        layer,
    );
    let area = col()
        .key("files-ask-area")
        .gap(8.0)
        .pb(16.0)
        .child(
            listened
                .as_ref()
                .and_then(|of| super::super::playback::unfolded(ui, p, of, layer))
                .unwrap_or(chips),
        )
        .child(field);
    if live { area } else { area.dim(0.55) }
}

/// The field alone, for a session.
pub(super) fn composer_area(field: El) -> El {
    col().key("files-ask-area").gap(8.0).pb(16.0).child(field)
}

#[cfg(test)]
mod tests {
    use super::path_tail;

    #[test]
    fn the_path_shows_its_folder_and_a_long_name_keeps_its_end() {
        assert_eq!(
            path_tail(std::path::Path::new("/Users/a/Clients/Interview.m4a")),
            "…/Clients/Interview.m4a"
        );
        let long = path_tail(std::path::Path::new(
            "/x/Downloads/Zoom_Meeting_2026-11-04_18-05-33_recording_final_v2_audio_only.m4a",
        ));
        assert!(long.starts_with("…/Downloads/Zoom_Meeting_2026"), "{long}");
        assert!(long.ends_with("dio_only.m4a"), "{long}");
        assert!(long.contains('…'));
    }
}
