---
feature: filler-words
key: labs.filler_words
title: Remove filler words
for: [dictation, code]
ask: Remove ums and ahs from what you dictate?
group: Output
os: every OS
vibehex: false
---

# Remove filler words

**What it does.** Deletes hesitation sounds the speech model wrote down
("um", "uh", "er") from your dictation, with simple rules and no AI model, so
it adds no waiting time. The comma that came with a filler goes too, and
"Um, so I think" becomes "So I think".

**Turn it on when.** Your dictations are full of "um" and "uh" and you do not
use AI cleanup.

**Leave it off when.** You need the text exactly as spoken.

**Needs.** Nothing else.

**Good to know.** It removes only um, umm, uh, uhh, erm, er, and ah, as whole
words, so "umbrella" and "error" stay. These are English fillers; Russian
ones ("ну", "э-э") are not removed. Dictate without processing, meetings, and
live text are unchanged.

**Where.** Labs › Remove filler words (`labs.filler_words`).

**Asked as.** "Remove um and uh from my text"; "Get rid of filler words";
«Убрать слова-паразиты и "эээ"»; «Как убирать "um" из диктовки».

<!-- sources: docs/features/labs.md:59, docs/features/labs.md:271-290; src/labs.rs:332; src/rune_ui/labs.rs:451-456; src/dictation/filler_words.rs:1-10; src/dictation/processor.rs:342-350 -->
