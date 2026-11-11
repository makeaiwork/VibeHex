---
feature: live-typing
key: labs.live_typing
title: Live typing
group: Dictation
os: Linux
vibehex: false
---

# Live typing

**What it does.** While you speak, the words heard so far appear underlined in the focused field. When you let go, the final text is typed there instead of pasted through the clipboard.

**Turn it on when.** You use Sway, Hyprland, or another wlroots desktop.

**Leave it off when.** You use GNOME, KDE, fcitx, or ibus: it does nothing there.

**Needs.** Linux on Wayland, the only place the row appears. Words while speaking need a streaming model on the graphics chip; otherwise only the final text arrives.

**Good to know.** It works with Live text off.

**Where.** Labs › Live typing (`labs.live_typing`).

**Asked as.** "Type the words into the field while I speak"; «Текст прямо в поле во время диктовки».

<!-- sources: docs/features/labs.md:50, 273; docs/features/dictation.md:564-630, 1077; src/rune_ui/labs.rs:406-413; src/labs.rs (on_this_os) -->
