---
feature: transcribe-files
key: labs.transcribe_files
title: Transcribe files
group: Models
os: every OS
vibehex: false
---

# Transcribe files

**What it does.** Adds the terminal command `hex transcribe` with a file name: it prints a WAV recording's text, made with your dictation model or one you name.

**Turn it on when.** You want transcripts of saved recordings in scripts or the terminal.

**Leave it off when.** You don't use the terminal.

**Needs.** A WAV file up to 64 MiB; convert others first, for example with ffmpeg. On a Mac, VibeHex must be running.

**Good to know.** Nothing is pasted, saved to History, or cleaned up. Stable VibeHex builds don't list it; only dev builds do.

**Where.** Labs › Transcribe files (`labs.transcribe_files`).

**Asked as.** "Transcribe a WAV file from the command line"; «Как расшифровать аудиофайл из терминала».

<!-- sources: docs/cli.md:41, 145-162; docs/features/labs.md:35-40, 98-109; src/labs.rs:264-275; src/rune_ui/labs.rs:760-765; src/control/cli/transcribe.rs:62 -->
