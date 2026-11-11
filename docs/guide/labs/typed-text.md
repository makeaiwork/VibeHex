---
feature: typed-text
key: labs.typed_text
title: Transform typed text
group: Output
os: Linux
vibehex: false
---

# Transform typed text

**What it does.** Adds a shortcut that runs the focused app's Output steps, such as AI cleanup, on text you typed, and pastes the result over the whole field.

**Turn it on when.** You want typed drafts cleaned up like dictations.

**Leave it off when.** You work in terminals: it sends Ctrl+A and Ctrl+C.

**Needs.** Linux, and a shortcut you record.

**Good to know.** Mixed formatting is lost. It refuses while Terminal paste is on. Your clipboard comes back afterwards.

**Where.** Labs › Transform typed text (`labs.typed_text`); then Output › Typed text › Transform typed text (`typed_text_hotkey`).

**Asked as.** "Clean up text I typed by hand"; «Обработать набранный вручную текст».

<!-- sources: docs/features/labs.md:150-166; docs/features/dictation.md:306-382; src/rune_ui/labs.rs:656-660; src/rune_ui/output.rs:490-510; src/control/settings.rs:246-249, 613 -->
