---
feature: special-keys
key: labs.standalone_special_keys
title: Special keys alone
for: []
ask: Allow special keys alone as your dictation shortcut?
group: Shortcuts
os: [Linux, Windows]
vibehex: false
---

# Special keys alone

**What it does.** Allows keys that VibeHex normally refuses on their own to be
a dictation shortcut. On Linux these are Menu, Pause, and Scroll Lock, for
example Caps Lock remapped to Menu so it can be held to dictate on GNOME
Wayland. On Windows (Menu, Pause, and Scroll Lock already work) they are
Insert, Home, End, Page Up, and Page Down.

**Turn it on when.** You want one of those keys as a plain shortcut.

**Leave it off when.** You use ordinary shortcuts. Off, Linux refuses Menu,
Pause, or Scroll Lock alone, and Windows refuses Insert, Home, End, Page Up,
or Page Down alone; a saved one goes back to its default. Windows takes Menu,
Pause, and Scroll Lock alone either way.

**Needs.** Linux or Windows. On Linux, the window's recorder can't capture
Menu, Pause, or Scroll Lock, so set it from the terminal, for example `hex
settings set dictation_hotkey menu`.

**Where.** Labs › Special keys alone (`labs.standalone_special_keys`), shown
on Linux; on Windows, turn it on with `hex labs on special-keys`.

**Asked as.** "Use Caps Lock or Pause as the dictation key"; "Menu key as
shortcut on Linux"; «Использовать Pause или Scroll Lock как клавишу диктовки»;
«Caps Lock как кнопка диктовки в GNOME».

<!-- sources: docs/features/labs.md:69,534-570; src/labs.rs:294-298,528-533; src/rune_ui/labs.rs:403-415 (row on Linux only); src/rune_ui/settings.rs:300-372 (recorder has no Menu/Pause/Scroll Lock/Insert/Page keys); src/app_settings/shortcut_repair.rs:74-140 -->
