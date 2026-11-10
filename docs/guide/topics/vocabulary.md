---
topic: vocabulary
title: Vocabulary
keys: [vocabulary, vocabulary_boost, labs.vocabulary_biasing, labs.vocabulary_paste_list, labs.vocabulary_quick_add, labs.russian_vibe_coding]
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

## Who uses it

- **AI cleanup and search** always use it, with every model.
- **Whisper, Qwen3-ASR, and QwenCleo** read the first terms as a hint while
  transcribing (always on). Their "sounds like" spellings are not sent.
- **Parakeet, Underdog Bark, and Nemotron 3.5** use it only with **Labs ›
  Vocabulary biasing** on (see that article).
- **GigaAM, SenseVoice, Cohere** don't use it in transcription. The row says so;
  fix words with AI cleanup or word fixes (Output article).

## Limits

- Transcription takes the first 64 terms. The list holds up to 500 terms,
  with 64 characters per term or spelling and 16 spellings per term.
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
cleanup instructions, and turns on Clean up with AI and the other VibeHex Labs
features except Stop shortcut. Turning it off removes what it added and you
didn't change. Terms with dots or slashes (file names) sit at the end, for AI cleanup only.

**Asked as.** "Why does it write the wrong spelling of my product name?"; "How do I add a word?"; «Как добавить слово в словарь»; «Почему пишет Кубер вместо Kubernetes»; «Словарь не работает».

<!-- sources: docs/features/vocabulary.md:1-135; src/rune_ui/vocabulary.rs:143-160,271-505; src/dictation/vocabulary.rs:19-26; src/rune_ui/labs.rs:268-306; docs/features/labs.md:1218-1240; docs/vocabulary/vibe-coding-ru.txt -->
