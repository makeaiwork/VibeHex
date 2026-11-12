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
that choosing it switches the language to one it does speak (English, or its
own, such as Russian for GigaAM v3 and Norwegian for NB-Whisper Medium).
A new install asks first: setup's first page lists the models for your
language, the recommended one selected (Parakeet Unified English for English,
GigaAM v3 for Russian), and downloads nothing until you press Download.

## Models you can pick

| Model | Languages | Download | Live text | Vocabulary |
|---|---|---|---|---|
| Parakeet Unified English | English | 731 MB | yes | biasing |
| Parakeet v2 | English | 729 MB | no | biasing |
| Underdog Bark 3.0 Pro, Clean | English | 739 MB | no | biasing |
| Parakeet v3 | 25 European, incl. Russian | 739 MB | no | biasing |
| Orukeet | the same 25 as Parakeet v3 | 739 MB | no | biasing |
| Nemotron 3.5 ASR | 27 | 751 MB | yes | biasing |
| Whisper large-v3-turbo | almost any, hears which | 886 MB | no | hint |
| Qwen3-ASR 0.6B | 30 | 850 MB | no | hint |
| Cohere Transcribe | 14 | 2.4 GB | no | no |
| GigaAM v3 | Russian | 273 MB | no | no |
| Granite Speech 5.0 | 7: English, French, German, Spanish, Portuguese, Italian, Japanese | 505 MB | no | no |
| SenseVoice Small | Chinese, Cantonese, English, Japanese, Korean | 252 MB | no | no |
| ivrit.ai Whisper large-v3-turbo | Hebrew | 886 MB | no | hint |
| NB-Whisper Medium | Norwegian | 539 MB | no | hint |
| QwenCleo-ASR 1.7B | Egyptian Arabic | 2.2 GB | no | hint |

Parakeet Ultra (740 MB, 25 languages, biasing) appears after you turn on
**Labs › Parakeet Ultra**. Underdog Bark Clean also drops "um" and "uh".
Orukeet is a Parakeet v3 fine-tune, a bit more accurate in 20 of its 25 languages;
its **Details** button shows the credit and licence (CC BY-SA 4.0).
Granite Speech 5.0 is IBM's, with punctuation. NB-Whisper Medium is a Whisper
tuned for Norwegian by the National Library of Norway.

- **Live text** yes means it writes while you talk, so text is ready on release.
  It needs a graphics chip (Metal on Mac, Vulkan on Linux and Windows) and
  falls back to waiting for release otherwise. Words on screen need **Labs › Live text**.
- **Vocabulary**: "biasing" works only with **Labs › Vocabulary biasing** on;
  "hint" always works; "no" leaves it to AI cleanup, which always reads it.
  See the Vocabulary article.
- The list is the same on every OS; no model here is tied to one system.

## Auto

Set **Language** to Auto and the model detects what you speak. Only models that
can do this list Auto: Whisper large-v3-turbo, Qwen3-ASR 0.6B, Parakeet v3,
Orukeet, Nemotron 3.5 ASR, SenseVoice Small, Granite Speech 5.0, and Parakeet
Ultra when its lab is on. For Auto, Compare shows Whisper large-v3-turbo
(Recommended), Qwen3-ASR 0.6B (Fastest), and Parakeet v3. Parakeet v3, Orukeet,
Parakeet Ultra, and Granite always hear the language from the audio, whatever the row says.

## Which to pick

- **Russian:** GigaAM v3, small and with punctuation, for plain Russian speech.
  It speaks only Russian and ignores your Vocabulary. If names must come out
  right, use Whisper large-v3-turbo.
- **Russian with English tech terms (coding, vibe coding):** Orukeet or
  Parakeet v3 (Orukeet is its fine-tune; in Russian they score about the
  same). Both know Russian and English, hear which from the audio, and take
  your Vocabulary with **Labs › Vocabulary biasing** on. GigaAM v3 only for
  plain Russian speech.
- **English:** Parakeet Unified English. Cohere Transcribe makes the fewest wrong words but is a big download.
- **Norwegian:** NB-Whisper Medium (Recommended for Norwegian), with Whisper large-v3-turbo as the second choice.
- **Many languages, or mixed:** Whisper large-v3-turbo or Qwen3-ASR 0.6B with Auto; both wait about 2 s
  after you stop.
- **Slow computer:** choose the model badged Fastest for your language and read "Wait when you stop" in Compare.
- **Offline:** every model here, once downloaded.

## Add your own model file

In the picker, open the full list ("N more models") and press **Add model file…**, then choose a GGUF file (Whisper,
Parakeet, Qwen3-ASR, Cohere, SenseVoice, or GigaAM type; Q8_0, F16, or F32). VibeHex
checks and copies it. It has no live text and shows as Imported, not benchmarked. **Labs › Use model files where they are** adds **Use model file where it is…**,
which copies nothing. Files other apps saved are listed under **Found on this computer**.

**Asked as.** "Which speech model should I use?"; "Can it work offline?"; «Какую модель выбрать для русского?»; «Какая модель лучше для русского с английскими терминами?»; «Как сменить модель распознавания»; «Где скачать другую модель».

<!-- sources: src/transcription/models.rs:166-178,300-320,335-360,436-657,672-752; src/transcription/model_benchmarks.rs:183-186; src/transcription/model_details.rs:72-120; src/transcription/artifacts.rs:44-60,191-224; src/transcription/custom_models.rs:37-47; src/transcription/gguf_session.rs:237; src/rune_ui/picker/facts.rs:215-238,170-182; src/rune_ui/picker/compare.rs:22-50,89-150; src/rune_ui/picker/list.rs:268-280; src/rune_ui/picker.rs:540; src/rune_ui/general.rs:141-210; src/rune_ui/model_details.rs:1-6; docs/features/README.md:341-352; docs/features/own-models.md -->
