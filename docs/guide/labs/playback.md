---
feature: playback
key: labs.playback
title: Listen to the recording
group: Meetings
os: macOS
vibehex: true
---

# Listen to the recording

**What it does.** Lets you play the sound from a transcript. Press Play on a
finished meeting or a file in Files and a small player opens inside the card.
The word being spoken lights up, and clicking a line plays from there, so you
can check what was really said.

**Turn it on when.** You spot a wrong or unclear passage in a transcript and
want to hear it.

**Leave it off when.** You never need the sound.

**Needs.** macOS, with the Meetings or Files pane turned on. The audio itself:
a file must still be at its original location.

**Good to know.** The player has Pause, Back 5 seconds, a position bar, and a
speed from 0.75x to 2x at the voice's natural pitch. Space plays or pauses,
the arrow keys skip 5 seconds. Exact word highlighting needs a speech model
that times words (Parakeet or GigaAM); with others the word is estimated
within its line. Unreadable audio shows "Couldn't play it".

**Where.** Labs › Listen to the recording (`labs.playback`).

**Asked as.** "Can I play the recording from the transcript?"; "Play audio of
a meeting"; «Как прослушать запись встречи»; «Включить звук по тексту».

<!-- sources: docs/features/files.md:113-160; docs/features/meetings.md:1064-1111; docs/features/labs.md:91; src/labs.rs:595-597, 312-313; src/rune_ui/labs.rs:310-316 -->
