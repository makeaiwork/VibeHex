//! About: VibeHex's card, what the fork is with its source, and a way to
//! reach its developer by email or Telegram.

use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, ControlSize};

use super::Ui;
use super::brand;
use super::controls::{self, DESCRIPTION, Paint, icon_row, raised};
use super::icons::{MAIL, SEND, SOURCE};
use crate::desktop::i18n::tr;

/// Where feedback on VibeHex goes.
pub(super) const DEVELOPER_EMAIL: &str = "zelenpro@gmail.com";
/// The developer's Telegram.
pub(super) const TELEGRAM_URL: &str = "https://t.me/DenisKey";
/// The fork's source.
pub(super) const SOURCE_URL: &str = "https://github.com/makeaiwork/hex";

/// A `mailto:` link to [`DEVELOPER_EMAIL`] whose subject names `version`.
pub(super) fn feedback_mailto(version: &str) -> String {
    format!(
        "mailto:{DEVELOPER_EMAIL}?subject={}",
        mailto_encode(&format!("VibeHex feedback (version {version})"))
    )
}

/// Percent-encodes a `mailto:` query value: everything but unreserved
/// characters, so spaces become `%20` (not `+`, which mail apps show as is).
fn mailto_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Opens `url` in its app (the browser, Mail, Telegram); the window shows
/// why it couldn't.
fn open(ui: &mut Ui, url: &str) {
    if ui.live()
        && let Err(error) = crate::desktop::opencode_catalog::open_url(url)
    {
        ui.error = Some(format!("{error:#}"));
    }
}

pub(super) fn pane(_ui: &Ui, p: Paint) -> El {
    let content = controls::page("content")
        .child(app_card(p))
        .child(fork_card(p))
        .child(developer(p));
    super::scroll_body("scroll", content)
}

/// The logo, the name, and the version.
fn app_card(p: Paint) -> El {
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
                    .child(
                        text(super::sidebar::version_label())
                            .key("version")
                            .font(DESCRIPTION)
                            .color(p.muted()),
                    ),
            ),
    )
}

/// What the fork is, and its source.
fn fork_card(p: Paint) -> El {
    let open_source = Button::new(tr("Open on GitHub"))
        .variant(ButtonVariant::Neutral)
        .control_size(ControlSize::Small)
        .on_press(|ui: &mut Ui| open(ui, SOURCE_URL))
        .build(p.k)
        .key("open-source");
    controls::card(
        p,
        "fork",
        vec![
            text(tr(
                "VibeHex is a fork of Hex, the open-source dictation app. It adds many useful features of its own, and its code is open too.",
            ))
            .key("about")
            .font(p.k.ty.ui)
            .color(p.muted())
            .wrap()
            .px(controls::METRICS.row_px)
            .py(14.0),
            icon_row(
                p,
                "source",
                &SOURCE,
                tr("Source code"),
                Some("github.com/makeaiwork/hex"),
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
        .on_press(|ui: &mut Ui| open(ui, &feedback_mailto(crate::VERSION)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_mailto_names_the_version_in_the_subject() {
        assert_eq!(
            feedback_mailto("0.0.0-dev-12"),
            "mailto:zelenpro@gmail.com?subject=VibeHex%20feedback%20%28version%200.0.0-dev-12%29"
        );
    }
}
