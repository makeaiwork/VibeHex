---
feature: subtitle-export
key: labs.subtitle_export
title: Subtitle export
for: []
ask: Export meeting transcripts as subtitle files from the terminal?
group: Meetings
os: [macOS, Windows, Linux]
vibehex: false
---

# Subtitle export

**What it does.** Lets you turn a recorded meeting into a subtitle file (SRT
or VTT) that a video player or editor can load. Each speaker turn becomes one
timed line starting with You or Them.

**Turn it on when.** You have a call recording and want subtitles for it, or
need timed text for a video editor.

**Leave it off when.** You only read transcripts in the Meetings pane. This
switch adds nothing to the window.

**Needs.** Meetings, with at least one recording, and the Terminal: the
export is a command, `hex meetings show <meeting> --srt` or `--vtt`, which
prints the subtitles to save into a file. Turn the switch on first, otherwise
the command refuses and names the switch. On Windows and Linux, switch it
in the window: `hex labs` and `hex settings` refuse it there.

**Good to know.** Times come from each turn's start and end, not from single
words, so a cue can be a whole sentence or longer. There is no export button
in the window.

**Where.** Labs › Subtitle export (`labs.subtitle_export`).

**Asked as.** "Can I export a meeting as subtitles?"; "SRT from a call
recording"; «Как сохранить расшифровку встречи в субтитры»; «Экспорт в SRT или
VTT».

<!-- sources: docs/features/labs.md:294-309; docs/transcript-cli.md:104-116; src/control/transcript_cli.rs:595-600; src/labs.rs:503-505 -->
