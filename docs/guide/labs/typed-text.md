---
feature: typed-text
key: labs.typed_text
title: Transform typed text
for: [dictation, cleanup]
ask: Clean up text you typed, just like a dictation?
then: Record its shortcut in Output › Typed text
group: Output
os: Linux
vibehex: false
---

# Transform typed text

**What it does.** Adds a shortcut that runs the focused app's Output steps, such as AI cleanup, on text you typed, and pastes the result over the whole field. Spoken formatting, spoken symbols and filler words are not applied, since they are for dictation.

**Turn it on when.** You want typed drafts cleaned up like dictations.

**Leave it off when.** You work in terminals: it sends Ctrl+A and Ctrl+C.

**Needs.** Linux, and a shortcut you record. On a desktop without the shortcuts portal, run `hex typed-text` instead.

**Good to know.** It replaces the whole field, so mixed formatting is lost. It refuses while Terminal paste is on, because the paste would become Ctrl+Shift+V. Your clipboard comes back afterwards. Off by default.

**Where.** Labs › Transform typed text (`labs.typed_text`); then Output › Typed text › Transform typed text (`typed_text_hotkey`), unbound until you record a key.

**Asked as.** "Clean up text I typed by hand"; "Run AI cleanup on typed text"; «Обработать набранный вручную текст»; «Исправить текст, который я напечатал».

<!-- sources: src/rune_ui/labs.rs:656-661; src/labs.rs:474-476; src/rune_ui/output.rs:228-232, 487-507; src/linux/typed_text.rs:114-122; src/linux/app/control.rs:107-122; docs/features/dictation.md:306-382; docs/cli.md:138-143; src/control/settings.rs:246-250, 611-615 -->
