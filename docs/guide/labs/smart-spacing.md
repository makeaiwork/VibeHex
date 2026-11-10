---
feature: smart-spacing
key: labs.smart_spacing
title: Smart spacing
group: Pasting
os: macOS
vibehex: false
---

# Smart spacing

**What it does.** When you dictate right after a word you typed, the
dictation used to stick to it ("alreadyin progress"). With Smart spacing on,
VibeHex looks at the character before the cursor and adds one space when the
text needs it.

**Turn it on when.** You mix typing and dictating in the same sentence, or
often dictate a continuation after a word or punctuation mark.

**Leave it off when.** Nothing to lose: it adds a space only after a letter,
digit, sentence punctuation, or a closing bracket or quote, never after a
space, an opening bracket, or symbols like `@`, `#`, `/`, `-`.

**Needs.** macOS and the Accessibility permission VibeHex already has for
pasting. Nothing else.

**Good to know.** It does nothing when text is selected, when the cursor is
at the very start, or in apps that don't tell VibeHex where the cursor is.
It doesn't change the first letter's case. It doesn't apply with Copy
dictation only, since nothing is pasted.

**Where.** Labs › Smart spacing (`labs.smart_spacing`).

**Asked as.** "My dictation sticks to the previous word"; "Add a space before
dictated text"; «Надиктованный текст слипается с предыдущим словом»; «Нет
пробела после вставки».

<!-- sources: docs/features/labs.md:63, docs/features/labs.md:373-390; src/rune_ui/labs.rs:365 -->
