---
feature: expire-meeting-audio
key: labs.expire_meeting_audio
title: Expire meeting audio separately
group: Meetings
os: every OS
vibehex: false
---

# Expire meeting audio separately

**What it does.** Audio takes most of a meeting's disk space. With this on, Meetings › Settings gets Keep audio under Keep meetings, so finished meetings drop their audio sooner while the transcript, notes, summary and Markdown stay.

**Turn it on when.** You keep transcripts for months but need recordings only for a day or a week.

**Leave it off when.** You may want to transcribe a meeting again later.

**Needs.** Meetings.

**Good to know.** A time at or beyond Keep meetings changes nothing. Unfinished meetings keep their audio.

**Where.** Labs › Expire meeting audio separately (`labs.expire_meeting_audio`). Related: Meetings › Settings › Keep audio (`meetings.audio_retention`).

**Asked as.** "Delete meeting audio but keep transcripts"; «Встречи занимают много места».

<!-- sources: docs/features/labs.md:144-147, 1797-1834; src/rune_ui/platform_settings.rs:613-660; src/app_settings/meetings.rs:152-159; src/control/settings.rs:642-646, 713-717 -->
