---
feature: follow-live-transcript
key: labs.follow_live_transcript
title: Follow live transcript
group: Meetings
os: every OS
vibehex: false
---

# Follow live transcript

**What it does.** While a meeting records, its transcript in Meetings keeps scrolling to the newest line, so you can read along without scrolling by hand.

**Turn it on when.** You keep the Meetings pane open during calls.

**Leave it off when.** You'd rather the transcript stay where you scrolled it.

**Needs.** Meetings turned on, with the recording meeting open in Meetings.

**Good to know.** Scroll up to read earlier text and following stops; scroll back to the bottom to follow again. It only follows a meeting that is starting or recording. Once you press Stop, and for finished meetings, the transcript scrolls by hand only.

**Where.** Labs › Follow live transcript (`labs.follow_live_transcript`).

**Asked as.** "Auto-scroll the meeting transcript"; "Keep the live transcript at the bottom"; «Автопрокрутка расшифровки встречи»; «Расшифровка не прокручивается сама во время встречи».

<!-- sources: docs/features/labs.md:132-136; docs/features/meetings.md:111-115; src/rune_ui/labs.rs:474-479 (row, shown where Meetings is); src/rune_ui/meetings.rs:1023-1031 (only Starting or Recording, not after Stop), 998-1002; src/labs.rs:333, 407, 527 (Everyone, every OS); src/labs/group.rs:95 (Meetings); src/control/settings.rs:628-631 -->
