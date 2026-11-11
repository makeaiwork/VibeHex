---
topic: troubleshooting
title: Troubleshooting
keys: [dictation_hotkey, shortcut_mode, microphone, release_microphone_while_idle, avoid_bluetooth_microphone, transcription.model, transcription.language, transcription_device, labs.unload_idle, labs.ai_cleanup_server, dictation_processing.server.runs_on, dictation_processing.server.codex_wait_seconds, labs.copy_only, paste_with_shift, meetings.enabled, meetings.summaries, meetings.opencode_model]
os: every OS
---

# Troubleshooting

Try the causes in order, then open the article named. A Mac's missing permission shows at the top of **General** under "Permissions needed".

## Nothing is pasted
1. The recording was under a third of a second or nearly silent; VibeHex drops it quietly.
2. Mac: Accessibility ("Device Control and Data Access" on macOS 27) is off. Press **Allow** in General › Permissions needed; if grey, **Still grey? Fix it**; if it says "Turns on when Hex restarts", **Restart Hex**.
3. Linux Wayland: press **Allow** on Permissions needed › Paste into apps. If refused, it only copies; press Ctrl+V.
4. A terminal ignores Ctrl+V: turn on **Terminal paste** (General › Advanced › Output on Linux; Labs on Windows).
5. Mac or Windows: **Labs › Copy dictation only** is on, so text is only copied.

Lost text is in History. See "Dictation shortcut, HUD, and pasting".

## The shortcut does nothing
1. General › Speech model says "No speech model installed": press **Download**.
2. Windows and Linux: General's status card needs **Enable**.
3. A hold under a third of a second is discarded.
4. Linux Wayland without a recorder: bind the `hex dictation` commands in your desktop's keyboard settings.
See "What differs on macOS, Windows, and Linux".

## VibeHex doesn't hear me, or loses my first words
1. Choose another input in **General › Microphone**, and allow Microphone (Mac).
2. With a headset, turn on **Use the Mac's microphone instead of Bluetooth** (General › Advanced › Dictation; Windows and Linux: **Use another microphone instead of Bluetooth**).
3. Switch off **Turn off the mic when not dictating**; it can clip the first moment.

## Words come out wrong
Add names to **General › Vocabulary** (GigaAM, Granite Speech 5.0, SenseVoice and Cohere ignore it; the row says so), try another **Speech model**, or use **Output › Replace words**. See "Vocabulary".

## It writes in the wrong language
Choose the model first, then **General › Language**: a model without your language switches to its own, usually English.

## Dictation is slow
1. Pick the model badged Fastest in Speech model › Compare.
2. Windows and Linux: **Speech engine** (General › Advanced › Dictation). On Windows x64, if the row says the graphics driver's Vulkan is missing or too old, VibeHex uses the CPU until you update the GPU driver.
3. Slow only after a pause: turn off **Labs › Unload model when idle**.
4. AI cleanup waits for its engine; with Codex, up to **Wait for Codex** (1 minute).

## AI cleanup doesn't run
1. **Clean up with AI** is off (Output › AI cleanup). Without **Labs › Clean up on your own server**, cleanup needs a running OpenCode.
2. The engine failed or was slow, so text pasted without cleanup. Press **Test** on its row.
3. An Output › In some apps rule with no steps skips it.

See "AI cleanup and the models behind it".

## OpenCode isn't running
VibeHex never starts it: start OpenCode 2, leave it open, press **Check again**. "Update OpenCode 2" means it is too old. See "OpenCode".

## Codex isn't found, isn't signed in, or says the limit is reached
- "Not found": install Codex (**Copy command**), **Check again**.
- "Not signed in": **Sign in with ChatGPT**.
- "The limit is reached. Until it resets, dictations paste as heard": **Plan usage** shows what is spent; wait, or change **Runs on**.

See "Codex, your ChatGPT plan as the writer".

## The model won't download
Downloads need internet. In setup, "The download stopped." offers **Try again** or **Use a model file…**; later, use General › Speech model › **Download** or **Add model file…**. See "Speech models".

## Meetings don't record the other side
Mac: **Allow** on Meetings › Settings › Screen & System Audio Recording; if grey, **Still grey? Fix it**. Microphone is needed too. Linux needs PipeWire's pw-record installed. No offer? Check Detected calls for "Never offer"; Signal, Telegram and some other apps are listed only with **Labs › More call apps**.

## Meeting notes are missing
Check **Meetings › Settings › After a meeting › Summaries** first. **Off** writes no titles or summaries on its own, and it is the default on Windows and Linux; on Windows and Linux turn the switch on (OpenCode, which must be running); on a Mac choose **On this Mac** or **OpenCode**. On this Mac needs Apple Intelligence on macOS 26, switched on, with its model downloaded; the row says which is missing. If **Notes › Written by** names a writer, Summaries is hidden and that writer decides.

## A setting or tab isn't there
Labs features are off by default, and some exist on one OS only. Still stuck: About › Email the developer.

**Asked as.** "Nothing gets pasted"; "The shortcut doesn't work"; «Диктовка ничего не вставляет»; «Не работает горячая клавиша»; «Не слышно собеседника в записи»; «Модель не скачивается».

<!-- sources: docs/features/recovery.md (silent discard, microphone, Wayland); src/rune_ui/platform_settings.rs:20-165; src/rune_ui/general.rs:139-175,403-410,574,859-899,1319; src/rune_ui/settings.rs:603-662; src/rune_ui/output/codex_cleanup.rs:100-175,360; src/rune_ui/models.rs:241,425-445; src/rune_ui/onboarding_kit/first_run.rs:440-490; src/rune_ui/found_choice.rs:93; src/rune_ui/meetings.rs:163-186, 1156-1174; src/rune_ui/platform_settings.rs:268-380; src/meeting/generator.rs:229-250; src/meeting/linux.rs:27-35; src/transcription/models.rs:348-360, 621-632; src/rune_ui/vocabulary.rs:149-160; src/rune_ui/labs.rs:520-535; src/rune_ui/general.rs:795-815; src/app_settings/meetings.rs:251-275; src/rune_ui/lists.rs:421; src/desktop/opencode_service.rs:21; src/app_settings.rs:1411; AGENTS.md:773-790; docs/features/meetings.md:69-76 -->
