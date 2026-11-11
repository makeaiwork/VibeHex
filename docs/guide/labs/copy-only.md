---
feature: copy-only
key: labs.copy_only
title: Copy dictation only
group: Output
os: [macOS, Windows]
vibehex: false
---

# Copy dictation only

**What it does.** A finished dictation goes to the clipboard and is not
pasted into the app you are in. You paste it yourself (Command+V on a Mac,
Ctrl+V on Windows) when and where you choose.

**Turn it on when.** You never want a dictation to land in the wrong place or
replace selected text, or you want to look at it before pasting.

**Leave it off when.** You want dictation to type for you; with this on,
nothing appears until you paste.

**Needs.** macOS or Windows. Nothing else. Linux has no such lab.

**Good to know.** It covers the dictation shortcut, Dictate without
processing, and `hex dictation` without `--print`. The earlier clipboard is
not restored. "Paste last dictation" still pastes, and on a Mac so do meeting
pastes. History still records the dictation. On Windows the dictation is an
ordinary clipboard entry, so Win+V history may keep it.

**Where.** Labs › Copy dictation only (`labs.copy_only`).

**Asked as.** "Don't paste automatically, just copy"; "Dictation replaces my
selected text"; «Не вставлять текст автоматически, только копировать»;
«Надиктованный текст сам вставляется, а я хочу сначала проверить».

<!-- sources: docs/features/labs.md:56,199-215; src/labs.rs:492-494; src/rune_ui/labs.rs:440-460; src/parakeet.rs:1005-1015,1077,1123; src/paste.rs:234-235,284-299; src/windows/dictation.rs:1096-1108; src/windows/paste.rs:237-252; src/linux/dictation.rs:230-233 -->
