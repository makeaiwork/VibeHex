---
feature: copy-only
key: labs.copy_only
title: Copy dictation only
group: Pasting
os: macOS
vibehex: false
---

# Copy dictation only

**What it does.** A finished dictation goes to the clipboard and is not
pasted into the app you are in. You paste it yourself with Command+V when and
where you choose.

**Turn it on when.** You never want a dictation to land in the wrong place or
replace selected text, or you want to look at it before pasting.

**Leave it off when.** You want dictation to type for you; with this on,
nothing appears until you paste.

**Needs.** macOS. Nothing else.

**Good to know.** It covers the dictation shortcut, Dictate without
processing, and `hex dictation` without `--print`. The earlier clipboard is
not restored. "Paste last dictation" and meeting pastes still paste. History
still records the dictation.

**Where.** Labs › Copy dictation only (`labs.copy_only`).

**Asked as.** "Don't paste automatically, just copy"; "Dictation replaces my
selected text"; «Не вставлять текст автоматически, только копировать»;
«Надиктованный текст сам вставляется, а я хочу сначала проверить».

<!-- sources: docs/features/labs.md:56, docs/features/labs.md:199-215; src/labs.rs:302; src/rune_ui/labs.rs:340-345; src/parakeet.rs:972-982; src/paste.rs:234, 284-299 -->
