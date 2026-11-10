---
feature: keep-recordings
key: labs.keep_recordings
title: Keep recordings
group: History and recording
os: every OS
vibehex: false
---

# Keep recordings

**What it does.** Saves the audio of each dictation along with its History entry. Open the entry, choose Transcribe again, pick another speech model you have installed, and get a second version of the text. You can copy it or replace the entry's text.

**Turn it on when.** You try different speech models, or want a second chance when a name came out wrong.

**Leave it off when.** You don't want voice recordings stored on the computer. Nothing is kept by default.

**Needs.** History turned on. Another installed speech model, if you want to compare results.

**Good to know.** Only dictations made after you turn it on have audio. Recordings are deleted with their entry, on Delete or Clear all, or when retention removes it. Past about 512 MB of audio, the oldest entries lose their audio but keep their text. Turning the switch off saves nothing new, hides Transcribe again, and keeps existing audio until it expires.

**Where.** Labs › Keep recordings (`labs.keep_recordings`).

**Asked as.** "Can I redo a dictation with another model?"; "Transcribe again"; «Как перераспознать запись другой моделью»; «Хранятся ли аудиозаписи диктовок».

<!-- sources: docs/features/labs.md:79, 790-834; src/history/recordings.rs:24; src/rune_ui/history.rs:1-5, 721, 746; src/rune_ui/labs.rs:465-470 -->
