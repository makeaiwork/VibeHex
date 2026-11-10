---
feature: spoken-symbols
key: labs.spoken_symbols
title: Spoken symbols
group: Text cleanup
os: every OS
vibehex: false
---

# Spoken symbols

**What it does.** Turns spoken symbol words into the symbols, attached the way
you would type them. "Hashtag release" becomes "#release", "my underscore
variable" becomes "my_variable", and "done comma thanks" becomes "done,
thanks".

**Turn it on when.** You dictate code, file names, tags, or addresses, or want
punctuation by voice.

**Leave it off when.** You use these words as plain words, since "comma" or
"slash" will always turn into the symbol.

**Needs.** Nothing else.

**Good to know.** English: hashtag, hash sign, at sign, underscore, hyphen,
slash, comma, colon, semicolon. Spanish: arroba, guion bajo, guion, coma, dos
puntos, punto y coma. Other languages, Russian included, are not covered, and
you cannot add your own words. Forms like "commas" are left alone. Dictate
without processing, meetings, and live text are unchanged.

**Where.** Labs › Spoken symbols (`labs.spoken_symbols`).

**Asked as.** "Say hashtag to type #"; "How to dictate an underscore?";
«Как надиктовать знаки: запятая, двоеточие, нижнее подчёркивание»;
«Слово "хэштег" не превращается в #».

<!-- sources: docs/features/labs.md:58, docs/features/labs.md:243-269; src/labs.rs:322; src/rune_ui/labs.rs:445-450; src/dictation/spoken_symbols.rs:1-39; src/dictation/processor.rs:334-342 -->
