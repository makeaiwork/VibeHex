---
feature: quit-closes-window
key: labs.quit_closes_window
title: ⌘Q closes Settings
group: Dictation
os: macOS
vibehex: false
---

# ⌘Q closes Settings

**What it does.** Normally ⌘Q quits VibeHex, and dictation stops. With this on, ⌘Q only closes the Settings window, as ⌘W does, and your shortcut keeps working.

**Turn it on when.** You press ⌘Q out of habit and keep finding dictation gone.

**Leave it off when.** You use ⌘Q to really quit.

**Needs.** macOS. Nothing else.

**Good to know.** Quit from the menu bar icon still quits, as does the Dock's Quit. Stable VibeHex builds don't list it; only dev builds do.

**Where.** Labs › ⌘Q closes Settings (`labs.quit_closes_window`).

**Asked as.** "Cmd+Q should just close the window"; «Cmd+Q выключает диктовку».

<!-- sources: docs/features/labs.md:35-40, 111, 1317-1330; src/labs.rs:264-275, 1234-1239; src/rune_ui/labs.rs:594-600 -->
