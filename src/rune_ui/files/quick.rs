//! Files › Settings › Quick requests: the chips above the request field.
//! Settings shows one row for them; Edit… opens this page, the list on the
//! left and the open request's name and prompt on the right. Built-in
//! requests start from Hex's prompt in the speech model's language and say
//! Edited once changed; Reset to default brings the default back.

use rune::{El, TextEdit, col, row, text};
use rune_kit::{Button, ButtonVariant, TextArea, TextInput};

use super::super::Ui;
use super::super::controls::{DESCRIPTION, Paint, PushSize, icon_row};
use super::super::icons;
use super::super::lists::{Tone, select_at_once, source_item, source_list, tone_tag};
use crate::app_settings::QuickPrompt;
use crate::app_settings::prompt_defaults::{self, Basis};
use crate::desktop::i18n::{tr, tr_fill, tr_named};

/// The open Quick requests page: the request in the editor (`None` while a
/// new one is written) and its fields as edited.
#[derive(Clone, Debug, Default)]
pub(in super::super) struct QuickPage {
    pub(in super::super) index: Option<usize>,
    pub(in super::super) label: TextEdit,
    pub(in super::super) prompt: TextEdit,
}

/// The prompts' language and why, and the speech model's name.
fn language(ui: &Ui) -> (&'static str, Basis, &'static str) {
    let files = &ui.settings.files;
    let dictation = &ui.settings.transcription;
    let (language, basis) = files.prompt_language(dictation);
    let model = files
        .selection(dictation)
        .unwrap_or_else(|| dictation.clone())
        .model;
    let model = crate::transcription::models::definition(model).name;
    (language, basis, model)
}

/// Settings' row: what the chips are, the prompts' language, and Edit….
pub(super) fn settings_row(ui: &Ui, p: Paint) -> El {
    let (language, basis, _) = language(ui);
    let chips = &ui.settings.files.quick_prompts;
    let names = chips
        .iter()
        .map(|chip| chip.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let language = prompt_defaults::language_name(language);
    let description = match (names.is_empty(), basis) {
        (true, _) => tr("None yet. They show above the request field").to_owned(),
        (false, Basis::SpeechModel) => tr_named(
            "{names}. Prompts for {language}, the language of your speech model",
            &[("names", &names), ("language", language)],
        ),
        (false, Basis::Interface) => tr_named(
            "{names}. Prompts for {language}, the app's language",
            &[("names", &names), ("language", language)],
        ),
    };
    let status = if chips.iter().any(QuickPrompt::edited) {
        tr("Edited")
    } else {
        tr("Default")
    };
    let control = row()
        .key("quick-control")
        .gap(10.0)
        .center_y()
        .child(
            text(status)
                .key("status")
                .font(DESCRIPTION)
                .color(p.faint()),
        )
        .child(
            Button::new(tr("Edit…"))
                .variant(ButtonVariant::Neutral)
                .on_press(|ui: &mut Ui| open(ui, Some(0)))
                .build_push(p.k)
                .key("quick-edit"),
        );
    icon_row(
        p,
        "files-quick",
        &icons::MESSAGE,
        tr("Quick requests"),
        Some(&description),
        control,
    )
}

/// Opens request `index` in the editor, or a new one.
pub(super) fn open(ui: &mut Ui, index: Option<usize>) {
    let (language, _, _) = language(ui);
    let chip = index.and_then(|index| ui.settings.files.quick_prompts.get(index));
    ui.files.quick = Some(match chip {
        Some(chip) => QuickPage {
            index,
            label: TextEdit::new(chip.label.clone()),
            prompt: TextEdit::new(chip.resolved(language)),
        },
        None => QuickPage::default(),
    });
}

fn page_slot(ui: &mut Ui) -> &mut QuickPage {
    ui.files.quick.get_or_insert_with(QuickPage::default)
}

/// The page: back to Settings, the prompts' language, then the list beside
/// the editor.
pub(super) fn page(ui: &Ui, p: Paint, edit: &QuickPage) -> El {
    let (language, basis, model) = language(ui);
    let note = match basis {
        Basis::SpeechModel => tr_fill(
            "The language of your speech model, {}. Pick another model or language and the prompts switch to that language's defaults",
            model,
        ),
        Basis::Interface => tr_fill(
            "Your speech model, {}, isn't set to one language VibeHex writes prompts in, so they follow the app's language",
            model,
        ),
    };
    let back = Button::new(format!("‹ {}", tr("Settings")))
        .variant(ButtonVariant::Ghost)
        .on_press(|ui: &mut Ui| ui.files.quick = None)
        .build_small_push(p.k)
        .key("quick-back");
    col()
        .key("files-quick-page")
        .gap(12.0)
        .child(row().key("back-row").child(back))
        .child(
            text(tr("Quick requests"))
                .key("title")
                .font(p.k.ty.body_medium)
                .color(p.text()),
        )
        .child(
            row()
                .key("language")
                .gap(10.0)
                .child(tone_tag(
                    p,
                    "language-tag",
                    prompt_defaults::language_name(language),
                    Tone::Neutral,
                ))
                .child(
                    text(note)
                        .key("note")
                        .font(DESCRIPTION)
                        .color(p.muted())
                        .wrap()
                        .grow(1.0)
                        .shrink(),
                ),
        )
        .child(
            row()
                .key("quick-split")
                .gap(12.0)
                .child(list(ui, p, edit))
                .child(editor(ui, p, edit, language)),
        )
}

/// The requests, the open one filled, and Add request.
fn list(ui: &Ui, p: Paint, edit: &QuickPage) -> El {
    let c = &p.k.c;
    let mut items = col().key("items").gap(2.0);
    for (index, chip) in ui.settings.files.quick_prompts.iter().enumerate() {
        let selected = edit.index == Some(index);
        let mut item = row()
            .key(format!("quick{index}"))
            .role("button")
            .label(chip.label.as_str())
            .px(10.0)
            .py(7.0)
            .gap(8.0)
            .center_y()
            .radius(p.k.radius.md)
            .hover(c.hover)
            .child(
                text(chip.label.clone())
                    .key("label")
                    .font(p.k.ty.ui)
                    .color(p.text())
                    .grow(1.0)
                    .shrink()
                    .truncate(),
            )
            .children(chip.edited().then(|| {
                text(tr("Edited"))
                    .key("edited")
                    .font(p.k.ty.small)
                    .color(p.warning())
            }));
        if selected {
            item = item.fill(c.layer_02);
        }
        items = items.child(source_item(
            select_at_once(item, selected, move |ui: &mut Ui| open(ui, Some(index))),
            selected,
        ));
    }
    let adding = edit.index.is_none();
    let mut add = row()
        .key("quick-add")
        .role("button")
        .label(tr("Add request"))
        .px(10.0)
        .py(7.0)
        .radius(p.k.radius.md)
        .hover(c.hover)
        .child(
            text(format!("+ {}", tr("Add request")))
                .key("label")
                .font(p.k.ty.ui)
                .color(c.text_accent),
        )
        .on_press(|ui: &mut Ui| open(ui, None));
    if adding {
        add = add.fill(c.layer_02);
    }
    col()
        .key("quick-list")
        .self_start()
        .w(200.0)
        .px(6.0)
        .py(6.0)
        .gap(4.0)
        .radius(p.k.radius.lg)
        .fill(c.layer_01)
        .border(p.hair, c.border_weak)
        .child(source_list(items))
        .child(rune::el().key("rule").h(p.hair).mx(4.0).fill(c.border_weak))
        .child(add)
}

/// The open request's name and prompt, what it is, Remove, Reset to
/// default, and Save.
fn editor(ui: &Ui, p: Paint, edit: &QuickPage, language: &'static str) -> El {
    let saved = edit
        .index
        .and_then(|index| ui.settings.files.quick_prompts.get(index));
    let default = saved
        .and_then(QuickPrompt::builtin_id)
        .and_then(|id| prompt_defaults::quick_default(id, language));
    let label = edit.label.text.trim();
    let prompt = edit.prompt.text.trim();
    let changed = match saved {
        Some(saved) => label != saved.label.trim() || prompt != saved.resolved(language),
        None => !label.is_empty() || !prompt.is_empty(),
    };
    let ready = !label.is_empty() && !prompt.is_empty();
    let status = match (default, saved) {
        (Some(default), _) if prompt == default.trim() => tr("Default prompt"),
        (Some(_), _) => tr("Edited"),
        (None, Some(_)) => tr("Your own request"),
        (None, None) => tr("A new request"),
    };
    let mut buttons = row().key("buttons").gap(8.0).center_y();
    if let Some(index) = edit.index {
        buttons = buttons.child(
            Button::new(tr("Remove"))
                .variant(ButtonVariant::Ghost)
                .on_press(move |ui: &mut Ui| remove(ui, index))
                .build_push(p.k)
                .key("quick-remove"),
        );
    }
    if let Some(default) = default {
        let at_default = prompt == default.trim();
        buttons = buttons.child(
            Button::new(tr("Reset to default"))
                .variant(ButtonVariant::Neutral)
                .disabled(at_default)
                .on_press(move |ui: &mut Ui| {
                    page_slot(ui).prompt = TextEdit::new(default.to_owned());
                })
                .build_push(p.k)
                .key("quick-reset"),
        );
    }
    buttons = buttons.child(
        Button::new(tr("Save"))
            .variant(ButtonVariant::Contrast)
            .disabled(!(changed && ready))
            .on_press(move |ui: &mut Ui| save(ui, language))
            .build_push(p.k)
            .key("quick-save"),
    );
    col()
        .key("quick-editor")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(10.0)
        .child(
            row()
                .key("name-row")
                .gap(10.0)
                .center_y()
                .child(
                    text(tr("Name"))
                        .key("name")
                        .font(DESCRIPTION)
                        .color(p.muted()),
                )
                .child(
                    TextInput::new(&edit.label)
                        .placeholder(tr("Name"))
                        .width(Some(240.0))
                        .on_edit(|ui: &mut Ui| &mut page_slot(ui).label)
                        .build(p.k)
                        .key("label"),
                ),
        )
        .child(
            TextArea::new(&edit.prompt)
                .rows(16)
                .max_height(520.0)
                .width(None)
                .placeholder(tr("What it asks for"))
                .on_edit(|ui: &mut Ui| &mut page_slot(ui).prompt)
                .build(p.k)
                .key("prompt"),
        )
        .child(
            row()
                .key("footer")
                .gap(8.0)
                .center_y()
                .child(
                    text(status)
                        .key("status")
                        .font(DESCRIPTION)
                        .color(p.faint())
                        .grow(1.0)
                        .shrink(),
                )
                .child(buttons),
        )
}

/// Saves the open request in its place, or at the end. A built-in request
/// saved with its default keeps following the default.
fn save(ui: &mut Ui, language: &'static str) {
    let Some(edit) = ui.files.quick.clone() else {
        return;
    };
    let label = edit.label.text.trim().to_owned();
    let prompt = edit.prompt.text.trim().to_owned();
    if label.is_empty() || prompt.is_empty() {
        return;
    }
    let mut index = edit.index;
    ui.update(|settings| {
        let chips = &mut settings.files.quick_prompts;
        match index.filter(|index| *index < chips.len()) {
            Some(at) => {
                let chip = &mut chips[at];
                let default = chip
                    .builtin_id()
                    .and_then(|id| prompt_defaults::quick_default(id, language));
                if let Some(id) = chip.builtin_id() {
                    chip.builtin = id.into();
                }
                chip.prompt = match default {
                    Some(default) if default.trim() == prompt => String::new(),
                    _ => prompt,
                };
                chip.label = label;
            }
            None => {
                chips.push(QuickPrompt {
                    label,
                    prompt,
                    builtin: String::new(),
                });
                index = Some(chips.len() - 1);
            }
        }
    });
    open(ui, index);
}

/// Removes request `index` and opens the one now in its place.
fn remove(ui: &mut Ui, index: usize) {
    ui.update(|settings| {
        if index < settings.files.quick_prompts.len() {
            settings.files.quick_prompts.remove(index);
        }
    });
    let left = ui.settings.files.quick_prompts.len();
    open(ui, (left > 0).then(|| index.min(left - 1)));
}
