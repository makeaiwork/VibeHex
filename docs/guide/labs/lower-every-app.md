---
feature: lower-every-app
key: labs.lower_every_app
title: Lower every app
for: [dictation]
ask: Turn down every app while you dictate, even through monitor speakers?
then: Set While dictating to Mute or Lower audio, allow system audio
group: Dictation
os: macOS
vibehex: true
---

# Lower every app

**What it does.** With While dictating set to Mute or Lower audio, VibeHex turns your sound down. macOS can't set the volume of some outputs, such as a monitor on HDMI or DisplayPort, so only Music or Spotify got quieter. With this on, VibeHex turns down every other app, browsers included, through a system audio tap.

**Turn it on when.** You listen through a monitor's speakers and sound keeps playing over your dictation.

**Leave it off when.** Your output has its own volume, or While dictating is Pause media or Do nothing: the lab only acts on Mute and Lower audio.

**Needs.** macOS 14.2 or later and the Screen & System Audio Recording permission. Turning the lab on asks for it. Without the permission, VibeHex quietly falls back to Music and Spotify.

**Good to know.** VibeHex's own sounds aren't lowered. It is off by default.

**Where.** Labs › Lower every app (`labs.lower_every_app`), in the Dictation group, marked with a star. Related: General › Advanced › Output › While dictating (`recording_audio_behavior`).

**Asked as.** "Music doesn't get quieter while I dictate"; "Mute doesn't work on my monitor speakers"; «Звук не приглушается при диктовке»; «Через HDMI монитор звук не стихает, пока я говорю».

<!-- sources: src/rune_ui/labs.rs:817-822, 1017-1023; src/labs.rs:355, 544, 1370-1375, 427; src/labs/group.rs:60; src/audio/recording_environment/ducking.rs:1-60; src/audio/recording_environment/every_app.rs:48-62; src/audio/recording_environment/players.rs:3; src/rune_ui/general.rs:866-890; src/app_settings.rs:200-220; src/control/settings.rs:581-586 -->
