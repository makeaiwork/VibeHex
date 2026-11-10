---
topic: troubleshooting
title: Troubleshooting
keys: [dictation_hotkey, shortcut_mode, microphone, release_microphone_while_idle, avoid_bluetooth_microphone, transcription.model, transcription.language, transcription_device, labs.unload_idle, labs.ai_cleanup_server, dictation_processing.server.runs_on, dictation_processing.server.codex_wait_seconds, labs.copy_only, paste_with_shift, meetings.enabled, meetings.notes.writer]
os: every OS
---

# Troubleshooting

Try the causes in order, then open the article named. A Mac's missing permission shows at the top of **General** under "Permissions needed".

## Nothing is pasted
1. The recording was under a third of a second or nearly silent; VibeHex drops it quietly.
2. Mac: Accessibility ("Device Control and Data Access" on macOS 27) is off. Press **Allow** in General › Permissions needed; if grey, **Still grey? Fix it**; if "Needs a restart", **Restart Hex**.
3. Linux Wayland: press **Allow** on Permissions needed › Paste into apps. If refused, it only copies; press Ctrl+V.
4. A terminal ignores Ctrl+V: turn on **Terminal paste** (General › Advanced › Output on Linux; Labs on Windows).
5. Mac: **Labs › Copy dictation only** is on.

Lost text is in History. See "Dictation shortcut, HUD, and pasting".

## The shortcut does nothing
1. General › Speech model says "No speech model installed": press **Download**.
2. Windows and Linux: General's status card needs **Enable**.
3. A hold under a third of a second is discarded.
4. Linux Wayland without a recorder: bind the `hex dictation` commands in your desktop's keyboard settings.
See "What differs on macOS, Windows, and Linux".

## VibeHex doesn't hear me, or loses my first words
1. Choose another input in **General › Microphone**, and allow Microphone (Mac).
2. With a headset, turn on **Use the Mac's microphone instead of Bluetooth** (General › Advanced › Dictation).
3. Switch off **Turn off the mic when not dictating**; it can clip the first moment.

## Words come out wrong
Add names to **General › Vocabulary** (GigaAM, SenseVoice and Cohere ignore it), try another **Speech model**, or use **Output › Replace words**. See "Vocabulary".

## It writes in the wrong language
Choose the model first, then **General › Language**: a model without your language switches to its own, usually English.

## Dictation is slow
1. Pick the model badged Fastest in Speech model › Compare.
2. Windows and Linux: **Speech engine** (General › Advanced › Dictation).
3. Slow only after a pause: turn off **Labs › Unload model when idle**.
4. AI cleanup waits for its engine, up to **Wait for Codex** (1 minute).

## AI cleanup doesn't run
1. **Clean up with AI** is off (Output › AI cleanup). Without **Labs › Clean up on your own server**, cleanup needs a running OpenCode.
2. The engine failed or was slow, so text pasted without cleanup. Press **Test** on its row.
3. An Output › In some apps rule with no steps skips it.

See "AI cleanup and the models behind it".

## OpenCode isn't running
VibeHex never starts it: start OpenCode 2, leave it open, press **Check again** or **Retry**. "Update OpenCode 2" means it is too old. See "OpenCode".

## Codex isn't found, isn't signed in, or says the limit is reached
- "Not found": install Codex (**Copy command**), **Check again**.
- "Not signed in": **Sign in with ChatGPT**.
- "The limit is reached. Until it resets, dictations paste as heard": **Plan usage** shows what is spent; wait, or change **Runs on**.

See "Codex, your ChatGPT plan as the writer".

## The model won't download
Downloads need internet. In setup, "The download stopped." offers **Try again** or **Use a model file…**; later, use General › Speech model › **Download** or **Add model file…**. See "Speech models".

## Meetings don't record the other side
Mac: **Allow** on Meetings › Settings › Screen & System Audio Recording; if grey, **Still grey? Fix it**. Microphone is needed too. No offer? Check Detected calls for "Never offer".

## Meeting notes are missing
Apple Intelligence, the default writer, needs macOS 26, switched on, with its model downloaded; the row says which is missing. Choose another in **Meetings › Settings › Notes › Written by**.

## A setting or tab isn't there
Labs features are off by default, and some exist on one OS only. Still stuck: About › Email the developer.

**Asked as.** "Nothing gets pasted"; "The shortcut doesn't work"; «Диктовка ничего не вставляет»; «Не работает горячая клавиша»; «Не слышно собеседника в записи»; «Модель не скачивается».

<!-- sources: docs/features/recovery.md (silent discard, microphone, Wayland); src/rune_ui/platform_settings.rs:20-165; src/rune_ui/general.rs:139-175,403-410,574,859-899,1319; src/rune_ui/settings.rs:603-662; src/rune_ui/output/codex_cleanup.rs:100-175,360; src/rune_ui/models.rs:241,425-445; src/rune_ui/onboarding_kit/first_run.rs:440-490; src/rune_ui/found_choice.rs:93; src/rune_ui/meetings.rs:915-940; src/rune_ui/lists.rs:421; src/desktop/opencode_service.rs:21; src/app_settings.rs:1411; AGENTS.md:773-790; docs/features/meetings.md:69-76 -->
