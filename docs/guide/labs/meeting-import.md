---
feature: meeting-import
key: labs.meeting_import
title: Import audio
for: []
ask: Import an audio file into Meetings to transcribe it?
group: Meetings
os: macOS
vibehex: false
---

# Import audio

**What it does.** Adds an **Import audio…** button beside Start recording in Meetings. Pick a WAV, M4A, MP3, AIFF, or CAF file. VibeHex transcribes it on this Mac with your selected speech model and keeps it as a meeting, named after the file.

**Turn it on when.** You have a recording made elsewhere, such as a call another app captured.

**Leave it off when.** You only record meetings in VibeHex.

**Needs.** macOS, Labs › Meetings (beta) on, and your selected speech model installed.

**Good to know.** The button shows only while nothing records, and one import runs at a time. Turns say Speaker, not You and Them. Notes and a title are written afterward, as for a recording, if Meetings is set to write them. A file that can't be read shows "Audio could not be imported." Only dev builds of VibeHex list this lab.

**Where.** Labs › Import audio (`labs.meeting_import`).

**Asked as.** "Transcribe an audio file as a meeting"; "Can I import a voice recording?"; «Как загрузить аудиозапись встречи»; «Расшифровать готовый файл с записью».

<!-- sources: src/rune_ui/labs.rs:486-493; src/rune_ui/meetings.rs:540-560, 713; src/meeting/import.rs:36-60, 112, 158-174; src/meeting/watcher.rs:676-760; src/labs.rs:271, 467 (on_this_os, vibehex_hides), 1318; src/labs/group.rs:98; src/rune_ui/meetings.rs:158-161, 1348-1356; src/control/settings.rs:633-636 -->
