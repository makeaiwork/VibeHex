---
feature: russian-vibe-coding
key: labs.russian_vibe_coding
title: Russian vibe coding
group: Dictation
os: every OS
vibehex: true
---

# Russian vibe coding

**What it does.** For Russian speech full of English tech words, so «клод
код» becomes Claude Code. One switch writes a ready preset into your settings.

**Turning it on adds.** 80 terms (Claude Code, GitHub, MCP) with how a Russian
model hears them, in General › Vocabulary. 8 word fixes in Modes › Global ›
Corrections («тот есть» to «то есть»). Russian cleanup instructions (speech
stays Russian; product names and commands in English; коммит, пуш stay
Cyrillic), with Clean up with AI switched on. It also switches on the other
starred Labs this system has, except Stop shortcut.

**It leaves to you.** Where cleanup runs (Output › AI cleanup › Runs on;
the default, OpenCode, must be running, or text pastes without AI cleanup), keys,
the speech model, and Vocabulary biasing, which stays off. For this speech,
Orukeet or Parakeet v3 (General › Speech model) with Labs › Vocabulary
biasing write the terms as you speak; GigaAM v3 speaks only Russian and
ignores them, leaving them to AI cleanup. Where you already have a fix for the
same phrase, yours stays; your own instructions stay too, with the
preset's added after them.

**Turning it off.** Removes the terms, fixes and instruction text that still
read as the preset wrote them. If those were your only instructions, Clean up
with AI goes off too. The Labs it switched on stay on.

**Where.** Labs › Russian vibe coding (`labs.russian_vibe_coding`).

**Asked as.** "Set up for Russian programming"; «Как настроить, чтобы клод код
писался по-английски»; «Включить словарь для вайб-кодинга».

<!-- sources: src/dictation/presets.rs:1-100; src/transcription/models.rs:348-360; src/rune_ui/labs.rs:284-288; docs/features/labs.md:1218-1240, 26-37; docs/vocabulary/*; src/app_settings/dictation_processing.rs:61-70; src/labs.rs:268-358 -->
