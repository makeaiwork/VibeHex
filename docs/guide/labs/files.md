---
feature: files
key: labs.files
title: Files
group: History and recording
os: macOS
vibehex: true
---

# Files

**What it does.** Turns recordings you already have into text. Drop an audio
or video file on the Files pane (or choose one) and VibeHex transcribes it on
this Mac, can tidy the text with AI cleanup, and lets you ask for more: a
summary, an outline, action items, or your own request. Results sit in three tabs: Original, Cleanup, and Requests.

**Turn it on when.** You have voice memos, interviews, lectures, or videos to
read instead of listen to.

**Leave it off when.** You only dictate. Off, there is no Files pane.

**Needs.** macOS. A speech model, the same as dictation's or one you pick in
Files › Settings. Cleanup and requests need an AI engine set up in Output ›
AI cleanup, or a model built into VibeHex.

**Good to know.** Common formats work: mp3, m4a, wav, flac, mp4, mov and more.
VibeHex keeps only the file's location, never a copy, and never changes the
file. The queue does not survive quitting VibeHex.

**Where.** Labs › Files (`labs.files`); then the Files pane.

**Asked as.** "Can it transcribe an audio file?"; "Transcribe a video from
disk"; «Как расшифровать аудиозапись или видео»; «Загрузить файл и получить
текст».

<!-- sources: docs/features/files.md:1-50, 59-112, 113-135; docs/features/labs.md:90; src/labs.rs:586-588, 312-313; src/rune_ui/labs.rs:303-309 -->
