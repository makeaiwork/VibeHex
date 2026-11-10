---
topic: privacy
title: Privacy and your data
keys: [history_retention, assistant.read_recordings, labs.keep_recordings, labs.ai_cleanup_server, dictation_processing.server.runs_on, dictation_processing.default_mode, meetings.retention]
os: every OS
---

# Privacy and your data

By default VibeHex does its work on your computer. This page says what stays
there, what can leave and only when you choose it, and how to clear things.

## What stays on your computer

- **Your voice.** A speech model downloaded once turns it into text locally.
  Audio is not sent anywhere and is not kept, unless you turn on Labs › Keep
  recordings.
- **History.** Finished dictations are kept for 7 days by default. The Keep menu
  in History sets Off, 24 hours, 7 days, 30 days, or Forever.
- **Recordings.** With Labs › Keep recordings on, each new History entry also
  keeps its audio, so you can transcribe it again with another model. The audio
  goes when its entry does.
- **Meetings** (Mac, from Labs). Audio and transcripts stay on the Mac. Meetings ›
  Settings › Keep meetings defaults to 24 hours.
- **A diagnostic log** in the data folder records what dictation did, including
  the recognised text. History's Off and retention do not control it.

The data folder is `~/Library/Application Support/hex` on a Mac, `%APPDATA%\hex`
on Windows, and `~/.local/share/hex` on Linux.

## What leaves, and when

- **Model downloads** come from Hugging Face, once per model. On macOS 26
  and later, Annotate also has macOS fetch Apple's on-device speech model at
  launch, from Apple.
- **Update checks** ask GitHub for the newest version. No dictation goes.
- **AI cleanup** is off until you turn on Output › AI cleanup › Clean up with
  AI. Then each dictation, your instructions, and your Vocabulary go to the
  engine in "Runs on". OpenCode is a separate app on your computer, which then
  uses whatever model you gave it. Codex sends text to OpenAI over your ChatGPT
  plan, and OpenRouter to its model provider. "Your own" stays on your
  computer for LM Studio or Ollama at their usual local addresses, and goes
  wherever another address points.
  "Built in" (Mac) never leaves.
- **Meeting notes:** Apple Intelligence keeps the transcript on the Mac; any
  other writer in Meetings › Notes › Written by receives it.
- **Assistant** sends your question, the conversation, and guide text to
  the model in Assistant › Settings › Runs on. Only after you allow it on
  the first question about your recordings does it also send passages it
  found on this computer in your meetings, Files texts, and dictations; the
  recordings themselves never go. Assistant › Settings › What it reads takes
  that back.
- Your own scripts in Output ask first to use the network.

## Where API keys are kept

Not in the settings file. A Mac keeps them in the Keychain, Windows in
Credential Manager, and Linux in a file only you can read in the data folder.
Codex keeps its own sign-in. Output › AI cleanup › API key has Remove.

## Clear things

- History: Clear all (press again on "Really clear all?") or Delete on one
  entry. Both remove saved recordings too.
- Stop saving: History's Keep menu to Off (entries already there stay until
  deleted), and Labs › Keep recordings off.
- Meetings: delete a recording, or shorten Keep meetings.
- Assistant: Settings › Clear conversations…, or Keep on this computer off.
- The log: quit VibeHex, then delete the `logs` folder.

**Asked as.** "Does my voice go to the cloud?"; "How do I delete my history?";
«Мои записи отправляются в интернет?»; «Как удалить историю диктовок?»; «Где
хранится API-ключ?».

<!-- sources: docs/features/README.md:660-690; src/history.rs:26-96,390; src/history/recordings.rs:1-27; src/rune_ui/history.rs:515-560; src/dictation/cleanup_server/keys.rs:1-12; src/dictation/cleanup_server/codex.rs:1-12; src/rune_ui/output/ai_cleanup.rs:120-200,456-510; src/rune_ui/meeting_notes.rs:130-145; src/rune_ui/platform_settings.rs:470-490; src/rune_ui/output.rs:300-330; src/updater.rs:1-60; AGENTS.md:1756-1775; src/desktop/events.rs:26-30; docs/features/README.md:56-110 -->
