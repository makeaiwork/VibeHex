---
topic: output-and-instructions
title: Output, word fixes, and instructions
keys: [dictation_processing.default_mode, dictation_processing.modes, labs.output_fixes]
os: every OS
---

# Output, word fixes, and instructions

After the speech model writes your words, VibeHex can change them before
pasting. The **Output** pane shows those steps. By default there are none and
your words are pasted without AI cleanup.

## See what happens to every dictation

**Output › Every dictation** lists the steps in the order they run, numbered,
each tagged **Built in**, **JS** (your script), or **AI**: **Replace words**
(word fixes), the AI rewrite, then text steps such as Lowercase or No final period. A failed step keeps its input, and a
failed rewrite pastes the text without AI cleanup.

## Change the steps

The pane only shows them. Press **Edit with agent** to copy a prompt that tells
your coding agent (such as Claude Code) where the steps live, then say what you
want ("delete every um", "spell it Kubernetes"). **Open file** opens the settings file.

## Fix a word that comes out wrong

- A **word fix** swaps a heard phrase for fixed text, ignoring case. An empty
  replacement deletes the phrase.
- For names and products, use Vocabulary instead; it also helps transcription.
- **Try** (type a sentence, see "What Hex pastes") previews fixes and the
  built-in text steps; scripts and the AI rewrite run only when you dictate.
  **Labs › Fix results in Output** adds **Vocabulary** and **Fix this**, which
  copies an agent prompt naming the wrong result.

## AI cleanup instructions

With **Labs › Clean up on your own server** on, **Output › AI cleanup** has
**Clean up with AI** and **Custom instructions**, your own rules for how to
write. Engines and models are in the AI cleanup article.

## Different behaviour in some apps

Once a rule exists, **In some apps** lists it by app or website with "Only: …",
the steps used there instead. Global's word fixes still run first. A rule with
no steps pastes without AI cleanup.

## Skip everything once

**Output › Without processing › Dictate without processing** is a second shortcut,
unbound until you record one. It pastes exactly what the model heard.

## Personal commands

`~/.config/hex/hex.config.ts` (create it with `hex commands init`) holds
your own text steps in JavaScript or TypeScript, for changes the built-ins can't do.
A tiny one, "Trim whitespace", is `transform: (text) => text.trim()`. They show
in Output with the **JS** tag and apply once your agent selects them. A script that wants the network
or to run programs waits for **Allow** in Output; until then your words paste unchanged.

**Asked as.** "How do I make it delete 'um'?"; "Replace a word automatically"; «Как заменять слова при диктовке»; «Как задать инструкции для чистки текста»; «Как отключить обработку текста».

<!-- sources: src/rune_ui/output.rs:1-15,98-200,230-260,360-420,470-640,737-780; docs/features/README.md:973-1062; docs/features/dictation.md:217-228; src/dictation/text_replacements.rs; src/rune_ui/output/ai_cleanup.rs:109-130,540-575; site/public/docs/transforms.md; src/personal_commands/hex.config.ts; src/rune_ui/labs.rs:470-475 -->
