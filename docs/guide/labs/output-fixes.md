---
feature: output-fixes
key: labs.output_fixes
title: Fix results in Output
for: []
ask: Get quick fixes beside Output's test sentence when it's wrong?
group: Output
os: every OS
vibehex: false
---

# Fix results in Output

**What it does.** In Output, the Try card lets you type a sentence and see
what VibeHex would paste for it. This adds a row under it: Vocabulary,
and Fix this once you have typed a sentence, so a wrong result is quick to
correct.

**Turn it on when.** You are tuning how VibeHex writes your words and want a
quick route from a wrong result to a fix.

**Leave it off when.** You do not use the Try card.

**Needs.** Nothing else. Fix this works together with the "Edit with agent"
prompt on that page and your own AI agent.

**Good to know.** Vocabulary opens the same Vocabulary sheet as in General.
Fix this appears once a sentence is typed and copies a prompt, ending with
what you said and what VibeHex pasted, for you to finish and give to your
agent. It reads Copied afterwards. Nothing is saved by the button itself. The
result shown does not include scripts or AI rewriting, which run only when you
dictate.

**Where.** Labs › Fix results in Output (`labs.output_fixes`).

**Asked as.** "Output result is wrong, how do I fix it?"; «Как исправить
неправильный результат в Output»; «Хочу добавить слово в словарь прямо из
Try».

<!-- sources: docs/features/labs.md:75, docs/features/labs.md:705-723; src/labs.rs:327; src/rune_ui/labs.rs:468-473; src/rune_ui/output.rs:539-575, 717-722 -->
