//! Brings the window up for a newly staged update, so its update sheet
//! ([`crate::rune_ui`]) shows even while the window is closed.

use crate::rune_ui::Ui;

/// Shows the window, without taking focus, once for each newly staged
/// update; never while dictation captures or transcribes, so a paste never
/// lands in Hex. `announced` is the version it last did so for.
pub(super) fn announce(window: &rune::AppHandle<Ui>, announced: &mut Option<String>) {
    let Some(version) = super::UPDATER
        .get()
        .and_then(crate::updater::Updater::notes)
        .map(|notes| notes.version)
    else {
        return;
    };
    if announced.as_ref() == Some(&version) || crate::transcription::engine::dictation_busy() {
        return;
    }
    *announced = Some(version);
    if !super::window_visible() {
        // An open window always shows the Dock icon (see `bring_forward`).
        crate::app_settings::set_dock_icon_visible(true);
        window.show();
    }
}
