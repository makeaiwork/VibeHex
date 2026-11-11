---
feature: lower-every-app
key: labs.lower_every_app
title: Lower every app
group: Dictation
os: macOS
vibehex: true
---

# Lower every app

**What it does.** While dictating set to Mute or Lower audio turns your sound down. macOS can't set the volume of some outputs, such as an HDMI monitor, so only Music or Spotify got quieter. With this on, VibeHex turns down every other app, browsers too.

**Turn it on when.** You listen through a monitor's speakers and sound plays over your dictation.

**Leave it off when.** Your output has its own volume.

**Needs.** macOS 14.2 or later and Screen & System Audio Recording; turning it on asks.

**Good to know.** VibeHex's own sounds aren't lowered.

**Where.** Labs › Lower every app (`labs.lower_every_app`). Related: General › While dictating (`recording_audio_behavior`).

**Asked as.** "Music doesn't get quieter while I dictate"; «Звук не приглушается при диктовке».

<!-- sources: docs/features/labs.md:120, 1982-2004; src/audio/recording_environment/ducking.rs:28-55; src/rune_ui/labs.rs:817-822, 1017-1023; src/rune_ui/general.rs:866-890; src/control/settings.rs:582-586 -->
