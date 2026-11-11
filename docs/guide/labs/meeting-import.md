---
feature: meeting-import
key: labs.meeting_import
title: Import audio
group: Meetings
os: macOS
vibehex: false
---

# Import audio

**What it does.** Adds **Import audio…** beside Start recording in Meetings. Pick a WAV, M4A, MP3, AIFF, or CAF file; VibeHex transcribes it on this Mac with your selected speech model and keeps it as a meeting.

**Turn it on when.** You have a recording made elsewhere.

**Leave it off when.** You only record meetings in VibeHex.

**Needs.** macOS and Meetings turned on.

**Good to know.** The button shows only while nothing records. Turns say Speaker, not You and Them. Stable VibeHex builds don't list this lab; only dev builds do.

**Where.** Labs › Import audio (`labs.meeting_import`).

**Asked as.** "Transcribe an audio file as a meeting"; «Как загрузить аудиозапись встречи».

<!-- sources: docs/features/labs.md:37-38, 137-139, 2471-2488; docs/features/meetings.md:103-110, 750-785; src/rune_ui/labs.rs:486-493; src/rune_ui/meetings.rs:540; src/labs.rs (vibehex_hides); src/control/settings.rs:633-636 -->
