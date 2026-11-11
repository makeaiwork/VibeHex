---
feature: pause-browser
key: pause_browser_media
title: Also pause the browser
for: [dictation]
ask: Pause music and browser videos while you dictate?
then: Set While dictating to Pause media and allow system audio
group: Dictation
os: macOS
vibehex: false
---

# Also pause the browser

**What it does.** With General › While dictating set to Pause media, VibeHex pauses players like Music. Browser tabs such as YouTube don't answer that, so this switch also pauses a tab that is playing, and starts it again when you finish, but only if VibeHex paused it.

**Turn it on when.** You dictate with a video or music tab playing in your browser.

**Leave it off when.** While dictating you use Mute, Lower audio, or Do nothing, since only Pause media uses it.

**Needs.** macOS and While dictating set to Pause media. The first time, macOS asks for Screen & System Audio Recording, so VibeHex can hear whether a tab is playing. A matching permission row then appears in General.

**Good to know.** A browser using the microphone (a call) is left alone. A tab that was already silent isn't paused, and one that plays again on its own before you finish is left alone.

**Where.** Labs › Also pause the browser (`pause_browser_media`). Related: General › While dictating.

**Asked as.** "YouTube keeps playing while I dictate"; "Pause the browser video during dictation"; «Видео в браузере не ставится на паузу при диктовке»; «Как останавливать музыку в Chrome».

<!-- sources: docs/features/labs.md:50; docs/features/README.md:790-824; src/rune_ui/labs.rs:329-334, 715-719; src/rune_ui/general.rs:424-450, 951-958; src/labs.rs (on_this_os macOS) -->
