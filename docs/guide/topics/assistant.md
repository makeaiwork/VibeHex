---
topic: assistant
title: Assistant
keys: [assistant.enabled, assistant.engine, assistant.read_recordings, assistant.keep_conversations, assistant.send_settings, assistant.web]
os: every OS
---

# Assistant

Assistant, at the bottom of the sidebar, answers questions about VibeHex from
this guide, and, once you allow it, questions about your own meetings, Files
texts, and dictations: "What did we decide about the budget in November?",
"What did I dictate about the release yesterday?".

## Its language

Assistant speaks the language you dictate in: with a specific speech
recognition language, its example questions, setup's questions and chips,
card buttons, and answers are in it; on Auto, the window's language. Write in
another language and it answers in that one. Places it names keep the
window's words. Without a translation (Italian, say) its buttons stay
English.

## Changes it suggests

When a setting would do what you asked, the answer ends with a card: where
the setting lives, its value now and the one suggested, and why. Nothing
changes until you press the card's button; **Not now** leaves it as it is.
What the change needs first, a Labs feature or Meetings, is turned on in the
same save ("Also turns on…"), and a shorter retention says what it removes.
A new speech model downloads first and switches when ready.

No card comes for a value you already have, a feature this computer lacks,
or an ordinary word as a term: **What Assistant knew** lists what was left
out.

**Undo** puts back everything a card changed, while the conversation stays
open; after that the card offers **Show in…**. A card follows your
settings: changed by hand since, it says so.

For the AI cleanup instructions, the card shows the lines that change.
**Edit first** opens the new text in the card itself, to adjust before
**Save instructions**; **Cancel edit** goes back to the lines.

Names your dictation gets wrong come as one **Add 3 terms** card: each term
with what it sounds like, a checkbox, and **Edit** to fix either. Terms
already in your Vocabulary are left out.

Ask why dictation wrote one word as another ("it writes AI cleanup when I
say AI") and, with Your current settings on, Assistant reads your cleanup
instructions (every mode's) and the Vocabulary entries your words name, and
looks for the cause there. Such a question never searches the web.

## Set VibeHex up for you

**Set VibeHex up for me**, on Assistant's start screen, or asking "help me
set VibeHex up", first says what is set now: the model that hears you and its language,
where dictation is cleaned up, who writes meeting notes, the Labs features on,
and how many Vocabulary terms you have. Then four questions, already answered
from your settings: what you use VibeHex for, your language (one with
English tech terms mixed in is its own choice), where dictation is cleaned up (this
Mac, Codex, OpenCode, OpenRouter, your own server such as LM Studio or Ollama,
or nowhere), and, if you record calls or files, where notes and files are
worked on. A few yes-or-no questions about fitting Labs features follow;
**Skip to the plan** goes straight on.

Then comes one list of changes, grouped by area, each with a checkbox, what it
changes from and to, and what it takes: a download and its size, a ChatGPT
sign-in, or a macOS permission. A setting you chose yourself is never ticked
for you ("You chose …"), an engine not set up yet says where to finish, and a
model that already hears your language stays. What already fits is
folded under **Already set**; a long value folds to three lines (**Show
all**). Untick what you don't need and press **Apply 3 changes and 81
terms** (Vocabulary terms are counted apart). A change that can't be made
says why, and the rest still apply; **Undo** takes the whole list back.

Setting up again, after a setup or once you have used VibeHex a while (50
dictations, or any meeting), skips the first questions: it says what came
since your last setup, what is new in VibeHex, and, if Assistant may read
your recordings, what your use shows (how much you dictate and where, how
many are long, the words cleanup keeps fixing, terms Vocabulary lacks, as
one **Add N terms** card). Your earlier answers stay folded under **Has
anything changed in how you use VibeHex?**: **Continue** goes on to the
plan, **Change answers** opens them. Something you declined before isn't
offered again until that setting changes.

**Or set it up like Denis**, under it, or asking "set me up like Denis" («сделай
настройки как у Дениса»), offers the settings of the person who makes VibeHex
as one such list: his Labs features, speech model, cleanup and notes engines,
cleanup instructions, and Vocabulary terms you don't have yet. Keeping meetings
for a month starts unticked. Then it shows his shortcuts and asks whether they
suit you: **Use these**, **Keep mine**, or type the key you'd like.

That preset is his setup for dictating in one language, so it is offered
only when Russian is the recognition language in **General › Language**,
whatever the speech model. With Auto or another language there is no such
row and asking for it sets nothing up; use **Set VibeHex up for me**.

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
tinted and the phrases it cited lit, a Files text on its Original the same
way (also while you listen to it), or the dictation in History, its row
marked with the number. **Back to Assistant**, at the top of that pane, returns
to the same spot in the conversation; Cmd-[ on a Mac and Ctrl+[ elsewhere do
the same.

## What leaves your computer

The question, the conversation so far, and guide text go to the model in
**Assistant › Settings › Runs on**. With your leave, so do the passages it
found, a few thousand characters for a built-in model and more for larger
ones; recordings themselves are never sent. A model on this computer keeps
everything here; Codex sends to OpenAI, OpenRouter to its provider.
On Codex, when the guide has no answer (setting up LM Studio, say), it may
search the web through OpenAI, never with anything from your recordings or
settings, and names the pages under **From the web**; **Assistant ›
Settings › What it reads › The web**, or **Web** in the field for one
conversation, turns that off.

## Turn it off

**Assistant › Settings › What it reads › Your meetings, files, and
dictations** takes the permission back; the next such question asks again.
**Conversations › Keep on this computer** and **Clear conversations…** control
what Assistant keeps.

**Where.** Assistant › Settings (`assistant.read_recordings`,
`assistant.engine`, `assistant.keep_conversations`, `assistant.web`); the
sidebar row (`assistant.enabled`).

**Asked as.** "Can it search my meetings?"; "Does Assistant read my
dictations?"; "Can Assistant change settings for me?"; "Set VibeHex up for
me"; "Can Assistant search the web?"; «Может ли ассистент искать по моим встречам?»; «Ассистент ходит в интернет?»; «Как запретить
ассистенту читать мои записи?»; «Помоги мне настроить VibeHex под меня»;
«Добавь Kubernetes в словарь».

<!-- sources: docs/features/assistant.md; src/assistant/web.rs; src/rune_ui/assistant/web.rs; src/assistant/intent/mod.rs; src/assistant/intent/setup.rs; src/assistant/setup/mod.rs; src/assistant/setup/follow.rs; src/assistant/setup/plan.rs; src/assistant/setup/preset.rs; src/assistant/changes/vocab.rs; src/rune_ui/assistant/card.rs; src/rune_ui/assistant/plan.rs; src/rune_ui/assistant/vocab.rs; src/assistant/recordings/mod.rs; src/rune_ui/assistant/recall.rs; src/rune_ui/assistant/jump.rs; src/rune_ui/assistant/settings.rs -->
