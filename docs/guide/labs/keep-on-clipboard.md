---
feature: keep-on-clipboard
key: labs.keep_on_clipboard
title: Keep dictation on clipboard
group: Pasting
os: macOS
vibehex: false
---

# Keep dictation on clipboard

**What it does.** Normally VibeHex pastes a dictation through the clipboard
and, half a second later, puts back what you had copied before. With this on,
the dictation stays on the clipboard after pasting and the earlier contents
are not restored.

**Turn it on when.** You want to paste the same dictation again somewhere
else, or use a clipboard manager that should record dictations.

**Leave it off when.** You copy something, dictate, and expect to paste the
copied thing afterwards.

**Needs.** macOS. Nothing else.

**Good to know.** The dictation is saved as an ordinary clipboard entry, so
clipboard managers may record it. It still gets pasted; to only copy, use
Labs › Copy dictation only.

**Where.** Labs › Keep dictation on clipboard (`labs.keep_on_clipboard`).

**Asked as.** "Dictation disappears from my clipboard"; "I want to paste the
dictation again"; «Хочу, чтобы надиктованный текст оставался в буфере
обмена»; «После диктовки пропадает то, что я копировал».

<!-- sources: docs/features/labs.md:55, docs/features/labs.md:183-197; src/labs.rs:301; src/rune_ui/labs.rs:333-338; src/paste.rs:259, 342 -->
