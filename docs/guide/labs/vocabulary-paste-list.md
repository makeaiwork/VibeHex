---
feature: vocabulary-paste-list
key: labs.vocabulary_paste_list
title: Vocabulary text view
group: Vocabulary
os: every OS
vibehex: true
---

# Vocabulary text view

**What it does.** Adds a Text view button to the Vocabulary window. It shows
the whole list as plain text, one term per line, written
`Term | sounds like, sounds like`. You can paste a long list, fix many terms,
or delete many at once, instead of editing row by row.

**Turn it on when.** You have a list of names to paste in, or a long list to
clean up.

**Leave it off when.** Your list is short; the normal rows are enough.

**Needs.** Nothing else.

**Good to know.** Save makes the list exactly the text: an edited line fixes
its term, a deleted line removes it, a pasted line adds it, and repeated terms
merge. Cancel or Escape saves nothing. Lines starting with `#` are skipped. A term is cut to 64 characters, has at most 16 spellings, and the
list holds 500 terms. Turning it off leaves the list as it is.

**Where.** Labs › Vocabulary text view (`labs.vocabulary_paste_list`). The
button is in General › Vocabulary, beside Add term.

**Asked as.** "How do I paste many words into the Vocabulary?"; "Edit the
Vocabulary as text"; «Как добавить много слов в словарь сразу»; «Как вставить
список терминов в словарь».

<!-- sources: docs/features/labs.md:1145-1163; docs/features/vocabulary.md:25-35; src/rune_ui/labs.rs:278-282; src/rune_ui/vocabulary.rs:176-260; src/dictation/vocabulary.rs:17-22, 344-357, add_clamped -->
