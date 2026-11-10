//! The header's conversation picker: the open conversation's name and
//! when it was last used, a list of every conversation to switch to, and
//! "+" for a new one.

use rune::{El, Tx, col, el, icon, row, text};
use rune_kit::{Placement, Popover};

use super::super::controls::{Paint, icon_button};
use super::super::files::session::when;
use super::super::{Menu, Ui};
use super::composer::new_conversation;
use crate::assistant::store::Conversation;
use crate::desktop::i18n::{tr, tr_fill, tr_named};

const MENU: &str = "assistant-conversations";
static PLUS: &rune::Icon = &rune_theme_opencode::icons::PLUS;

/// The picker and "+", once there is a conversation to pick.
pub(super) fn actions(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> Vec<El> {
    let pane = &ui.assistant;
    if pane.store.conversations().is_empty() {
        return Vec::new();
    }
    let new = icon_button(p, "assistant-new", PLUS, tr("New conversation"), |ui| {
        new_conversation(ui)
    });
    vec![picker(ui, p, layer), new]
}

/// The trigger and the list it opens.
fn picker(ui: &Ui, p: Paint, layer: &mut Vec<El>) -> El {
    let c = &p.k.c;
    let pane = &ui.assistant;
    let open = ui.open == Some(Menu::Pick(MENU));
    let (name, detail) = match pane.conversation() {
        Some(conversation) => (conversation.name.clone(), when(conversation.updated_ms())),
        None => (
            tr("New conversation").to_owned(),
            tr_fill("{} earlier", &pane.store.conversations().len().to_string()),
        ),
    };
    let small = rune::TextStyle {
        size: 12.0,
        ..p.k.ty.ui
    };
    let trigger = row()
        .key("assistant-picker")
        .role("button")
        .label(tr("Conversations"))
        .focusable(true)
        .focus_ring(2.0, 0.0, c.border_focus)
        .h(28.0)
        .max_w(360.0)
        .pl(10.0)
        .pr(6.0)
        .gap(8.0)
        .center_y()
        .radius(p.k.radius.sm)
        .border(p.hair, c.border_base)
        .hover(c.hover)
        .press(c.pressed)
        .child(
            text(name)
                .key("name")
                .font(p.k.ty.ui_medium)
                .color(p.text())
                .shrink()
                .truncate(),
        )
        .child(text(detail).key("when").font(small).color(p.faint()))
        .child(
            icon(
                if open {
                    p.k.glyph.chevron_up
                } else {
                    p.k.glyph.chevron_down
                },
                12.0,
            )
            .key("chevron")
            .color(c.icon_muted),
        )
        .on_press(|ui: &mut Ui| {
            let menu = Menu::Pick(MENU);
            ui.open = (ui.open != Some(menu)).then_some(menu);
        });
    let mut popover = Popover::new(format!("{MENU}-menu"))
        .placement(Placement::BottomEnd)
        .gutter(4.0)
        .padding(4.0)
        .open(open)
        .on_close(|ui: &mut Ui| ui.open = None)
        .trigger(trigger);
    let mut conversations: Vec<&Conversation> = pane.store.conversations().iter().collect();
    conversations.sort_by_key(|conversation| std::cmp::Reverse(conversation.updated_ms()));
    let mut list = col().key("list").max_h(360.0).scroll(rune::ScrollSpec {
        stick_bottom: false,
    });
    for conversation in conversations {
        list = list.child(choice(p, conversation, pane.open == Some(conversation.id)));
    }
    popover = popover.child(list);
    let new = row()
        .key("new-conversation")
        .role("menuitem")
        .label(tr("New conversation"))
        .px(10.0)
        .py(7.0)
        .gap(8.0)
        .center_y()
        .radius(4.0)
        .hover(c.hover)
        .child(el().key("check").size(14.0, 16.0))
        .child(
            text(format!("+ {}", tr("New conversation")))
                .key("label")
                .font(p.k.ty.ui)
                .color(c.text_accent),
        )
        .on_press(new_conversation);
    popover
        .child(
            el().key("rule")
                .h(p.hair)
                .mx(4.0)
                .my(4.0)
                .fill(c.border_weak),
        )
        .child(new)
        .build(p.k, layer)
}

/// One conversation: its name over how many questions and when, ✓ on the
/// open one.
fn choice(p: Paint, conversation: &Conversation, on: bool) -> El {
    let c = &p.k.c;
    let updated = when(conversation.updated_ms());
    let asked = match conversation.turns.len() {
        1 => tr_fill("1 question, {}", &updated),
        count => tr_named(
            "{count} questions, {when}",
            &[("count", &count.to_string()), ("when", &updated)],
        ),
    };
    let id = conversation.id;
    row()
        .key(format!("conversation{id}"))
        .role("menuitem")
        .label(conversation.name.clone())
        .w(300.0)
        .px(10.0)
        .py(6.0)
        .gap(8.0)
        .items_start()
        .radius(4.0)
        .fill(if on {
            c.hover
        } else {
            rune::Color::TRANSPARENT
        })
        .hover(c.hover)
        .child(
            el().key("check").size(14.0, 16.0).center().child(
                icon(p.k.glyph.check_small, 14.0)
                    .color(c.icon_base)
                    .opacity(if on { 1.0 } else { 0.0 }),
            ),
        )
        .child(
            col()
                .key("words")
                .grow(1.0)
                .shrink()
                .gap(2.0)
                .child(
                    text(conversation.name.clone())
                        .key("name")
                        .font(p.k.ty.ui)
                        .color(p.text())
                        .wrap_anywhere()
                        .max_lines(2),
                )
                .child(
                    text(asked)
                        .key("asked")
                        .font(p.k.ty.small)
                        .color(p.muted())
                        .truncate(),
                ),
        )
        .on_press_tx(move |ui: &mut Ui, tx: &mut Tx| {
            tx.settle();
            ui.open = None;
            ui.assistant.open = Some(id);
            ui.assistant.ask = rune::TextEdit::default();
        })
}
