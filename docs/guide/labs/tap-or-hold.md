---
feature: tap-or-hold
key: labs.tap_or_hold
title: Tap or hold
for: []
ask: In Tap mode, also hold the shortcut to talk?
group: Shortcuts
os: [macOS, Linux]
vibehex: false
---

# Tap or hold

**What it does.** Lets one dictation shortcut work two ways. A quick tap starts
dictation and the next tap finishes it. If you hold the shortcut for about
0.3 seconds or longer, VibeHex dictates until you let go, like push-to-talk.

**Turn it on when.** You use the Tap shortcut mode but sometimes just want to
say a short phrase while holding the key.

**Leave it off when.** You use Hold, which already works that way, or you
like Tap exactly as it is.

**Needs.** macOS or Linux (not Windows), and General › Shortcut mode set to Tap.

**Good to know.** It only applies in Tap mode. It covers the dictation
shortcut and Dictate without processing; on Linux, `hex dictation` commands are
unchanged. On a Mac, a chord meant for another app is still discarded as
before. A hold ends with the usual short tail of audio after you let go.

**Where.** Labs › Tap or hold (`labs.tap_or_hold`). Related: General ›
Shortcut mode.

**Asked as.** "Both push-to-talk and tap with the same key"; "Hold the
shortcut to dictate, tap to toggle"; «Одна клавиша и для нажатия, и для
удержания»; «Диктовка пока держу клавишу».

<!-- sources: docs/features/labs.md:76,790-820; src/labs.rs:495; src/suppression/hotkey.rs:117-121,832-846; src/dictation.rs:93; src/linux/input.rs:249-252,856-870,2251-2270; src/linux/dictation/shortcuts.rs:14-17; src/rune_ui/labs.rs:572-580,640-648; src/rune_ui/shortcut_mode.rs:34-60 (UI names Toggle "Tap") -->
