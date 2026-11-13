---
topic: assistant-steps
title: Assistant reading step by step
keys: [assistant.agent, assistant.read_recordings, assistant.send_settings, assistant.web]
os: every OS
---

# Assistant reading step by step

On Codex, Assistant decides for itself what a question needs and reads it
a step at a time: more of this guide, a setting, your cleanup instructions
and Vocabulary, your meetings, Files texts, and dictations, how you use
VibeHex, the release notes. Each thing it reads appears as a line while it
works ("Read the notes of “Pricing review”", "Read your cleanup
instructions") and stays under the answer's fold, such as "Read 3 meetings
and your settings in 9 s". A question this guide answers is still answered
at once.

## What it may read

It only reads. A change is still a card you press, and what **Assistant ›
Settings › What it reads** turns off stays unread: with **Your current
settings** off it sees no value, instruction, or Vocabulary term.

If an answer needs your recordings and you haven't allowed them, the same
**Read your meetings, files, and dictations?** card comes up mid-answer,
once in a conversation: **Allow** and it reads on, **Answer without
recordings** and it answers without them. A line with a small cross says
what it didn't read.

Once an answer has read anything of yours, it and the rest of that
conversation stay off the web. A question about your own recordings or
Vocabulary never searches the web at all.

## Which engines, and turning it off

Built-in models, OpenRouter, your own server, and OpenCode answer in one
go, from what VibeHex picks for the question. `hex settings set
assistant.agent off` makes Codex do the same; `all` reads step by step on
every engine that isn't built in, and a list (`codex, openrouter, custom,
opencode`) names them; an empty value is the default again. There is no
row for it in the window.

Reading takes a second request, so such an answer starts a few seconds
later; the steps show what it is doing meanwhile. Six requests and twelve
reads are the most one answer makes.

**Where.** `hex settings set assistant.agent` (no row); Assistant ›
Settings › What it reads (`assistant.read_recordings`,
`assistant.send_settings`, `assistant.web`).

**Asked as.** "Why does Assistant read in steps?"; "What can Assistant
read on its own?"; "How do I turn off Assistant's step-by-step reading?";
"Why did Assistant ask to read my recordings in the middle of an answer?";
«Почему ассистент читает по шагам?»; «Как отключить пошаговое чтение у
ассистента?»; «Что ассистент может читать сам?».

<!-- sources: docs/features/assistant.md; src/assistant/agent/mod.rs; src/assistant/agent/table.rs; src/assistant/agent/words.rs; src/assistant/agent/live.rs; src/rune_ui/assistant/recall.rs -->
