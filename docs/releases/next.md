---
version: next
date: unreleased
labs: [screenshots, notch-indicator, caps-lock-dictation, lower-every-app, more-call-apps, settings-search, failure-notices, follow-live-transcript, expire-meeting-audio, meeting-descriptive-filenames, meeting-notes-vocabulary, keep-recordings, history-stats, files]
keys: [assistant.enabled, assistant.send_settings, assistant.read_recordings, ui_language, history_retention, meetings.summaries, meetings.audio_retention, files.cleanup_effort]
---

# VibeHex Next (not released yet)

- New: Assistant, at the bottom of the sidebar. It answers questions about VibeHex, suggests
  setting changes as cards you decide on, and can set VibeHex up from three questions.
- New: With your permission, Assistant answers about your own meetings, Files texts, and
  dictations, and links the places it read. Ask it what's new after an update, too.
- New: VibeHex speaks Russian: General › Advanced › Application › Interface language.
- New in Labs: Screenshots (macOS). ⌘⇧4 takes a shot, copies and saves it, and a corner card
  opens it to mark it up.
- New in Labs: Notch indicator and Caps Lock dictation (macOS). The indicator grows out of a
  MacBook's notch; Caps Lock starts and finishes a hands-free dictation.
- New in Labs: Lower every app (macOS). On an output macOS can't set the volume of, such as
  an HDMI monitor, Mute and Lower audio turn down every app, not only Music and Spotify.
- New in Labs for meetings: More call apps (Signal, Telegram, and others), Follow live
  transcript, Expire meeting audio separately, Descriptive meeting file names.
- New in Labs: Vocabulary in meeting notes, Settings search, and Failure notices, which
  says why a dictation failed.
- New: More speech models: Orukeet, Granite Speech 5.0, and NB-Whisper Medium for Norwegian.
- Changed: History's Kept menu holds how long to keep, Keep audio too, and Clear all
  history; Dictation stats open from a Stats button.
- Changed: Meetings › Settings › After a meeting › Summaries is one picker for titles and
  summaries, and replaces the Choose meeting summary model lab.
- Changed: Labs groups its features by area, in sections that fold. The menu bar menu puts a
  meeting first and shows the shortcuts.
- Changed: Files › Settings has its own Reasoning for cleanup on Codex.
- Fixed: Mute and Lower audio make Music and Spotify quieter on outputs without a volume
  control, and lower the sound faster.
- Fixed: The dictation HUD stays on the Space you work in, VibeHex uses less CPU while idle,
  and a settings file that isn't valid JSON is never replaced.
