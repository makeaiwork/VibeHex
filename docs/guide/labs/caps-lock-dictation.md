---
feature: caps-lock-dictation
key: labs.caps_lock_dictation
title: Caps Lock dictation
for: [dictation, code]
ask: Use Caps Lock as a one-press dictation key?
group: Shortcuts
os: macOS
vibehex: false
---

# Caps Lock dictation

**What it does.** Caps Lock becomes a dictation key: press once to start, speak hands-free, press again to finish and paste. Typing never turns to capitals. The key's light shows recording and blinks while transcribing.

**Turn it on when.** You never use Caps Lock and want one easy key for dictation.

**Leave it off when.** You type capitals with Caps Lock.

**Needs.** macOS. The light needs Input Monitoring; without it dictation still works.

**Good to know.** Escape cancels a Caps Lock dictation in every shortcut mode. Your usual shortcut keeps working when nothing records, and stands down while Caps Lock records. A press is ignored while a held shortcut, a double tap, or Annotate is recording. A Caps Lock remapped to Control or Escape doesn't start dictation. Turning the lab off cancels a running one. The card's Recording light picks Steady or Slow blink.

**Where.** Labs › Caps Lock dictation (`labs.caps_lock_dictation`), Recording light (`labs.caps_lock_light`).

**Asked as.** "Use Caps Lock to dictate"; "Start dictation with Caps Lock"; «Диктовка по Caps Lock»; «Как диктовать одной кнопкой без удержания».

<!-- sources: docs/features/labs.md:73, 640-742; src/rune_ui/labs.rs:540-545, 1045-1075 (row, Recording light with Steady/Slow blink); src/caps_lock_dictation.rs:1-22 (Input Monitoring, remapped key, no capitals), 50-70 (press ignored while Busy); src/recognition.rs:2176 (shortcut stands down, test), 306-315 (Escape cancels in every mode), 319-333, 530-534 (lab off cancels), 989 (not over Annotate); src/labs.rs:491, 1331-1342 (macOS only); src/labs/group.rs:112 (Shortcuts); src/app_settings/labs.rs:9-29; src/control/settings.rs:517-526 -->
