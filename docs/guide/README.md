# VibeHex Guide

The Guide is what Assistant (the in-app helper) knows about VibeHex. It is
written for people who use the app, not for people who build it: what each
feature does, when to turn it on, what it needs, and where it lives in the
window. Assistant answers from it first and goes to the web only when it has
no answer (mockups: `docs/design/assistant/`, canvas
<https://claude.ai/artifact/8nnvDfmN3VNNpdrBk4ntPJ>).

The Guide is built into the app, so a question never needs the network to
find VibeHex's own answer. Large models get the whole Guide in their prompt;
small built-in models get the articles a search picks. Both work only while
the Guide stays small: about 25,000 tokens for everything. Keep articles short.

## Layout

```text
docs/guide/
├── README.md            this file: rules and the article template
├── labs/<name>.md       one article per Labs feature; <name> is its `hex labs` name
└── topics/<slug>.md     one article per area of the app (models, meetings, …)
```

Every `labs::Feature` must have `labs/<name>.md` whose `key` is the feature's
`hex settings` key. A test enforces this, so a new Labs feature cannot ship
without its article.

## Who reads it

Someone who installed VibeHex, speaks Russian or English, may not know what
a "speech model" or an "OpenAI-compatible server" is, and asks Assistant
things like "can it record my Zoom calls?" or "why does it write Кубер instead
of Kubernetes?". The Guide is in English, the language of the app's UI;
Assistant answers in whatever language it is asked.

## Which language Assistant answers in

- The language of the person's latest message, not the system's or the
  window's: someone whose Mac is in English may well ask in Russian.
- A message with no clear language ("ok", a setting's name) keeps the
  conversation's language.
- Before the person has written anything (the first-launch tip, an empty
  conversation), Assistant speaks the VibeHex window's language.
- Names of panes, buttons, and settings stay as the window shows them
  (**Labs › Meetings**, **Always record**), so they can be found on screen.
  That is why "Asked as" lines carry Russian phrasings while the rest of
  each article stays in English.

## Rules

- **Facts only from the source.** Every claim must be checkable against
  `docs/features/*.md` or the code. When the docs and the code disagree, the
  code wins; mention the disagreement in your report, not in the article.
  If you cannot confirm something, leave it out.
- **User words, not code words.** Name things as the window names them:
  pane › section › row, the row's exact title ("Labs › Smart spacing",
  "Output › AI cleanup › Runs on"). No module names, file paths, function or
  test names, issue numbers, or JSON. Settings keys appear only in the
  front matter and in the **Where** line.
- **Say what it is for.** Lead with the problem it solves, in one or two
  sentences a non-engineer understands, then how it behaves.
- **Be honest about limits.** OS, which speech models it works with, other
  features it needs, permissions macOS asks for, downloads and their size,
  what leaves the computer, and what it breaks or changes.
- **Short.** A Labs article: 80–200 words of body. A topic article:
  up to 450 words. No filler, no marketing, no "simply".
- **No `·` (middle dot) anywhere.** Use commas, "and", or a new line.
- Write the Labs row's title exactly as the Labs page shows it.

## Labs article template

```markdown
---
feature: smart-spacing            # `hex labs` name, same as the file name
key: labs.smart_spacing           # its `hex settings` key
title: Smart spacing              # the Labs row title, exactly
group: Pasting                    # the Labs section it sits in
os: macOS                         # macOS | Windows | Linux | every OS, or a list
vibehex: false                    # true when VibeHex added it (a star in Labs)
---

# Smart spacing

**What it does.** One or two sentences: the problem, then the behaviour.

**Turn it on when.** Who benefits, in concrete situations.

**Leave it off when.** Who it would bother, or "Nothing to lose; it only …".

**Needs.** OS, model, other settings, permissions, downloads. "Nothing else." if none.

**Good to know.** Side effects, interactions with other features, what off does.
Omit the line when there is nothing worth saying.

**Where.** Labs › Smart spacing (`labs.smart_spacing`). Related: Output › …
(`other.key`) when another setting matters.

**Asked as.** "Words stick together when I dictate after typing"; «Слова слипаются с предыдущим словом»;
two to four phrasings a person might use, at least one in Russian.

<!-- sources: docs/features/labs.md:373-390; src/paste.rs (needs_leading_space) -->
```

## Topic article template

```markdown
---
topic: ai-cleanup
title: AI cleanup and models
keys: [default_mode.post_processing.enabled, dictation_processing.server.runs_on]
os: every OS
---

# AI cleanup and models

One opening paragraph: what this part of the app is for.

## <Short task-shaped heading, e.g. "Connect a model">

Plain steps or a short list, naming the window's rows exactly.

**Asked as.** …

<!-- sources: … -->
```

The `<!-- sources -->` comment is for review and upkeep: the app strips it
before Assistant reads the article. List the doc line ranges and code files
each claim came from.
