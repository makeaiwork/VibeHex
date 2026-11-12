---
feature: unload-idle
key: labs.unload_idle
title: Unload model when idle
for: [dictation]
ask: Free the speech model's memory while you're not dictating?
group: Models
os: every OS
vibehex: false
---

# Unload model when idle

**What it does.** A loaded speech model holds memory (RAM, and video memory on
a GPU) all day. With this on, VibeHex frees it after it has gone unused for a
while, and the next dictation loads it again.

**Turn it on when.** Your computer is short on memory, or you play games or run
heavy apps and VibeHex should not keep a large model loaded.

**Leave it off when.** You want every dictation to start at once; a model that
was unloaded must load again first.

**Needs.** Nothing else. Apple Speech, which holds no model of VibeHex's,
stays as it is.

**Good to know.** Once it is on, a row Unload after offers 5, 15, 30, or 60
minutes (15 by default). The model does not unload while a dictation or a
meeting is recording. Audio recorded while it reloads waits, so nothing is lost.
Turning the feature off stops the unloading.

**Where.** Labs › Unload model when idle (`labs.unload_idle`), with its minutes
in `labs.unload_idle_minutes`.

**Asked as.** "VibeHex uses too much memory"; "Free RAM when I'm not
dictating"; «Модель занимает много памяти, как её выгружать»; «Почему первая
диктовка после паузы медленная».

<!-- sources: docs/features/labs.md:154-169; src/labs.rs:487-489; src/rune_ui/labs.rs:429-433, 901-925; src/app_settings/labs_settings.rs:18-20, 94; src/transcription/engine.rs:1012-1055 -->
