//! About: VibeHex's card (the version, the update check, and the release
//! notes), what the fork is with its source, and a way to reach its
//! developer by email or Telegram.

mod update;

use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize};

use super::Ui;
use super::brand;
use super::controls::{self, DESCRIPTION, Paint, icon_row, raised};
use super::icons::{MAIL, SEND, SOURCE, STAR};
use crate::desktop::i18n::tr;

pub(super) use update::BuildKind;

/// The developer's Telegram.
pub(super) const TELEGRAM_URL: &str = "https://t.me/DenisKey";
/// The fork's source.
pub(super) const SOURCE_URL: &str = "https://github.com/makeaiwork/VibeHex";

/// Opens `url` in its app (the browser, Mail, Telegram); the window shows
/// why it couldn't.
fn open(ui: &mut Ui, url: &str) {
    if ui.live()
        && let Err(error) = crate::desktop::opencode_catalog::open_url(url)
    {
        ui.error = Some(format!("{error:#}"));
    }
}

pub(super) fn pane(ui: &Ui, p: Paint) -> El {
    let content = controls::page("content")
        .child(app_card(ui, p))
        .child(fork_card(p))
        .child(developer(p));
    super::scroll_body(p.k, "scroll", content)
}

/// The logo, the name, the version, and under it the update check and the
/// release notes (`update`).
fn app_card(ui: &Ui, p: Paint) -> El {
    raised(p, "app").child(
        row()
            .key("app-row")
            .padding(16.0)
            .gap(14.0)
            .center_y()
            .child(brand::logo("logo", 56.0))
            .child(
                col()
                    .key("names")
                    .gap(3.0)
                    .child(
                        text(brand::NAME)
                            .key("name")
                            .font(rune::TextStyle {
                                size: 16.0,
                                ..p.k.ty.ui_medium
                            })
                            .color(p.text()),
                    )
                    .child(update::version(ui, p))
                    .children(update::control(ui, p))
                    .children(update::release_notes(ui, p)),
            ),
    )
}

/// What the fork is, a word asking for a star, and its source.
fn fork_card(p: Paint) -> El {
    // Starring happens on the repository's page.
    let open_source = Button::new(tr("Star on GitHub"))
        .icon(&STAR)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press(|ui: &mut Ui| open(ui, SOURCE_URL))
        .build(p.k)
        .key("open-source");
    controls::card(
        p,
        "fork",
        vec![
            col()
                .key("about")
                .gap(6.0)
                .px(controls::METRICS.row_px)
                .py(14.0)
                .child(
                    text(tr(
                        "VibeHex is a fork of Hex, the open-source dictation app. It adds many useful features of its own, and its code is open too.",
                    ))
                    .key("fork")
                    .font(p.k.ty.ui)
                    .color(p.muted())
                    .wrap(),
                )
                .child(
                    text(tr(
                        "If VibeHex is useful to you, a star on GitHub helps others find it.",
                    ))
                    .key("star")
                    .font(p.k.ty.ui)
                    .color(p.muted())
                    .wrap(),
                ),
            icon_row(
                p,
                "source",
                &SOURCE,
                tr("Source code"),
                Some("github.com/makeaiwork/VibeHex"),
                open_source,
            ),
        ],
    )
}

/// A quiet footer with no card: the developer, then email and Telegram.
fn developer(p: Paint) -> El {
    let email = Button::new(tr("Email the developer"))
        .icon(&MAIL)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press(|ui: &mut Ui| open(ui, &crate::labs::feedback_mailto(crate::VERSION)))
        .build(p.k)
        .key("email");
    let telegram = Button::new("Telegram — @DenisKey")
        .icon(&SEND)
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press(|ui: &mut Ui| open(ui, TELEGRAM_URL))
        .build(p.k)
        .key("telegram");
    col()
        .key("developer")
        .gap(10.0)
        .pt(8.0)
        .items_center()
        .child(
            text(tr(
                "Hi! I'm Denis, and I make VibeHex. Feedback, ideas, and bug reports are very welcome.",
            ))
            .key("hello")
            .font(DESCRIPTION)
            .color(p.muted())
            .wrap()
            .shrink(),
        )
        .child(row().key("contacts").gap(10.0).child(email).child(telegram))
}
