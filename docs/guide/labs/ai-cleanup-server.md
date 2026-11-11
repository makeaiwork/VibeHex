---
feature: ai-cleanup-server
key: labs.ai_cleanup_server
title: Clean up on your own server
group: Output
os: every OS
vibehex: true
---

# Clean up on your own server

**What it does.** AI cleanup normally needs OpenCode, a separate app, running
on your computer. This switch lets cleanup run on other engines instead:
Codex with your ChatGPT plan, OpenRouter, LM Studio or Ollama, or a model
built into VibeHex.

**Turn it on when.** You have no OpenCode, or you want cleanup on a ChatGPT
plan, a local model, or a server of your own.

**Leave it off when.** OpenCode already works for you.

**Needs.** The engine you pick: a Codex sign-in, an OpenRouter key, or a
running local server. Built in works on macOS only and downloads a model.

**Good to know.** It adds the section Output › AI cleanup, where you choose
Output › AI cleanup › Runs on. Details and how to choose are in the article
"AI cleanup and the models behind it".
Marked with a star: added in VibeHex.

**Where.** Labs › Clean up on your own server (`labs.ai_cleanup_server`).
Related: Output › AI cleanup (`dictation_processing.server.runs_on`).

**Asked as.** "Use LM Studio or Ollama for cleanup"; "AI cleanup without
OpenCode"; «Чистка текста без OpenCode»; «Как подключить свою модель».

<!-- sources: docs/features/labs.md:84-86, docs/features/labs.md:913-1000; src/labs.rs:333, 341-345; src/rune_ui/labs.rs:270-275, 108-125 (ai_cleanup.rs); src/dictation/cleanup_server.rs:57-63; docs/guide/topics/ai-cleanup.md -->
