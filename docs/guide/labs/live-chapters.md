---
feature: live-chapters
key: labs.live_chapters
title: Live chapters
group: Meetings
os: [macOS, Windows, Linux]
vibehex: false
---

# Live chapters

**What it does.** Splits a meeting transcript into named chapters while the
call is still recording, such as "image storage" or "chat app name". After you
stop, VibeHex moves each chapter boundary to where the topic really began. The
meeting's title also gains its longest chapter names, unless you typed the
title yourself.

**Turn it on when.** Your meetings are long and you want to jump to one topic
or copy one section.

**Leave it off when.** Short calls, or you don't want a model reading the
transcript during the call.

**Needs.** Labs › Meetings (beta) on, and a model that writes meeting notes:
Apple Intelligence (Mac), OpenCode with Meetings › Settings › After a meeting ›
Summaries set to OpenCode, or the writer chosen in Meetings › Settings › Notes
› Written by. Without one, no chapters appear. The row shows on Windows and
Linux too, but there `hex labs` and `hex settings` refuse it; switch it in the
window.

**Good to know.** It is read when a recording starts, so it does not affect a
recording already running. A chapter needs at least 90 seconds. Each chapter
has a Copy section action in the Meetings pane.

**Where.** Labs › Live chapters (`labs.live_chapters`).

**Asked as.** "Can meetings have chapters?"; "Break my call transcript by
topic"; «Как разбить расшифровку встречи на главы»; «Темы встречи по ходу
записи».

<!-- sources: docs/features/meetings.md:1258-1342; docs/features/labs.md:1254-1259; src/meeting/generator.rs:166-207, 392-443; src/meeting.rs:542, 672-680; src/rune_ui/labs.rs:226-232 -->
