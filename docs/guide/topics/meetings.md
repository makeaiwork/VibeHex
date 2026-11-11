---
topic: meetings
title: Meetings
keys: [meetings.enabled, meetings.summaries, meetings.opencode_model, meetings.providers, meetings.calendar.connected, meetings.calendar.remind, meetings.calendar.lead_minutes, meetings.retention, meetings.output_folder, labs.ai_cleanup_server, labs.live_chapters, labs.stop_when_call_ends, labs.more_call_apps]
os: macOS, Windows, and Linux (beta; Windows and Linux have a reduced version)
---

# Meetings

Meetings records a call (the other people's audio and your microphone), transcribes it on this computer, and keeps a transcript of "You" and "Them" turns, plus notes. Beta, off by default.

## Turn it on

Switch on **Labs › Meetings (beta)**; the row is there on macOS, Windows, and Linux. Meetings joins the sidebar, with a header switch between Recordings and Settings. Press **Start recording**. Windows and Linux record too, but as a beta with no permission prompts and no calendar. Some meeting labs exist only on a Mac: **Stop when the call ends**, **Movable meeting capsules** and **Import audio**.

## Allow the audio

On a Mac, the **Screen & System Audio Recording** row in **Meetings › Settings** has an Allow button. macOS asks once; if you answered before, System Settings opens with a guide. If it stays grey, press **Still grey? Fix it**. Once allowed, the row moves under Advanced. Recording needs this and the microphone.

## Choose which calls are recorded

When Zoom, Google Meet, Microsoft Teams, Slack huddles, FaceTime, WhatsApp, Gather, Discord or Yandex Telemost uses the microphone for a couple of seconds (about 20 seconds for WhatsApp, Gather and Telemost), an offer appears: **Record** or **Not now**. Its "…" saves a choice for that app; change it in **Meetings › Settings › Detected calls**: **Ask**, **Always record** or **Never offer**. Always record never stops by itself unless **Labs › Stop when the call ends** is on (Mac only).

**Labs › More call apps** adds Signal, Telegram, WeChat, LINE, Webex, Element and the Phone app to Detected calls, and, on a Mac, calls on more websites (Google Voice, Messenger, GroupMe). Messaging apps wait about 20 seconds, since voice notes use the microphone too.

## Connect your calendar

On a Mac, **Settings › Calendar › Connect calendar** asks macOS for access only when pressed. **Remind me before meetings** then reminds you at the start, or 1, 2 or 5 minutes before, and names the recording after the event.

## What a finished meeting has

Tabs: **Transcript**, **Summary** (once there are notes; chapters come with **Labs › Live chapters**) and **Requests** (questions, with **Labs › Clean up on your own server** and Settings › Questions › Ask about a meeting on). The "…" menu regenerates the summary or deletes the meeting.

A question in Requests that reaches past the meeting ("what did we decide at the previous meeting?") also reads your other meetings, Files texts and dictations once you allow it, citing them with [n] marks that open the place; **Back to Requests** returns to the session.

## Choose who writes the title and summary

**Settings › After a meeting › Summaries** is a pop-up on a Mac: **Off**, **On this Mac** (Apple Intelligence, the default; macOS 26 or later; nothing leaves the Mac) or **OpenCode** (sends the transcript to your OpenCode model). With OpenCode, a **Model** row appears; left unset, VibeHex picks a fast model. OpenCode must be running. On Windows and Linux Summaries is a switch, off by default, and on means OpenCode. With Off, a meeting gets no title or summary on its own.

**Settings › Notes › Written by** can pick another writer; while it does, Summaries is hidden. Default is Apple Intelligence on a Mac and OpenCode (when turned on) elsewhere; OpenCode is always listed. Codex, Built in and Other appear when **Labs › Clean up on your own server** is on and set up in Output › AI cleanup. Apple Intelligence and Built in keep the transcript on the computer; any other writer receives it.

## Ask questions about a meeting

With **Labs › Clean up on your own server** on, **Settings › Questions › Ask about a meeting** adds a chat under each finished meeting. **Questions run on** picks the model; the default is Same as Files.

## Where recordings live

Audio and transcripts stay in VibeHex's data folder; a Markdown copy goes to a Hex Meetings folder inside Documents. **Settings › After a meeting › Transcript folder** reveals or changes it. **Keep meetings** (same section): 24 hours, 7 days, 30 days or Forever. New meetings default to 24 hours; expired ones go to the Trash.

**Where.** Labs › Meetings (`meetings.enabled`). Off a Mac, `hex labs on meetings` is refused; `hex settings set meetings.enabled on` works.

**Asked as.** "Can it record my Zoom calls?"; "Where are my meeting notes?"; «Как записывать созвоны?»; «Почему не слышно собеседника в записи?»; «Сколько хранятся записи встреч?»

<!-- sources: docs/features/meetings.md:19-143, 128-215; src/rune_ui/platform_settings.rs:268-380, 420-560, 925-980; src/rune_ui/models.rs:207-232, 381-390; src/rune_ui/meeting_notes.rs:41-210, 266-292; src/rune_ui/meeting_tabs/mod.rs:34-48; src/rune_ui/meeting_tabs/requests.rs:56-63; src/rune_ui/labs.rs:422-505; src/rune_ui/platform.rs:75-120; src/labs.rs:460-495; src/labs/cli.rs:104-113; src/control/settings.rs:679-722, 879-882; src/app_settings/meetings.rs:93-110, 152-172, 251-290; src/app_settings/meeting_folder.rs:5-18; src/meeting/detection.rs:270-292; src/meeting/generator.rs:229-250; src/microphone_activity.rs:106-120 -->
