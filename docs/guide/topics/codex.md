---
topic: codex
title: Codex, your ChatGPT plan as the writer
keys: [labs.ai_cleanup_server, dictation_processing.server.runs_on, dictation_processing.server.codex_model, dictation_processing.server.codex_effort, dictation_processing.server.codex_wait_seconds, meetings.notes.writer, meetings.notes.effort, meetings.questions.engine, meetings.questions.effort, files.cleanup_engine, files.request_engine, files.request_effort]
os: every OS (Files: macOS)
---

# Codex, your ChatGPT plan as the writer

Codex is OpenAI's agent program for the terminal. VibeHex borrows it as a language model: it rewrites your dictations, writes meeting notes, answers questions about meetings and handles Files requests on your own ChatGPT plan, with no API key.

## Install Codex and sign in

- macOS: run `brew install codex` in Terminal.
- Windows and Linux: run `npm install -g @openai/codex` (needs Node.js).

In **Output › AI cleanup** (visible with **Labs › Clean up on your own server** on), set **Runs on** to **Codex**. The Codex row shows "Not found" until it is installed; **Copy command** copies the line above and **Check again** looks again. Then press **Sign in with ChatGPT**: your browser opens, you sign in there, and VibeHex never sees your password.

## Where you can pick a Codex model

All of these need **Labs › Clean up on your own server** and Codex set up above.

- **Output › AI cleanup › Model**: dictation cleanup. The list comes from Codex itself.
- **Meetings › Settings › Notes › Written by**: the Codex group.
- **Meetings › Settings › Questions › Questions run on**, while **Ask about a meeting** is on.
- **Files › Settings › Cleanup › Runs on** and **Requests › Runs on** (Files is a macOS Labs feature).

## Model and Reasoning

**Reasoning** (Low, Medium, High): Low answers soonest, higher levels think longer. Meeting notes and questions have their own Reasoning, and Files has one under Requests. **Wait for Codex** (30 s, 1 min, 2 min; 1 min by default) is how long a dictation waits before it is pasted without AI cleanup.

## Plan limits

**Plan usage** shows how much of each window is spent (such as five hours, today, this week). It is shared with Codex itself, and a dictation uses very little. When the limit is reached, dictations paste without AI cleanup until it resets. Meeting notes then try your AI cleanup engine, then OpenCode if it runs. If sign-in is lost, VibeHex says to sign in to Codex again.

## What Codex receives

Only the text being worked on (a dictation, a transcript, a file) and your instructions, sent to OpenAI through your ChatGPT account. VibeHex runs Codex isolated: no tools, shell, web search, plugins, memories or project files, an empty working folder, and a throwaway conversation outside its history. Your sign-in stays with Codex.

**Asked as.** "Can I use my ChatGPT subscription instead of an API key?"; "Codex not found"; "Does Codex see my files?"; «Как подключить Codex»; «Пишет, что лимит Codex исчерпан»; «Что отправляется в ChatGPT?».

<!-- sources: src/dictation/cleanup_server/codex.rs:1-70, 419-430; src/rune_ui/output/codex_cleanup.rs:70-380; src/app_settings/dictation_processing.rs:20-60; src/rune_ui/meeting_notes.rs:44-300; src/rune_ui/files/settings.rs:55-70, 330-450; src/rune_ui/files.rs:138; docs/features/labs.md:915-940, 1054-1081; docs/features/meetings.md:36-52, 1155-1165; src/app_settings/notes_settings.rs:46 -->
