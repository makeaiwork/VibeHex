---
feature: skip-pauses
key: labs.skip_pauses
title: Skip long pauses
group: Dictation
os: [Windows, Linux]
vibehex: false
---

# Skip long pauses

**What it does.** On a slow processor, silence costs time: when a dictation
is transcribed as a whole recording, every second of a pause goes through the
speech model. With this on, each long pause is shortened first, so text
appears sooner.

**Turn it on when.** Your computer is slow (an older processor or integrated
graphics) and you pause while you think during a dictation.

**Leave it off when.** Your dictations already paste quickly.

**Needs.** Windows or Linux. Not shown on macOS.

**Good to know.** Each pause keeps its first 2.5 seconds and last 0.3 seconds;
a pause at the end keeps its first 2.5 seconds; shorter pauses are untouched.
It applies only to dictations transcribed as a whole recording (a model that
does not write while you talk, or a live stream that fell back). Meetings,
live streams, and the local API are unchanged. History still shows the full duration.
If nothing in the recording rises above background noise, it is left whole.

**Where.** Labs › Skip long pauses (`labs.skip_pauses`).

**Asked as.** "Dictation is slow on my old laptop"; "Cut silence before
transcribing"; «Диктовка долго обрабатывается на слабом компьютере»; «Как
убрать паузы перед распознаванием».

<!-- sources: docs/features/labs.md:654-680; docs/features/dictation.md:266-283; src/labs.rs:296-298, 540-542; src/rune_ui/labs.rs:419-427; src/transcription/engine/streaming.rs:76-78, 254-260; src/transcription/engine.rs:664-675 -->
