---
feature: notch-indicator
key: labs.notch_indicator
title: Notch indicator
for: [dictation]
ask: On a notched MacBook, show the recording indicator in the notch?
group: Dictation
os: macOS
vibehex: false
---

# Notch indicator

**What it does.** On a MacBook screen with a notch, the dictation indicator grows sideways out of the notch, like a Dynamic Island, instead of sitting below the menu bar. It shows a microphone with level bars while you speak, a spinner while VibeHex works, and a check mark when done.

**Turn it on when.** You dictate on a notched MacBook and want the indicator to look part of the Mac.

**Leave it off when.** You prefer the usual indicator.

**Needs.** macOS and a screen with a notch. Nothing else.

**Good to know.** It works with General › Advanced › Output › Dictation HUD set to Top or Bottom. Off still hides it. A screen without a notch, such as an external display, keeps the Top or Bottom indicator. It applies from the next dictation.

**Where.** Labs › Notch indicator (`labs.notch_indicator`). Related: General › Advanced › Output › Dictation HUD (`indicator_position`).

**Asked as.** "Can the indicator use the notch?"; "Move the recording indicator into the MacBook notch"; «Индикатор записи в вырезе экрана»; «Индикатор диктовки рядом с вырезом на MacBook».

<!-- sources: src/rune_ui/labs.rs:602-607; src/rune_hud/macos.rs:255-275; src/rune_hud/hud_placement.rs:278-290; src/rune_hud.wgsl:171-190; docs/features/labs.md:1333-1358; src/rune_ui/general.rs:934-950; src/rune_ui/general/search.rs:60-72; src/control/settings.rs:528-531, 807-810; src/labs.rs:464-490, 1258 -->
