---
topic: files
title: Files
keys: [labs.files, files.transcription, files.language, files.cleanup_after, files.cleanup_engine, files.cleanup_effort, files.request_engine, files.request_effort, files.quick_prompts]
os: macOS (Labs)
---

# Files

Files turns a recording you already have, a voice memo, a lecture, a video, into text. You drop it in, VibeHex transcribes it on this Mac, and you can then clean the text up or ask questions about it. It is a Labs feature, off by default, macOS only.

## Turn it on and add a file

Switch on **Labs › Files**. A Files pane appears in the sidebar. Drop audio or video onto it, or press **Choose file…**. Formats: mp3, m4a, aac, wav, aif, aiff, caf, flac, ogg, opus, m4b, mp4, mov, m4v, 3gp, webm. Anything else is refused with a message. VibeHex keeps only the file's location, not a copy: if you move it, the file shows "File not found".

## Watch the queue

Files go through one at a time: Extracting audio, then Transcribing, each row showing its place in line and progress. **Stop** ends a running one; **Remove** drops a waiting one. The queue does not survive quitting VibeHex: an interrupted file shows Stopped, with **Try again**. Cleanup works in pieces, and a request too long for the model is read in three steps. Extracting the audio gives up after 30 minutes.

## Read the tabs

Each result is a tab: **Original** (the transcript; switch Timestamps and Plain text), **Cleanup** (tidied text) and **Requests** (answers). On Original, **Transcribe again** retries with another downloaded model; **Clean up** makes a Cleanup.

## Ask for something

Under the text, tap **Structure**, **Summary** or **Action items**, pick one from **Recent**, or type your own in "Ask about this text". Each request becomes a chat in Requests, open to follow-ups. A request that reaches past the file ("as in my other interviews") also reads your other recordings once you allow it, and cites them.

## Choose which model answers

Open the header switch **Settings**:
- **Transcription › Model** (Same as dictation or any downloaded model), Language, Use vocabulary, Show timestamps.
- **Cleanup**: **Clean up after transcription**, and **Runs on**. On Codex, **Reasoning** picks how long it thinks; until you pick, it follows Output › AI cleanup's. Cleanup uses your Default mode's prompt unless you turn **Use the Default mode's prompt** off.
- **Requests › Runs on**: Same as cleanup, or another. **Reasoning** shows for Codex. **Quick requests › Edit…** changes the chips.

Engines: Built in (a downloaded model, offline), OpenCode, and, with **Labs › Clean up on your own server** set up in Output › AI cleanup, Codex, OpenRouter or Your server. Codex, OpenRouter, Your server and OpenCode send the text to that service.

## Copy or export

**Copy** has "as Markdown" and "as plain text". **Edit** corrects the text. In the "…" menu: **Save as…** (.md or .txt), **Show in Finder**, **Open original**, and the two deletes. Deleting never touches your file.

**Asked as.** "Can I transcribe an mp3 or a video?"; "How do I summarize a recording?"; «Как расшифровать аудиофайл?»; «Как сделать краткое содержание записи?»

<!-- sources: docs/features/files.md:1-130, 217-227; src/files/decode.rs:10-27; src/rune_ui/files/settings.rs:195-470; src/rune_ui/files/actions.rs:190-400, 595; src/rune_ui/files.rs:587-830; src/app_settings/prompt_defaults.rs:83 -->
