---
feature: tap-or-hold
key: labs.tap_or_hold
title: Tap or hold
group: Shortcuts
os: macOS
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

**Needs.** macOS, and General › Shortcut mode set to Tap.

**Good to know.** It only applies in Tap mode; in other modes nothing
changes. A shortcut pressed together with other keys, as a chord meant for
another app, is still discarded as before. A hold ends with the usual short
tail of audio after you let go. A short tap behaves as plain Tap.

**Where.** Labs › Tap or hold (`labs.tap_or_hold`). Related: General ›
Shortcut mode.

**Asked as.** "Both push-to-talk and tap with the same key"; "Hold the
shortcut to dictate, tap to toggle"; «Одна клавиша и для нажатия, и для
удержания»; «Диктовка пока держу клавишу».

<!-- sources: docs/features/labs.md:65,416-436; src/suppression/hotkey.rs:117,806-840; src/rune_ui/shortcut_mode.rs:34-40,70-84,124 (UI names Toggle "Tap"); src/rune_ui/labs.rs:370-380 -->
