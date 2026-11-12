---
feature: stop-when-call-ends
key: labs.stop_when_call_ends
title: Stop when the call ends
for: [meetings]
ask: Stop the recording by itself when the call ends?
group: Meetings
os: macOS
vibehex: false
---

# Stop when the call ends

**What it does.** A recording VibeHex started by itself used to keep running
after you hung up. With this on, once the call app stops using the microphone
and goes quiet, a notice says "VibeHex stops recording in 30 s unless you keep
recording." After 30 seconds the recording stops and is transcribed, as with
Stop & Transcribe.

**Turn it on when.** You set calls to Always record and forget to stop them,
leaving long silent recordings.

**Leave it off when.** You want to decide yourself every time when a recording
ends. It never touches recordings you started, accepted from an offer, or
began from a calendar reminder.

**Needs.** Meetings on, and an app set to Always record in Meetings ›
Settings › Detected calls.

**Good to know.** Keep recording in the notice cancels the stop for that
pause. If the call app returns within 30 seconds, nothing stops. Off, nothing
stops by itself.

**Where.** Labs › Stop when the call ends (`labs.stop_when_call_ends`).

**Asked as.** "My meeting keeps recording after the call"; "Auto stop
recording"; «Запись не останавливается после звонка»; «Как автоматически
остановить запись встречи».

<!-- sources: docs/features/labs.md:680-699; docs/features/meetings.md:530-552, 36-38; src/meeting/watcher.rs:699; src/rune_ui/labs.rs:242-248 -->
