---
topic: speech-models
title: Speech models
keys: [transcription.model, transcription.language, labs.live_text, labs.vocabulary_biasing, labs.parakeet_ultra, labs.model_in_place]
os: every OS
---

# Speech models

A speech model is the program that turns your voice into text. It runs on your
computer: after a one-time download, dictation needs no internet. Models differ
in languages, speed, size, and accuracy, so you choose.

## Choose or switch a model

1. Open **General › Speech model** (the row shows the current model) and press it.
2. **Compare** shows the best three for your language, with badges: Recommended,
   Fewest wrong words, Fastest, Reads your vocabulary. "N more models" opens the full list.
3. Press **Use**, or **Download** with its size. **Delete** frees the space.

The language is the **Language** row below. A model that doesn't speak it says
that choosing it switches the language to one it does speak (English, or
Russian for GigaAM v3).

## Models you can pick

| Model | Languages | Download | Live text | Vocabulary |
|---|---|---|---|---|
| Parakeet Unified English | English | 731 MB | yes | biasing |
| Parakeet v2 | English | 729 MB | no | biasing |
| Underdog Bark 3.0 Pro, Clean | English | 739 MB | no | biasing |
| Parakeet v3 | 25 European, incl. Russian | 739 MB | no | biasing |
| Nemotron 3.5 ASR | 27 | 751 MB | yes | biasing |
| Whisper large-v3-turbo | almost any, hears which | 886 MB | no | hint |
| Qwen3-ASR 0.6B | 30 | 850 MB | no | hint |
| Cohere Transcribe | 14 | 2.4 GB | no | no |
| GigaAM v3 | Russian | 273 MB | no | no |
| SenseVoice Small | Chinese, Cantonese, English, Japanese, Korean | 252 MB | no | no |
| ivrit.ai Whisper large-v3-turbo | Hebrew | 886 MB | no | hint |
| QwenCleo-ASR 1.7B | Egyptian Arabic | 2.2 GB | no | hint |

Parakeet Ultra (740 MB, 25 languages, biasing) appears after you turn on
**Labs › Parakeet Ultra**. Underdog Bark Clean also drops "um" and "uh".

- **Live text** yes means it writes while you talk, so text is ready on release.
  It needs a graphics chip (Metal on Mac, Vulkan on Linux and Windows) and
  falls back to waiting for release otherwise. Words on screen need **Labs › Live text**.
- **Vocabulary**: "biasing" works only with **Labs › Vocabulary biasing** on;
  "hint" always works; "no" leaves it to AI cleanup, which always reads it.
  See the Vocabulary article.
- The list is the same on every OS.

## Which to pick

- **Russian:** GigaAM v3, small and with punctuation, but it ignores your
  Vocabulary. If names must come out right, use Whisper large-v3-turbo.
- **English:** Parakeet Unified English. Cohere Transcribe makes the fewest wrong words but is a big download.
- **Many languages, or mixed:** Whisper large-v3-turbo or Qwen3-ASR 0.6B with Auto; both wait about 2 s
  after you stop.
- **Slow computer:** choose the model badged Fastest for your language and read "Wait when you stop" in Compare.
- **Offline:** every model here, once downloaded.

## Add your own model file

In the picker, press **Add model file…** and choose a GGUF file (Whisper,
Parakeet, Qwen3-ASR, Cohere, SenseVoice, or GigaAM type; Q8_0, F16, or F32). VibeHex
checks and copies it. It has no live text and shows as Imported, not benchmarked. **Labs › Use model files where they are** adds **Use model file where it is…**,
which copies nothing. Files other apps saved are listed under **Found on this computer**.

**Asked as.** "Which speech model should I use?"; "Can it work offline?"; «Какую модель выбрать для русского?»; «Как сменить модель распознавания»; «Где скачать другую модель».

<!-- sources: src/transcription/models.rs:187-355,372-565,591-657; src/transcription/artifacts.rs; src/rune_ui/picker/facts.rs:231-262; src/rune_ui/general.rs:191; docs/features/README.md:275-295,344-383; docs/features/own-models.md; docs/features/dictation.md:690-694,727-743 -->
