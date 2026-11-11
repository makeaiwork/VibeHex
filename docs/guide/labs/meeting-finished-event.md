---
feature: meeting-finished-event
key: labs.meeting_finished_event
title: Meeting finished event
group: Meetings
os: every OS
vibehex: false
---

# Meeting finished event

**What it does.** Tells another app on this computer when a meeting's transcript is ready, so it can process the meeting. The app listens through VibeHex's local API and gets only the meeting's ID.

**Turn it on when.** A companion app or script should act on each finished meeting.

**Leave it off when.** No other app needs to know.

**Needs.** Meetings turned on, and an app that connects to the local API.

**Good to know.** It never sends the title, transcript, or audio; nothing leaves the computer. One app listens at a time.

**Where.** Labs › Meeting finished event (`labs.meeting_finished_event`).

**Asked as.** "Notify my app when a transcript is ready"; «Как узнать из скрипта, что встреча расшифрована».

<!-- sources: docs/features/labs.md:130-131, 1763-1795; docs/features/meetings.md:92-95; src/rune_ui/labs.rs:468-473; src/control/settings.rs:593-596, 907 -->
