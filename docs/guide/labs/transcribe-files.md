---
feature: transcribe-files
key: labs.transcribe_files
title: Transcribe files
for: []
ask: Transcribe WAV recordings from the terminal with hex transcribe?
group: Models
os: every OS
vibehex: false
---

# Transcribe files

**What it does.** Adds the terminal command `hex transcribe <file>`. It prints the text of a WAV recording, made with your dictation model or one you name with `--model` (and a language with `--language`).

**Turn it on when.** You want transcripts of saved recordings in scripts or the terminal.

**Leave it off when.** You don't use the terminal. With the lab off the command refuses and says how to turn it on (`hex labs on transcribe-files`).

**Needs.** A WAV file up to 64 MiB; convert others first, for example with ffmpeg. On a Mac, VibeHex must be running. On Linux and Windows the command loads the model itself when VibeHex is stopped.

**Good to know.** Nothing is pasted, saved to History, or cleaned up. Only dev builds of VibeHex list this lab. Off by default.

**Where.** Labs › Transcribe files (`labs.transcribe_files`).

**Asked as.** "Transcribe a WAV file from the command line"; "Convert audio to text in the terminal"; «Как расшифровать аудиофайл из терминала»; «Транскрибировать запись командой hex».

<!-- sources: src/rune_ui/labs.rs:762-766; src/labs.rs:259-277; src/labs/group.rs:114-122; src/control/cli/transcribe.rs:1-4, 56-88, 112-135; src/local_api.rs:50; docs/cli.md:145-162; src/control/settings.rs:666-670 -->
