---
feature: vocabulary-biasing
key: labs.vocabulary_biasing
title: Vocabulary biasing
group: Vocabulary
os: every OS
vibehex: false
---

# Vocabulary biasing

**What it does.** Makes Parakeet-family speech models write the terms from
your Vocabulary (names, products, jargon) while they transcribe, instead of
the ordinary words they sound like. Without it, these models ignore the
Vocabulary during transcription; AI cleanup and search still use it.

**Turn it on when.** You dictate with Parakeet, Underdog Bark, or Nemotron 3.5
and the same names keep coming out wrong even though they are in General ›
Vocabulary.

**Leave it off when.** Your model is Whisper, Qwen3-ASR, or QwenCleo: they
always read the Vocabulary as a hint, so this switch changes nothing for
them. GigaAM, SenseVoice, Cohere, and Apple Speech don't use the Vocabulary
in transcription at all; for them, fix words with AI cleanup or word fixes.

**Needs.** Terms in General › Vocabulary. A Parakeet-family speech model.

**Good to know.** Only the first 64 terms count. A term with spaces helps
only when said in full; give a name said alone its own row. Short terms
under five letters work better with a "sounds like" spelling. How strongly
it prefers terms is the `vocabulary_boost` setting (off, light, normal,
strong; normal by default), changed only from the terminal.

**Where.** Labs › Vocabulary biasing (`labs.vocabulary_biasing`). Related:
General › Vocabulary (`vocabulary`), `vocabulary_boost`.

**Asked as.** "Vocabulary doesn't work with Parakeet"; "Names I added still
come out wrong"; «Словарь не работает, слова всё равно пишутся неправильно»;
«Как заставить модель писать мои термины».

<!-- sources: docs/features/vocabulary.md:1-75; docs/features/labs.md:46, docs/features/labs.md:1391-1396; src/rune_ui/labs.rs:268 -->
