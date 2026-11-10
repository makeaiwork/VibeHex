---
topic: meetings
title: Meetings
keys: [meetings.enabled, meetings.providers, meetings.calendar.connected, meetings.calendar.remind, meetings.calendar.lead_minutes, meetings.notes.writer, meetings.notes.effort, meetings.questions.enabled, meetings.questions.engine, meetings.retention, meetings.output_folder, labs.ai_cleanup_server, labs.live_chapters, labs.stop_when_call_ends]
os: macOS (beta; Windows and Linux have a reduced version)
---

# Meetings

Meetings records a call (the other people's audio and your microphone), transcribes it on this computer, and keeps a transcript of "You" and "Them" turns, plus notes. Beta, off by default.

## Turn it on

Switch on **Labs › Meetings (beta)**. Meetings joins the sidebar, with a header switch between Recordings and Settings. Press **Start recording**.

## Allow the audio

In **Meetings › Settings**, the **Screen & System Audio Recording** row has an Allow button. macOS asks once; if you answered before, System Settings opens with a guide. If it stays grey, press **Still grey? Fix it**. Recording needs this and the microphone. Windows and Linux ask nothing and have no calendar.

## Choose which calls are recorded

When Zoom, Google Meet, Microsoft Teams, Slack huddles, FaceTime, WhatsApp, Gather or Yandex Telemost uses the microphone for a couple of seconds (about 20 seconds for WhatsApp, Gather and Telemost), an offer appears: **Record** or **Not now**. Its "…" saves a choice for that app; change it in **Meetings › Settings › Detected calls**: **Ask**, **Always record** or **Never offer**. Always record never stops by itself unless **Labs › Stop when the call ends** is on.

## Connect your calendar

**Settings › Calendar › Connect calendar** asks macOS for access only when pressed. **Remind me before meetings** then reminds you at the start, or 1, 2 or 5 minutes before, and names the recording after the event.

## What a finished meeting has

Tabs: **Transcript**, **Summary** (notes, and chapters with **Labs › Live chapters**) and **Requests** (questions). The "…" menu regenerates the summary or deletes the meeting.

## Choose who writes the notes

**Settings › Notes › Written by**: Apple Intelligence by default (macOS 26 or later; a title and two-sentence summary, kept on this Mac). OpenCode writes full notes. Codex, Built in models, OpenRouter and Your server appear when **Labs › Clean up on your own server** is on and set up in Output › AI cleanup. Any writer other than Apple Intelligence receives the transcript.

## Ask questions about a meeting

With the same Lab on, **Settings › Questions › Ask about a meeting** adds a chat under each finished meeting. **Questions run on** picks the model; the default follows Files.

## Where recordings live

Audio and transcripts stay in VibeHex's data folder; a Markdown copy goes to `~/Documents/Hex Meetings` (**Advanced › Transcript folder** changes it). **Keep meetings**: 24 hours, 7 days, 30 days or Forever. New meetings default to 24 hours; expired ones go to the Trash.

**Asked as.** "Can it record my Zoom calls?"; "Where are my meeting notes?"; «Как записывать созвоны?»; «Почему не слышно собеседника в записи?»; «Сколько хранятся записи встреч?»

<!-- sources: docs/features/meetings.md:19-143, 147-200, 324-345, 392-420, 943-1010, 1113-1150, 1153-1170; src/rune_ui/platform_settings.rs:268-480, 817-870; src/rune_ui/meeting_notes.rs:41-300; src/rune_ui/labs.rs:187-262; src/app_settings/meeting_providers.rs:7; src/app_settings/meeting_folder.rs; src/meeting.rs:424 -->
