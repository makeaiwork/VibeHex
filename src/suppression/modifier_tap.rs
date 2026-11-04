//! A one-shot shortcut on modifiers alone, such as Right Control for Labs ›
//! Add to vocabulary from a selection: it fires when exactly its modifiers
//! go down and all come up again quickly with nothing else pressed between,
//! so the same key still works in chords (⌃C) and held for other uses.

use std::time::Duration;

use crate::app_settings::{HotkeyModifiers, RuntimeHotkey, modifiers_from_flags};
use crate::audio::CaptureInstant;

use super::InputEvent;

/// The longest press that still counts as a tap.
const MAX_TAP: Duration = Duration::from_millis(600);

#[derive(Default)]
pub(crate) struct ModifierTap {
    /// When exactly the binding's modifiers went down, while nothing else
    /// has been pressed since.
    armed_at: Option<CaptureInstant>,
}

impl ModifierTap {
    /// Feeds one input; true when it completes a tap of `binding`. A binding
    /// with a key is the key press's to match, never a tap's.
    pub(crate) fn process(
        &mut self,
        event: InputEvent,
        at: CaptureInstant,
        binding: Option<RuntimeHotkey>,
    ) -> bool {
        let Some(binding) = binding.filter(|binding| {
            binding.key_code.is_none() && binding.modifiers != HotkeyModifiers::default()
        }) else {
            self.armed_at = None;
            return false;
        };
        let InputEvent::Flags(flags) = event else {
            // A key, click, or lost tap between press and release makes it a
            // chord, not a tap.
            if !matches!(event, InputEvent::Key { down: false, .. }) {
                self.armed_at = None;
            }
            return false;
        };
        let held = modifiers_from_flags(flags);
        if held == HotkeyModifiers::default() {
            let tapped = self
                .armed_at
                .take()
                .and_then(|armed| at.checked_duration_since(armed))
                .is_some_and(|held_for| held_for <= MAX_TAP);
            return tapped;
        }
        if binding.exact_modifiers(flags) {
            self.armed_at.get_or_insert(at);
        } else if !(self.armed_at.is_some() && within(held, binding.modifiers)) {
            // Another modifier joined; letting go of the binding's own keys
            // one by one keeps the tap.
            self.armed_at = None;
        }
        false
    }
}

/// Whether every modifier in `held` is one `binding` has.
fn within(held: HotkeyModifiers, binding: HotkeyModifiers) -> bool {
    (held.control.is_none() || binding.control.is_some())
        && (held.option.is_none() || binding.option.is_some())
        && (held.shift.is_none() || binding.shift.is_some())
        && (held.command.is_none() || binding.command.is_some())
        && (!held.function || binding.function)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_settings::{
        CONTROL_KEY_MASK, LEFT_CONTROL_MASK, LEFT_SHIFT_MASK, ModifierSide, RIGHT_CONTROL_MASK,
        SHIFT_KEY_MASK,
    };

    const RIGHT_CONTROL: u64 = CONTROL_KEY_MASK | RIGHT_CONTROL_MASK;
    const LEFT_CONTROL: u64 = CONTROL_KEY_MASK | LEFT_CONTROL_MASK;

    fn right_control() -> Option<RuntimeHotkey> {
        Some(RuntimeHotkey {
            modifiers: HotkeyModifiers {
                control: Some(ModifierSide::Right),
                ..Default::default()
            },
            key_code: None,
        })
    }

    fn at(ms: u64) -> CaptureInstant {
        CaptureInstant::from_nanos(1_000_000_000 + ms * 1_000_000)
    }

    fn feed(tap: &mut ModifierTap, events: &[(InputEvent, u64)]) -> Vec<u64> {
        events
            .iter()
            .filter(|(event, ms)| tap.process(*event, at(*ms), right_control()))
            .map(|(_, ms)| *ms)
            .collect()
    }

    #[test]
    fn a_quick_tap_of_right_control_alone_fires_on_release() {
        let mut tap = ModifierTap::default();
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(RIGHT_CONTROL), 0),
                (InputEvent::Flags(0), 120),
            ],
        );
        assert_eq!(fired, [120]);
    }

    #[test]
    fn chords_long_holds_other_sides_and_extra_modifiers_never_fire() {
        let key = |down| InputEvent::Key {
            code: 8,
            down,
            flags: RIGHT_CONTROL,
        };
        let mut tap = ModifierTap::default();
        // ⌃C with Right Control.
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(RIGHT_CONTROL), 0),
                (key(true), 50),
                (key(false), 90),
                (InputEvent::Flags(0), 120),
            ],
        );
        assert!(fired.is_empty());
        // Held too long.
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(RIGHT_CONTROL), 0),
                (InputEvent::Flags(0), 900),
            ],
        );
        assert!(fired.is_empty());
        // Left Control.
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(LEFT_CONTROL), 0),
                (InputEvent::Flags(0), 100),
            ],
        );
        assert!(fired.is_empty());
        // Shift joins.
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(RIGHT_CONTROL), 0),
                (
                    InputEvent::Flags(RIGHT_CONTROL | SHIFT_KEY_MASK | LEFT_SHIFT_MASK),
                    40,
                ),
                (InputEvent::Flags(0), 100),
            ],
        );
        assert!(fired.is_empty());
        // A click between press and release.
        let fired = feed(
            &mut tap,
            &[
                (InputEvent::Flags(RIGHT_CONTROL), 0),
                (InputEvent::MouseDown, 40),
                (InputEvent::Flags(0), 100),
            ],
        );
        assert!(fired.is_empty());
    }

    #[test]
    fn a_binding_with_a_key_or_none_never_taps() {
        let mut tap = ModifierTap::default();
        let with_key = right_control().map(|binding| RuntimeHotkey {
            key_code: Some(0),
            ..binding
        });
        assert!(!tap.process(InputEvent::Flags(RIGHT_CONTROL), at(0), with_key));
        assert!(!tap.process(InputEvent::Flags(0), at(50), with_key));
        assert!(!tap.process(InputEvent::Flags(RIGHT_CONTROL), at(0), None));
        assert!(!tap.process(InputEvent::Flags(0), at(50), None));
    }
}
