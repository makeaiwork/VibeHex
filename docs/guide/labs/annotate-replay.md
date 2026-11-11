---
feature: annotate-replay
key: labs.annotate_replay
title: Annotate 15 s replay
group: Annotate
os: macOS
vibehex: false
---

# Annotate 15 s replay

**What it does.** Keeps the last 15 seconds of your screen, so you can save what just happened. A tap of the replay shortcut saves the clip; holding it also records your voice over it.

**Turn it on when.** You want to show a glitch that already passed.

**Leave it off when.** macOS's screen-recording dot, lit the whole time this is on, bothers you.

**Needs.** macOS 27 or later, Annotate on, and Screen Recording permission.

**Good to know.** Stable VibeHex builds don't list this lab; only dev builds do.

**Where.** Labs › Annotate 15 s replay (`labs.annotate_replay`). Related: Annotate › Replay last 15 s (`capture.replay_hotkey`).

**Asked as.** "Save what just happened on screen"; «Сохранить последние 15 секунд экрана».

<!-- sources: docs/features/labs.md:37-38, 185-187, 1647-1660; docs/features/capture.md:907-935; src/rune_ui/labs.rs:617-625; src/rune_ui/scrawl.rs:218-246; src/labs.rs (vibehex_hides, on_this_os); src/control/settings.rs:603-606, 832-835 -->
