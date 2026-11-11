---
feature: quit-closes-window
key: labs.quit_closes_window
title: ⌘Q closes Settings
for: []
ask: Make ⌘Q close the window instead of stopping dictation?
group: Dictation
os: macOS
vibehex: false
---

# ⌘Q closes Settings

**What it does.** Normally ⌘Q quits VibeHex, and dictation stops. With this on, ⌘Q from the app menu only closes the Settings window, as ⌘W does, and your dictation shortcut keeps working.

**Turn it on when.** You press ⌘Q out of habit to put the window away, then find dictation gone.

**Leave it off when.** You use ⌘Q to really quit.

**Needs.** macOS. Nothing else.

**Good to know.** It applies at once. Quit from the menu bar icon still quits, and so does the Dock's Quit. Only dev builds of VibeHex list this lab.

**Where.** Labs › ⌘Q closes Settings (`labs.quit_closes_window`).

**Asked as.** "Cmd+Q should just close the window"; "Stop Cmd+Q from quitting VibeHex"; «Cmd+Q выключает диктовку»; «Как сделать, чтобы ⌘Q не закрывал приложение».

<!-- sources: src/rune_ui/labs.rs:594-600; src/macos_rune.rs:1438-1448, 990-1000; src/macos_rune/main_menu.rs:20-40; src/labs.rs:267-276, 1237-1239; src/control/settings.rs:513-516; docs/features/labs.md:1317-1330 -->
