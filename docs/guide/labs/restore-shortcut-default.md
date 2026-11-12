---
feature: restore-shortcut-default
key: labs.restore_shortcut_default
title: Restore shortcut default
for: []
ask: Add a button that restores the paste-last shortcut's default?
group: Shortcuts
os: every OS
vibehex: false
---

# Restore shortcut default

**What it does.** Puts a button beside the Paste last dictation shortcut that
brings the original shortcut back after you changed or cleared it. The default
is Option-Shift-V on a Mac and Ctrl+Alt+V on Windows and Linux.

**Turn it on when.** You changed the Paste last dictation shortcut and forgot
what it was, or you cleared it and want it back in one click.

**Leave it off when.** Nothing to lose; it only adds that button. Off, the row
is as before.

**Needs.** Nothing else.

**Good to know.** The button appears only while the shortcut is not already
the default; a cleared shortcut's row reads "Default shortcut: ...". If
another VibeHex shortcut already uses the default, nothing changes and the
recorder opens with "Already in use". It covers Paste last dictation only.

**Where.** Labs › Restore shortcut default (`labs.restore_shortcut_default`).
The button: General › Advanced › Output › Paste last dictation.

**Asked as.** "How do I reset the Paste last shortcut?"; "Restore the default
shortcut"; «Как вернуть сочетание клавиш по умолчанию»; «Сбросить шорткат
вставки последней диктовки».

<!-- sources: docs/features/labs.md:54,171-181; src/rune_ui/general.rs:291-330,920-935; src/app_settings/hotkeys.rs:658-680; src/rune_ui/labs.rs:432-437 -->
