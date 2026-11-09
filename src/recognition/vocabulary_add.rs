//! Labs › Add to vocabulary from a selection in the listener: a tap of the
//! shortcut when it is bound to modifiers alone (Right Control), and the
//! action that opens the card over the selection.

use color_eyre::Result;

use super::Shortcut;
use crate::audio::CaptureInstant;
use crate::dictation::audio::DictationAudio;
use crate::suppression::{HotkeyAction, InputEvent, ModifierTap};

/// The action a completed tap of the modifiers-alone shortcut fires.
pub(super) fn tapped(
    tap: &mut ModifierTap,
    event: InputEvent,
    at: CaptureInstant,
) -> Option<(Shortcut, HotkeyAction)> {
    let vocabulary = crate::app_settings::runtime_hotkeys().vocabulary_add;
    tap.process(event, at, vocabulary)
        .then_some((Shortcut::Dictation, HotkeyAction::VocabularyAdd))
}

/// The shortcut fired: its chord's modifier press started a provisional
/// capture, which goes, and the card opens while the lab is on.
pub(super) fn open(dictation: &DictationAudio) -> Result<()> {
    crate::feedback::retract_start();
    dictation.cancel()?;
    if crate::labs::vocabulary_quick_add() {
        crate::rune_ui::vocabulary_card::open_for_selection();
    }
    Ok(())
}
