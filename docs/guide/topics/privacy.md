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
  Audio is not sent anywhere and is not kept, unless you choose Keep audio
  too in History's menu (the same switch as Labs › Keep recordings).
- **History.** Finished dictations are kept for 7 days by default. The menu at
  the top of History, which reads "Kept 7 days", sets Off, 24 hours, 7 days,
  30 days, or Forever.
- **Recordings.** With Keep audio too chosen, each new History entry also
  keeps its audio, so you can transcribe it again with another model. The audio
  goes when its entry does.
- **Meetings** (from Labs, a beta on Windows and Linux). Audio and transcripts
  stay on the computer. Meetings › Settings › After a meeting › Keep meetings
  defaults to 24 hours for new meetings, which then move to the Trash; meetings
  from before the setting stay until you choose a period.
- **A diagnostic log** in the data folder records what dictation did, but not
  the words you said: those live only in History. History's Off and retention
  do not control the log.

The data folder is `~/Library/Application Support/hex` on a Mac, `%APPDATA%\hex`
on Windows, and `~/.local/share/hex` on Linux. On Windows the logs and the
speech models live in `%LOCALAPPDATA%\hex` instead, so they are not copied
to a server at sign-in; everything else stays in `%APPDATA%\hex`.

## What leaves, and when

- **Model downloads** come from Hugging Face, once per model. On macOS 26
  and later, Annotate also has macOS fetch Apple's on-device speech model at
  launch, from Apple.
- **Update checks** ask GitHub for the newest version. No dictation goes.
- **AI cleanup** is off until you turn on Labs › Clean up on your own server
  and then Output › AI cleanup › Clean up with AI. Then each dictation, your
  instructions, and your Vocabulary go to the engine in "Runs on". OpenCode is a separate app on your computer, which then
  uses whatever model you gave it. Codex sends text to OpenAI over your ChatGPT
  plan, and OpenRouter to its model provider. "Your own" stays on your
  computer for LM Studio or Ollama at their usual local addresses, and goes
  wherever another address points.
  "Built in" (Mac) never leaves.
- **Meeting notes:** Apple Intelligence keeps the transcript on the Mac, and a
  Built in writer keeps it on the computer; any other writer in Meetings ›
  Settings › Notes › Written by (OpenCode, Codex, Other) receives it. So does
  OpenCode when Meetings › Settings › After a meeting › Summaries is set to
  OpenCode (on Windows and Linux, a switch that turns it on): the transcript
  goes to OpenCode and to whatever model OpenCode uses. Summaries is hidden
  while Written by names a writer.
- **Assistant** sends your question, the conversation, and guide text to
  the model in Assistant › Settings › Runs on. Only after you allow it on
  the first question about your recordings does it also send passages it
  found on this computer in your meetings, Files texts, and dictations; the
  recordings themselves never go. Assistant › Settings › What it reads takes
  that back.
- Your own scripts in Output ask first to use the network.

## Where API keys are kept

Not in the settings file. A Mac keeps them in the Keychain, Windows in
Credential Manager, and Linux in a file only you can read in the data folder;
a Mac or Windows whose store refuses uses that file too.
Codex keeps its own sign-in. Output › AI cleanup › API key has Remove.

## Clear things

- History: the menu's "Clear all history…", then the red "Clear all history"
  button, or Delete on one entry. Both remove saved recordings too.
- Stop saving: History's menu to Off (entries already there stay until
  deleted), and Keep audio too off.
- Meetings: delete a recording, or shorten Keep meetings.
- Assistant: Settings › Clear conversations…, or Keep on this computer off.
- The log: quit VibeHex, then delete the `logs` folder (in `%LOCALAPPDATA%\hex`
  on Windows).

**Asked as.** "Does my voice go to the cloud?"; "How do I delete my history?";
«Мои записи отправляются в интернет?»; «Как удалить историю диктовок?»; «Где
хранится API-ключ?».

<!-- sources: docs/features/README.md:660-690; src/history.rs:26-96,390; src/history/recordings.rs:1-27; src/rune_ui/history.rs:316-331,702-800; src/app_paths.rs:29-100; src/dictation/cleanup_server/keys.rs:1-12; src/dictation/cleanup_server/codex.rs:1-12; src/rune_ui/output/ai_cleanup.rs:108-200,456-510; src/rune_ui/labs.rs:776-780; src/rune_ui/meeting_notes.rs:113-123,126-210; src/rune_ui/platform_settings.rs:260-380,500-520,574-611; src/app_settings/meetings.rs:142-172; src/control/settings.rs:688-697; src/rune_ui/output.rs:300-330; src/updater.rs:1-60; src/rune_ui/assistant/settings.rs:20-165; docs/features/meetings.md:128-135; crates/capture/src/speech_model.rs:1-10 -->
