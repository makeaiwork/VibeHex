---
feature: spoken-formatting
key: labs.spoken_formatting
title: Spoken formatting
for: [dictation]
ask: Format by voice, saying new line or new paragraph?
group: Output
os: every OS
vibehex: false
---

# Spoken formatting

**What it does.** Lets you format by voice. Say "new line" or "line break"
for a line break, "new paragraph" for a blank line, and "open quote" and
"close quote" for quotation marks. The commas and periods the model put
around the command go with it.

**Turn it on when.** You dictate emails, notes, or messages and want line
breaks and quotes without touching the keyboard.

**Leave it off when.** You often say these phrases as ordinary words, since
they would turn into formatting.

**Needs.** Nothing else.

**Good to know.** It understands English and Spanish ("nuevo párrafo", "nueva
línea", "abre comillas", "cierra comillas") whatever the Language setting is;
Russian and other languages are not covered. Words like "newline" or
"lineal" are left alone. Dictate without processing, meetings, and live
text are unchanged.

**Where.** Labs › Spoken formatting (`labs.spoken_formatting`).

**Asked as.** "How do I dictate a new line?"; "Say new paragraph to start a
paragraph"; «Как надиктовать новую строку или абзац»; «Голосовая команда
"новый абзац" не работает».

<!-- sources: docs/features/labs.md:57, docs/features/labs.md:218-241; src/labs.rs:321; src/rune_ui/labs.rs:439-444; src/dictation/spoken_formatting.rs:1-27; src/dictation/processor.rs:324-333 -->
