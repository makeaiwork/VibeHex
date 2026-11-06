//! VibeHex's name and logo: the sidebar's header and About's card.

use std::sync::{Arc, OnceLock};

use rune::{El, row, text};

use super::controls::Paint;

/// The fork's name, shown beside the logo.
pub(super) const NAME: &str = "VibeHex";

/// Hex's app icon, the 128 px one Linux installs.
static LOGO_PNG: &[u8] =
    include_bytes!("../../packaging/linux/icons/hicolor/128x128/apps/anomaly.hex.png");

/// The logo's pixels, decoded once when first shown. It is small, so the
/// first frame waits for it rather than drawing without it.
fn logo_pixels() -> Option<&'static rune::Pixels> {
    static LOGO: OnceLock<Option<rune::Image>> = OnceLock::new();
    LOGO.get_or_init(|| {
        let cache = rune::ImageCache::new(Arc::new(|| {}));
        let image = cache.load(LOGO_PNG);
        image.wait().ok()?;
        // Decoding marks the image ready a moment before its pixels hold it.
        while image.pixels().size() == (0, 0) {
            std::thread::yield_now();
        }
        Some(image)
    })
    .as_ref()
    .map(rune::Image::pixels)
}

/// The logo's mark alone, without the app icon's square: two hexagons and
/// a dot, in the text's colour, for the sidebar on either theme.
static MARK: rune::Icon = rune::Icon {
    name: "hex-brand-mark",
    view: 16.0,
    body: r#"<path d="M8 1.2 13.9 4.6v6.8L8 14.8 2.1 11.4V4.6z" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/><path d="M8 4.1 11.4 6v4L8 11.9 4.6 10V6z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/><circle cx="8" cy="8" r="1.4" fill="currentColor"/>"#,
};

/// The mark, `size` points square.
pub(super) fn mark(p: Paint, key: &str, size: f32) -> El {
    rune::icon(&MARK, size).key(key.to_owned()).color(p.text())
}

/// The logo, `size` points square.
pub(super) fn logo(key: &str, size: f32) -> El {
    let logo = rune::el().key(key).size(size, size);
    match logo_pixels() {
        Some(pixels) => logo.pixels(pixels),
        None => logo,
    }
}

/// The sidebar's header: the logo and the name, on the navigation's
/// icon edge.
pub(super) fn header(p: Paint) -> El {
    row()
        .key("brand")
        .role("heading")
        .label(NAME)
        .h(28.0)
        .px(4.0)
        .gap(8.0)
        .center_y()
        .child(mark(p, "logo", 22.0))
        .child(
            text(NAME)
                .key("name")
                .font(p.k.ty.ui_medium)
                .color(p.text()),
        )
}
