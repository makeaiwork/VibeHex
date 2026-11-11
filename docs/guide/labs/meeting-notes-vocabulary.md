---
feature: meeting-notes-vocabulary
key: labs.meeting_notes_vocabulary
title: Vocabulary in meeting notes
group: Meetings
os: every OS
vibehex: false
---

# Vocabulary in meeting notes

**What it does.** The model that writes a meeting's title and summary can misspell names. With this on, it reads your Vocabulary first, so names and terms come out as you wrote them.

**Turn it on when.** Your meetings mention names, products, or jargon from your Vocabulary.

**Leave it off when.** Nothing to lose; with an empty Vocabulary it changes nothing.

**Needs.** Labs › Meetings (beta) on, and a writer for notes: Summaries in Meetings › Settings › After a meeting, or Meetings › Settings › Notes › Written by. Terms live in General › Vocabulary.

**Good to know.** Only the first 30 terms are sent, each cut to 40 characters. It applies to final titles and summaries, including Regenerate summary, and not to titles shown while recording. Live chapters, when on, read your Vocabulary either way. Turning on Russian vibe coding turns this on too.

**Where.** Labs › Vocabulary in meeting notes (`labs.meeting_notes_vocabulary`).

**Asked as.** "Names are misspelled in meeting summaries"; "Make meeting notes use my vocabulary"; «В саммари встречи неправильно пишутся имена»; «Заметки встречи не знают мои термины».

<!-- sources: src/rune_ui/labs.rs:455-460; src/meeting/notes.rs:954-959; src/meeting/chapters.rs:47-48, 129-143, 611-616; src/dictation/presets.rs:60-67, 172; src/labs.rs:1251; src/rune_ui/platform_settings.rs:260-262; src/rune_ui/meeting_notes.rs:143-207; src/rune_ui/general.rs:214-225; docs/features/labs.md:1690-1700 -->
