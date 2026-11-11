---
feature: expire-meeting-audio
key: labs.expire_meeting_audio
title: Expire meeting audio separately
for: [meetings]
ask: Delete meeting audio sooner while keeping the transcripts?
then: Choose how long in Meetings › Settings › Keep audio
group: Meetings
os: every OS
vibehex: false
---

# Expire meeting audio separately

**What it does.** Audio takes most of a meeting's disk space. With this on, Meetings › Settings gets a Keep audio row under Keep meetings, with Follow Keep meetings, 24 hours, 7 days, 30 days and Forever. Finished meetings then move their audio to the Trash sooner, while the transcript, notes, summary and Markdown stay.

**Turn it on when.** You keep transcripts for months but need recordings only for a day or a week.

**Leave it off when.** You may want to transcribe a meeting again later; without its audio that can't be done.

**Needs.** Meetings turned on; the row sits in Meetings › Settings.

**Good to know.** A time at or beyond Keep meetings changes nothing, since the whole meeting goes first. Meetings still recording, being transcribed or having notes written keep their audio for a later pass. Turning the lab off returns audio to Keep meetings.

**Where.** Labs › Expire meeting audio separately (`labs.expire_meeting_audio`). Related: Meetings › Settings › Keep audio (`meetings.audio_retention`).

**Asked as.** "Delete meeting audio but keep transcripts"; "Meetings take too much disk space"; «Встречи занимают много места»; «Как удалять аудио встреч, оставляя расшифровку».

<!-- sources: docs/features/labs.md:144-147, 1797-1834; src/rune_ui/labs.rs:493-499 (row, shown with Meetings); src/rune_ui/platform_settings.rs:515-519 (Keep audio only while lab on), 574-600, 615-660 (choices, Trash wording); src/app_settings/meetings.rs:142-169 (choices), 420-445 (lab off means follows Keep meetings); src/meeting/retention.rs:111-157, 177-210 (Complete only, WAVs to the Trash, text stays); src/labs/group.rs:100; src/labs.rs:529; src/control/settings.rs:642-646, 713-717 -->
