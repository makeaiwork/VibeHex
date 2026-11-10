---
feature: meetings
key: meetings.enabled
title: Meetings (beta)
group: Meetings
os: macOS
vibehex: false
---

# Meetings (beta)

**What it does.** Records a call and keeps a written transcript, so you can
find what was said later. VibeHex records the other people (the computer's
sound) and you (the microphone) as two tracks, transcribes both on this
computer, and saves turns labelled You and Them. Turning it on adds a Meetings
pane to the sidebar.

**Turn it on when.** You want transcripts of Zoom, Meet, Teams, FaceTime,
WhatsApp, Slack, or Telemost calls. VibeHex offers to record when it notices
a call, and starts by itself only for apps set to Always record.

**Leave it off when.** You never record calls. Off, nothing watches the
microphone for calls and the pane is hidden.

**Needs.** macOS with the Microphone and System Audio Recording permissions
(the Meetings pane asks for the second). A speech model on this Mac.

**Good to know.** Linux and Windows show the same switch as a beta, with no
permissions asked. Recording never stops by itself unless Stop when the call
ends is on.

**Where.** Labs › Meetings (beta) (`meetings.enabled`); then the Meetings pane.

**Asked as.** "Can it record my Zoom calls?"; "Where do I turn on meeting
transcripts?"; «Как записывать созвонки»; «Как включить расшифровку встреч».

<!-- sources: docs/features/meetings.md:1-12, 13-76, 78-116; docs/features/labs.md:41, 1389-1391; src/rune_ui/labs.rs:186-192; src/rune_ui/platform.rs:72,90,108; src/labs.rs:287-292 -->
