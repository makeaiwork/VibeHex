---
feature: section-key
key: labs.section_key_alone
title: Section key alone
for: []
ask: Use the § key alone as your dictation shortcut?
group: Shortcuts
os: Linux
vibehex: false
---

# Section key alone

**What it does.** Lets the § key, left of 1 on ISO keyboards, be a shortcut
on its own with no modifier, for example for push-to-talk. Normally VibeHex
asks you to add a modifier.

**Turn it on when.** You have an ISO keyboard and want § as a dictation
key.

**Leave it off when.** You type § or don't want to lose it. While bound, §
is taken from every app, so you need another key to type it.

**Needs.** Linux. Nothing else.

**Good to know.** Off, a lone § is refused ("Add a modifier") and a saved one
goes back to its default; § with a modifier works as before. It works on X11
and through the Wayland shortcuts portal. From the terminal use `hex settings
set dictation_hotkey §`.

**Where.** Labs › Section key alone (`labs.section_key_alone`). Then record
it in General › Shortcut.

**Asked as.** "Use the section sign as the shortcut"; "§ key as push-to-talk
on Linux"; «Клавиша § как кнопка диктовки»; «Назначить § без модификатора».

<!-- sources: docs/features/labs.md:70,572-600; src/rune_ui/labs.rs:411-417; src/rune_ui/settings.rs:326-329,358-362; src/app_settings/shortcut_repair.rs:83-101 -->
