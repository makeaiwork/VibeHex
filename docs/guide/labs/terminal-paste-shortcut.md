---
feature: terminal-paste-shortcut
key: labs.terminal_paste_shortcut
title: Terminal paste shortcut
group: Shortcuts
os: [Windows, Linux]
vibehex: false
---

# Terminal paste shortcut

**What it does.** Adds another dictation shortcut that always pastes with Ctrl+Shift+V, which terminals accept, while your main shortcut keeps its usual paste.

**Turn it on when.** You dictate into terminals and ordinary apps, and want a key for each.

**Leave it off when.** Ctrl+V works everywhere you dictate.

**Needs.** Windows or Linux. On Linux, X11 or a desktop with the shortcuts portal.

**Good to know.** It's unbound until you record a key. Paste last dictation keeps the main paste setting.

**Where.** Labs › Terminal paste shortcut (`labs.terminal_paste_shortcut`); then General › Shortcut › Terminal paste shortcut (`terminal_paste_hotkey`).

**Asked as.** "A separate shortcut for dictating into the terminal"; «Отдельная клавиша для диктовки в терминал».

<!-- sources: docs/features/labs.md:71-72, 607-638; docs/features/dictation.md:384-412; src/rune_ui/labs.rs:874-881; src/rune_ui/general.rs:79-95; src/control/settings.rs:253-256, 441 -->
