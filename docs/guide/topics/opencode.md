---
topic: opencode
title: OpenCode
keys: [dictation_processing.default_mode, dictation_processing.server.runs_on, meetings.summaries, meetings.opencode_model]
os: every OS
---

# OpenCode

OpenCode is a separate coding-agent app that connects to AI providers you already use. VibeHex can borrow its models to tidy dictated text and write meeting notes, with no extra account in VibeHex itself.

## What VibeHex does with it

VibeHex only uses an OpenCode that is already running. It never installs, starts or restarts OpenCode, and it talks to it only on your own computer. It is used for:

- **AI cleanup**: without Labs, the rewrite of every dictation (the "Rewrite with OpenCode" step in Output) runs through OpenCode. With **Labs › Clean up on your own server**, **Output › AI cleanup › Runs on** can move it elsewhere (see AI cleanup and the models behind it). If a rewrite fails or runs too long, the text is pasted without AI cleanup.
- **Meeting notes**: set **Meetings › Settings › After a meeting › Summaries** to OpenCode for a title and short summary; On this Mac never falls back to OpenCode. **Notes › Written by** also lists OpenCode (under Other) for full notes. Either way the transcript goes to the OpenCode model.
- **Annotate**: captures can be sent to your OpenCode sessions.

## "OpenCode isn't running"

VibeHex found no running OpenCode. To fix it:

1. Start OpenCode 2 (the current version) and leave it open.
2. Press **Check again** or **Retry** where VibeHex offers it. VibeHex also looks again about once a minute while a screen that needs it is open.

**Open OpenCode setup** opens OpenCode's own instructions. If the message says "Update OpenCode 2", the running OpenCode is too old; update it from opencode.ai.

## Choose its model

The regular window has no OpenCode model picker now. If you set no model, VibeHex picks a fast one your OpenCode has connected. To choose one, use **Output › Edit with agent**, which copies a prompt for an AI agent to edit your settings, or set `model` in the rewrite part of `dictation_processing.default_mode` (a provider/model name, such as `openai/gpt-5.4-mini`) with the `hex settings` command. For meeting notes, set Summaries to OpenCode and choose under it in **Model** (`meetings.opencode_model`), a list of the running OpenCode's models; "Hex picks a fast model" is the automatic choice.

## Good to know

Your organization may lock OpenCode off; the rows then say so. Without OpenCode, Codex, OpenRouter, your own server or a built-in model can do the same jobs.

**Asked as.** "OpenCode isn't running, what do I do?"; "How do I change the AI model for rewriting?"; «Пишет, что OpenCode не запущен»; «Как выбрать модель для обработки текста»; «Нужен ли OpenCode?».

<!-- sources: src/desktop/opencode_service.rs:1-130; src/desktop/opencode_catalog.rs:40-80; docs/opencode-compatibility.md:30-60; src/rune_ui/output.rs:150-170, 755-770; src/rune_ui/sidebar.rs:124; src/rune_ui/platform_settings.rs:396-440; src/rune_ui/meeting_notes.rs:56-90; src/rune_ui/models.rs:236-245; docs/features/labs.md:881-905; docs/features/capture.md:142-145 -->
