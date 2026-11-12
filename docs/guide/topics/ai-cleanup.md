---
topic: ai-cleanup
title: AI cleanup and the models behind it
keys: [dictation_processing.default_mode, labs.ai_cleanup_server, dictation_processing.server.runs_on, dictation_processing.server.codex_model, dictation_processing.server.codex_effort, dictation_processing.server.codex_wait_seconds, dictation_processing.server.url, dictation_processing.server.openrouter_model, dictation_processing.server.custom_model, dictation_processing.server.local_model]
os: every OS (Built in: macOS)
---

# AI cleanup and the models behind it

A speech model writes down what you said. AI cleanup then has a language
model tidy it: punctuation, obvious mishearings, your Vocabulary's spellings,
and your own instructions. If cleanup fails or takes too long, VibeHex pastes
the text without AI cleanup (your word fixes still apply), so a dictation is never lost.

## Where cleanup can run

Without Labs, cleanup runs through OpenCode, a separate app that must already
be running on the computer. Turn on **Labs › Clean up on your own server** to
choose in **Output › AI cleanup › Runs on**:

- **Codex**: your ChatGPT plan, no API key. Install Codex
  (`brew install codex` on a Mac), then press **Sign in with ChatGPT**. Pick
  the Model and Reasoning (Low, Medium, High); **Wait for Codex** sets how
  long a dictation waits. Plan usage shows how much of the limit is spent;
  when it runs out, dictations paste without AI cleanup. Codex gets only the dictation
  and your instructions.
- **OpenRouter**: hundreds of models with one API key.
- **Your own**: LM Studio (`http://127.0.0.1:1234/v1`) or Ollama
  (`http://127.0.0.1:11434/v1`), or any OpenAI-compatible address. Fill in
  the URL, choose a Model, no key needed for these two.
- **Built in** (macOS): a model downloaded into VibeHex, works offline.
  Gemma 4 E2B is fastest, Gemma 4 E4B writes best, Qwen3 4B is in between.
  Several GB each.
- **OpenCode**: as without the Labs switch.

API keys are kept in the macOS keychain or Windows Credential Manager, not in
settings. Each server row has **Test**.

## Which to pick

Codex if you have a ChatGPT plan: no API key and no setup beyond signing
in. Built in if nothing may leave the computer. Your own if you
already run LM Studio or Ollama.

## Turn it on and steer it

**Clean up with AI** switches cleanup for every dictation. **Custom
instructions** in the same section is where you tell it how to write (for
example, keep English terms in Latin letters). Long dictations are cleaned in
pieces of about three minutes, so there is no length limit.

**Asked as.** "Can I use ChatGPT to clean up dictation?"; "How do I connect
LM Studio?"; «Как подключить модель для чистки текста»; «Можно без
интернета?».

<!-- sources: docs/features/labs.md:913-1000; src/dictation/cleanup_server/codex.rs; src/dictation/cleanup_server/local_models.rs -->
