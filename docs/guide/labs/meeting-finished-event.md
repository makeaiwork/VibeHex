---
feature: meeting-finished-event
key: labs.meeting_finished_event
title: Meeting finished event
group: Meetings
os: every OS
vibehex: false
---

# Meeting finished event

**What it does.** Tells another app on this computer when a meeting's transcript is ready, so it can process the meeting. The app connects to VibeHex's local API and receives only the meeting's ID.

**Turn it on when.** A companion app or script should act on each finished meeting.

**Leave it off when.** No other app needs to know.

**Needs.** Meetings (on macOS, Meetings (beta) turned on in Labs), and an app that connects to the local API with its access token.

**Good to know.** It never sends the title, transcript, path or audio, and nothing leaves the computer. Only one app can listen at a time; a second one is turned away until the first leaves. While the lab is off the connection is refused, and an open one ends. It is off by default.

**Where.** Labs › Meeting finished event (`labs.meeting_finished_event`), in the Meetings group.

**Asked as.** "Notify my app when a transcript is ready"; "Webhook when a meeting finishes"; «Как узнать из скрипта, что встреча расшифрована»; «Уведомление другой программе о готовой расшифровке».

<!-- sources: src/rune_ui/labs.rs:468-473; src/labs.rs:1290-1298; src/meeting/finished_events.rs:1-12, 28-60; src/local_api/meeting_events.rs:1-55; src/control/settings.rs:592-596, 907; src/labs/group.rs:94; docs/features/labs.md:1763-1795 -->
