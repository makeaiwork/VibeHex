---
feature: annotate-replay
key: labs.annotate_replay
title: Annotate 15 s replay
for: []
ask: Keep the last 15 seconds of your screen, ready to save?
group: Annotate
os: macOS
vibehex: false
---

# Annotate 15 s replay

**What it does.** Keeps the last 15 seconds of your screen, so you can save what just happened. Once you set a shortcut, a tap saves the clip; holding it for about a third of a second or more also records your voice over it. A shorter tap saves a silent clip.

**Turn it on when.** You want to show a glitch that already passed.

**Leave it off when.** macOS's screen-recording dot, lit the whole time this is on, bothers you.

**Needs.** macOS 27 or later, Annotate on, and Screen Recording permission.

**Good to know.** The replay shortcut starts empty: set it in Annotate › Shortcuts › Replay last 15 s. It watches the display under the pointer and stops while the Scrawl app is running. Stable VibeHex builds don't list this lab; only dev builds do.

**Where.** Labs › Annotate 15 s replay (`labs.annotate_replay`). Related: Annotate › Shortcuts › Replay last 15 s (`capture.replay_hotkey`).

**Asked as.** "Save what just happened on screen"; "Record the last 15 seconds of my screen"; «Сохранить последние 15 секунд экрана»; «Записать то, что только что произошло на экране».

<!-- sources: docs/features/labs.md:37-38; src/rune_ui/labs.rs:617-625; src/rune_ui/scrawl.rs:204-249 (Shortcuts section, Replay last 15 s row, hidden unless lab on); src/labs.rs:274, 450-452, 1312 (vibehex_hides, key map, accessor); src/labs/group.rs:102; src/capture_host/replay.rs:1-13, 57-60 (300 ms narration hold), 97-109 (macOS 27), 164-171 (needs Annotate on, no Scrawl app running), 617-635 (Screen Recording); src/app_settings/capture.rs:76 (replay_hotkey default none); src/control/settings.rs:603-606, 832-835 -->
