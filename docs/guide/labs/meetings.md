---
feature: meetings
key: meetings.enabled
title: Meetings (beta)
for: []
ask: Record calls and keep written transcripts of them?
group: Meetings
os: [macOS, Windows, Linux]
vibehex: false
---

# Meetings (beta)

**What it does.** Records a call and keeps a written transcript. VibeHex records the other people (the computer's
sound) and you (the microphone) as two tracks, transcribes both on this
computer, and saves turns labelled You and Them. Turning it on adds a Meetings
pane to the sidebar.

**Turn it on when.** You want transcripts of Zoom, Meet, Teams, FaceTime (Mac),
WhatsApp, Slack, Discord, Gather, or Telemost calls; Labs › More call apps adds
Signal, Telegram, and more. VibeHex offers to record when it notices a call,
and starts by itself only for apps set to Always record.

**Leave it off when.** You never record calls. Off, nothing watches the
microphone for calls and the pane is hidden.

**Needs.** A speech model on this computer. macOS also needs the Microphone and
System Audio Recording permissions (the Meetings pane asks for the second).
Windows asks for none; Linux needs PipeWire's pw-record.

**Good to know.** Recording never stops by itself unless Stop when the call
ends is on, a macOS-only lab; on Windows and Linux you stop it yourself. There,
browser calls are offered as "Call in" the browser.

**Where.** Labs › Meetings (beta) (`meetings.enabled`); then the Meetings pane.
On Windows and Linux, `hex labs on meetings` is refused; use
`hex settings set meetings.enabled on`.

**Asked as.** "Can it record my Zoom calls?"; "Where do I turn on meeting
transcripts?"; «Как записывать созвонки»; «Как включить расшифровку встреч».

<!-- sources: docs/features/meetings.md:1-12,100-116,128-170; src/rune_ui/labs.rs:360-367,426-447,478-485; src/rune_ui/platform.rs:72-118; src/labs.rs:460-467; src/labs/cli.rs:106-120; src/control/settings.rs:679-682,945-955; src/app_settings/meetings.rs:253-267; src/microphone_activity.rs:281-290,296-300; src/meeting/linux.rs:27-35; src/meeting/watcher.rs:887; src/meeting/detection.rs:229; src/rune_ui/platform_settings.rs:944-977 -->
