---
topic: assistant
title: Assistant
keys: [assistant.enabled, assistant.engine, assistant.read_recordings, assistant.keep_conversations]
os: every OS
---

# Assistant

Assistant, at the bottom of the sidebar, answers questions about VibeHex from
this guide, and, once you allow it, questions about your own meetings, Files
texts, and dictations: "What did we decide about the budget in November?",
"Summarize my calls with Anna last week", "What did I dictate about the
release yesterday?".

## Ask about your recordings

The first time a question is about your recordings, before anything is read,
Assistant asks **Read your meetings, files, and dictations?** and names the
model the passages would go to. **Allow** is remembered; **Answer without
recordings** answers from the guide alone. Questions about how VibeHex works
never ask.

Once allowed, it searches on this computer for the meetings, Files texts, and
History dictations that match the question's time ("yesterday", "last week",
"the last five meetings") and words, and reads their notes and the best
passages. While it works, its steps appear one by one; the answer then folds
them into one line, such as "Read 4 meetings and 1 file in 9 s", which opens
on a click. If the recordings don't say, the answer says so.

## Go to the place cited

Each numbered mark in the answer, and each chip under **From your
recordings**, opens its place: a meeting on Transcript with the passage
tinted and the searched words lit, a Files text on its Original, or the
dictation in History. **Back to Assistant**, at the top of that pane, returns
to the same spot in the conversation; Cmd-[ on a Mac and Ctrl+[ elsewhere do
the same.

## What leaves your computer

The question, the conversation so far, and guide text go to the model in
**Assistant › Settings › Runs on**. With your leave, so do the passages it
found, a few thousand characters for a built-in model and more for larger
ones; recordings themselves are never sent. A model on this computer keeps
everything here; Codex sends to OpenAI, OpenRouter to its provider.

## Turn it off

**Assistant › Settings › What it reads › Your meetings, files, and
dictations** takes the permission back; the next such question asks again.
**Conversations › Keep on this computer** and **Clear conversations…** control
what Assistant keeps.

**Where.** Assistant › Settings (`assistant.read_recordings`,
`assistant.engine`, `assistant.keep_conversations`); the sidebar row
(`assistant.enabled`).

**Asked as.** "Can it search my meetings?"; "Does Assistant read my
dictations?"; «Может ли ассистент искать по моим встречам?»; «Как запретить
ассистенту читать мои записи?».

<!-- sources: docs/features/assistant.md; src/assistant/intent/mod.rs; src/assistant/recordings/mod.rs; src/rune_ui/assistant/recall.rs; src/rune_ui/assistant/jump.rs; src/rune_ui/assistant/settings.rs -->
