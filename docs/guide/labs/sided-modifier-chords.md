---
feature: sided-modifier-chords
key: labs.sided_modifier_chords
title: Sided modifier chords
for: []
ask: Let shortcuts tell left and right modifier keys apart?
group: Shortcuts
os: macOS
vibehex: false
---

# Sided modifier chords

**What it does.** When you record a shortcut made only of modifier keys, such
as Right ⌘ + Right ⌥, VibeHex remembers which side each key was on. Then only
those keys trigger it, and the left ones don't.

**Turn it on when.** You want a two-modifier shortcut on one side of the
keyboard so it can't fire by accident from the other.

**Leave it off when.** Either side is fine. Off, a combination such as ⌥⌘
records from either side, as before.

**Needs.** macOS. Nothing else.

**Good to know.** It only changes how the recorder records; saved shortcuts
and conflict checks work as before. A shortcut with a normal key in it still
accepts its modifiers from either side. When the dictation shortcut is a single
modifier, General › Advanced has its own side choice for it.

**Where.** Labs › Sided modifier chords (`labs.sided_modifier_chords`). Used
when recording any shortcut in General.

**Asked as.** "Record only the right Command and Option"; "Shortcut must use
the right-hand modifiers"; «Сочетание только из правых Command и Option»;
«Записать правый и левый модификатор по отдельности».

<!-- sources: docs/features/labs.md:64,397-414; src/rune_ui/labs.rs:362-369; src/rune_ui/general.rs:857-880 -->
