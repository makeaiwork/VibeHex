---
feature: cleanup-progress
key: labs.cleanup_progress
title: Cleanup progress
group: Output
os: macOS
vibehex: true
---

# Cleanup progress

**What it does.** A long dictation can take a while to clean up with AI, and
nothing shows what is happening. With this on, a small bar in place of the
HUD says "Cleaning up" and shows how far it has got, with a Paste now button.

**Turn it on when.** You dictate long texts, or use a slow cleanup model, and
want to see progress or not wait.

**Leave it off when.** Your cleanups are short. The bar appears only when
cleanup will plainly take a while, never for a quick one.

**Needs.** macOS and AI cleanup turned on.

**Good to know.** Paste now stops the cleanup and pastes the part already
cleaned, then the rest as transcribed. With a server engine the bar moves
smoothly; with OpenCode it moves one piece at a time (a piece is about three
minutes of speech). The bar never takes focus, so the text still lands in
your app. Marked with a star: added in VibeHex.

**Where.** Labs › Cleanup progress (`labs.cleanup_progress`).

**Asked as.** "Cleanup takes forever, can I skip it?"; "Paste now button";
«Как не ждать окончания чистки текста»; «Что за полоска Cleaning up».

<!-- sources: docs/features/labs.md:89, docs/features/labs.md:1010-1060; src/labs.rs:311, 345; src/rune_ui/labs.rs:316-322; src/dictation/cleanup_pieces.rs:21-30, 70-72, 374-385; src/rune_ui/cleanup_plate.rs:168-195 -->
