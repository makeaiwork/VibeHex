---
feature: terminal-paste-shortcut
key: labs.terminal_paste_shortcut
title: Terminal paste shortcut
group: Shortcuts
os: [Windows, Linux]
vibehex: false
---

# Terminal paste shortcut

**What it does.** Adds a second dictation shortcut that always pastes with Ctrl+Shift+V, which terminals accept. Your main shortcut keeps its usual paste, so one key suits terminals and one suits other apps.

**Turn it on when.** You dictate into terminals and ordinary apps and want a key for each.

**Leave it off when.** Ctrl+V works everywhere you dictate.

**Needs.** Windows or Linux. On Linux, X11 or a desktop with the shortcuts portal; where neither exists the key stays unregistered.

**Good to know.** The shortcut is unbound until you record a key, and it dictates like the main shortcut (same mode, History entry, Hold or Tap). It is refused if another Hex shortcut uses the same key. Paste last dictation still follows the Terminal paste setting. Off by default.

**Where.** Labs › Terminal paste shortcut (`labs.terminal_paste_shortcut`); then the Terminal paste shortcut row in General (`terminal_paste_hotkey`).

**Asked as.** "A separate shortcut for dictating into the terminal"; "Paste with Ctrl+Shift+V only for some apps"; «Отдельная клавиша для диктовки в терминал»; «Вставка через Ctrl+Shift+V по отдельной клавише».

<!-- sources: src/rune_ui/labs.rs:875-882; src/rune_ui/general.rs:79-96; src/labs.rs:478-480; src/app_settings.rs:1119-1128; src/linux/dictation/shortcuts.rs:24-26; src/control/settings.rs:253-257, 441-445; docs/features/dictation.md:384-408 -->
