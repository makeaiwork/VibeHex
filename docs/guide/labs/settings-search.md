---
feature: settings-search
key: labs.settings_search
title: Settings search
for: []
ask: Add a search field that finds any setting?
group: Dictation
os: every OS
vibehex: false
---

# Settings search

**What it does.** Puts a Search settings field at the top of General. Type a word and only the rows whose title or description contain it stay, ignoring case.

**Turn it on when.** You can't remember which section a setting is in.

**Leave it off when.** Nothing to lose; off, General is as before.

**Needs.** Nothing else.

**Good to know.** Rows under Advanced match without opening it. Permission and error cards always stay. If nothing matches, General says No settings match. It searches General only, and leaving General clears the field. ⌘F in General jumps to the field (Ctrl+F on Windows and Linux).

**Where.** Labs › Settings search (`labs.settings_search`).

**Asked as.** "How do I find a setting?"; "Search in settings"; «Поиск по настройкам»; «Как быстро найти нужную настройку».

<!-- sources: src/rune_ui/labs.rs:414-419; src/rune_ui/general/search.rs:21-96, 98-112, 114-137, 256-263; src/rune_ui.rs:1137; src/labs/group.rs:56; src/control/settings.rs:360-363 -->
