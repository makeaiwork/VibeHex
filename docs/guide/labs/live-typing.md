---
feature: live-typing
key: labs.live_typing
title: Live typing
for: []
ask: Type dictations straight into the field on Sway or Hyprland?
group: Dictation
os: Linux
vibehex: false
---

# Live typing

**What it does.** While you speak, the words heard so far appear underlined in the focused text field. When you let go, the final text is typed into the field instead of being pasted through the clipboard, so the clipboard stays untouched.

**Turn it on when.** You use Sway, Hyprland, or another wlroots desktop on Wayland.

**Leave it off when.** You use GNOME or KDE, or an input method such as fcitx or ibus: it does nothing there and dictation pastes as usual.

**Needs.** Linux on a Wayland session; the row appears nowhere else. The words while speaking need a streaming speech model running on the graphics chip. With a processor-only or non-streaming model, only the final text arrives.

**Good to know.** It works with Live text off. In Hold mode, short taps and shortcut chords don't start it; in Tap mode the press does. A result with more than one line, or an empty one, is pasted as before. It is off by default.

**Where.** Labs › Live typing (`labs.live_typing`), in the Dictation group.

**Asked as.** "Type the words into the field while I speak"; "Show dictation underlined as I talk"; «Текст прямо в поле во время диктовки»; «Слова появляются в поле по ходу речи».

<!-- sources: src/rune_ui/labs.rs:406-413; src/labs.rs:473-477, 1090-1097; src/labs/group.rs:56; src/linux/dictation.rs:1418-1450; src/linux/live_typing.rs:61-62, 724-734, 541; src/control/settings.rs:355-359; docs/features/dictation.md:564-630 -->
