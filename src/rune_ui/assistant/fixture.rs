//! The Assistant's scenes: fixed conversations on a Mac, rendered at time
//! zero, so nothing here reads the clock, the disk, or a model.

use super::super::Ui;
use super::super::scenes::mac_fixture;
use super::super::sidebar::Pane;

/// Assistant's pane on a Mac.
fn open(ui: &mut Ui) {
    *ui = mac_fixture();
    ui.pane = Pane::Assistant;
}

/// A conversation: a question, an answer from the guide with its sources.
pub(in super::super) fn conversation(ui: &mut Ui) {
    open(ui);
}
