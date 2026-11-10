---
feature: hide-menu-bar-icon
key: labs.hide_menu_bar_icon
title: Hide menu bar icon
group: Sound and interface
os: macOS
vibehex: false
---

# Hide menu bar icon

**What it does.** Removes VibeHex's icon from the Mac menu bar, to give the space back when you only use the shortcut.

**Turn it on when.** Your menu bar is crowded and you start dictation from the keyboard.

**Leave it off when.** You use the menu for Paste Last Dictation, Settings, or Quit, or want a visible sign that VibeHex is running.

**Needs.** macOS. Nothing else.

**Good to know.** VibeHex keeps running and shortcuts work as before. The icon comes back while a meeting is recording or being transcribed, so you can stop it, and hides again after. To open Settings with the icon hidden, open VibeHex again from Finder or Spotlight. It applies at once, and off restores the icon.

**Where.** Labs › Hide menu bar icon (`labs.hide_menu_bar_icon`).

**Asked as.** "Remove the VibeHex icon from the menu bar"; "I hid the icon, how do I open Settings?"; «Как убрать иконку из строки меню»; «Пропала иконка VibeHex, где настройки».

<!-- sources: docs/features/labs.md:80, 836-852; src/rune_ui/labs.rs:388-396; src/macos_rune/status_item.rs:540-544, 658-662; src/labs.rs:987-992 -->
