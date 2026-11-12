---
topic: vocabulary
title: Vocabulary
keys: [vocabulary_boost, labs.vocabulary_biasing, labs.vocabulary_paste_list, labs.vocabulary_quick_add, labs.russian_vibe_coding]
os: every OS
---

# Vocabulary

Vocabulary is one shared list of names, products, and jargon VibeHex should
spell your way (Kubernetes, Claude Code, a colleague's surname). It is empty
by default and then changes nothing.

## Add a term

Open **General › Vocabulary** (it shows "None", "1 term", or "N terms").
Press **Add term**, type the term, and optionally what it sounds like when the
model gets it wrong, separated by commas ("Kubernetes" with "кубернетис,
cooper netties"). **×** removes a row; **Done** saves and closes.

From a terminal, `hex vocabulary list`, `add` (with `--sounds-like`), and `remove`
edit the same list through the running VibeHex, on macOS, Windows, and Linux.

## Who uses it

- **AI cleanup and search** always use it, with every model.
- **Whisper, ivrit.ai Whisper, NB-Whisper, Qwen3-ASR, and QwenCleo** read the
  first terms as a hint while transcribing (always on). Their "sounds like"
  spellings are not sent.
- **Parakeet, Orukeet, Parakeet Ultra, Underdog Bark, and Nemotron 3.5** use it
  only with **Labs › Vocabulary biasing** on (see that article).
- **GigaAM, Granite Speech 5.0, SenseVoice, Cohere** don't use it in
  transcription. The row says so; fix words with AI cleanup or word fixes
  (Output article).

## What "sounds like" does

For the Parakeet-based models with biasing on, a heard word that matches one of
your "sounds like" spellings exactly is replaced by the term after
transcription, with no guessing. Spellings under two characters are ignored. A
spelling that is also an ordinary word is replaced every time it is heard, so
keep spellings distinctive. AI cleanup also sees each spelling next to its term.

## Limits

- What the speech model reads while it transcribes: Parakeet-based models
  take the first 64 terms. Whisper-based models take as many terms as fit in
  about 500 characters, and Qwen3-ASR and QwenCleo about 1,000, counting from
  the top of the list. When the model reads fewer terms than the list holds,
  the sheet says "This model reads the first N of M terms".
- That limit is only the speech model's. Clean up with AI, when it is on,
  reads the whole list, every term with its "sounds like" spellings, so terms
  past the limit are still corrected after transcription. With cleanup off,
  terms past the limit aren't used in dictation: move the ones you say most
  to the top, or turn Clean up with AI on.
- The list holds up to 500 terms, with 64 characters per term or spelling and
  16 spellings per term.
- A term with spaces ("Dax Rad") matches only when said in full. If you also
  say "Dax" alone, give it its own row.
- Short terms, under five letters, work better with a "sounds like" spelling.

## Add many terms, or one in passing

- **Labs › Vocabulary text view** adds **Text view** to the sheet: the list as
  text, one "Term | sounds like, sounds like" per line, to paste or edit. **Save**
  makes the list exactly that text.
- **Labs › Add to vocabulary from a selection** (macOS): press ⌥⇧A over a
  misheard word in any app, type the right spelling, and it is added and, where the text can be edited, replaces the selection.

## Russian vibe coding

**Labs › Russian vibe coding** adds a starter list of 80 terms (Claude Code, GitHub,
MCP, Docker, and so on, with how Russian speech models hear them), plus Russian word fixes and
cleanup instructions, and turns on Clean up with AI, **Labs › Vocabulary in meeting
notes** (so meeting titles and summaries use the terms), and the other VibeHex
Labs features except Stop shortcut. Turning it off removes what it added and you
didn't change; the Labs it turned on stay on. Terms with dots or slashes (file
names) sit at the end, past what Parakeet biasing takes.

**Asked as.** "Why does it write the wrong spelling of my product name?"; "How do I add a word?"; «Как добавить слово в словарь»; «Почему пишет Кубер вместо Kubernetes»; «Словарь не работает».

<!-- sources: docs/features/vocabulary.md:65-80,84-96,177-183; src/rune_ui/vocabulary.rs:143-172,451-480; src/dictation/vocabulary.rs:19-26,230-258,263-296; src/dictation/processor.rs:984-1000; src/transcription/models.rs:348-375; src/transcription/gguf_session.rs:131-157; src/control/cli.rs:155-158,378-402; src/control/host.rs:236-247; src/control.rs:920-955; src/dictation/presets.rs:1-14,59-70; src/rune_ui/labs.rs:456-461,788-799; docs/vocabulary/vibe-coding-ru.txt -->
