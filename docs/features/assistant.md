# Assistant (every OS)

A chat at the bottom of the sidebar that answers questions about VibeHex from
its guide (`docs/guide`, carried in the binary): whether it can do something,
how to set it up, which model to use. Once the person allows it, it also
answers questions about their own meetings, Files texts, and dictations,
citing the places it read. It is on by default and not in Labs, an
exception the owner chose; `assistant.enabled` hides the row.

```ts
Sidebar                                          // rune_ui::sidebar, rune_ui::assistant::nav
└── Assistant   below the spacer, above the version footer (nav-assistant):
                a 32 pt row with a 24 pt face (VibeHex's hexagon with two
                eyes); open, the face and word brighten, no fill. On the rail
                the last button, 36 by 32, with its own tooltip
Assistant                                        // rune_ui::assistant
├── Header      Chat | Settings; in Chat, the conversation picker and +.
│               A narrow window cuts the picker's name, never the title
├── Chat
│   ├── New     "Ask anything about VibeHex", after an update "Updated to
│   │           <version>" (a neutral row: a sparkles tile, "Ask me what's
│   │           new…", a chevron), "Set VibeHex up for me" (an
│   │           accent-tinted row: a sliders tile, a line on what it does,
│   │           a chevron), with Russian as the recognition language "Or set it up like
│   │           Denis, who makes VibeHex ›" (quiet text under it; no such
│   │           row with Auto or another language), four example questions, and what
│   │           the guide holds (Labs features and topics)
│   ├── Feed    the person's bubble; the answer as Markdown beside the face,
│   │           then "From the guide" chips naming the articles it drew on
│   │           and "From the web" chips (one per site it cites, warm tint;
│   │           a press opens the page), then Copy, Ask again, the model,
│   │           Stopped, the time.
│   │           From the web: "Read the guide and 2 web pages in 9 s ›"
│   │           over the answer; open, the guide it read, that the guide
│   │           didn't cover it all, "Searched the web: 2 pages read", and
│   │           each search's words. While it searches: "Searched the web
│   │           for “…”", "Read lmstudio.ai", "Searching the web".
│   │           A cloud model (Codex, OpenRouter) silent 75 s after its
│   │           turn starts (no word, reasoning, or search) is stopped and
│   │           asked once more, "No reply yet, asking again" in the steps;
│   │           its deadline counts from the new turn.
│   │           A failed answer says why and offers Try again.
│   │           From recordings: "Read 4 meetings and 1 file in 9 s ›" over
│   │           the answer (a click lists the steps), and with the guide
│   │           cited too "Read 8 dictations and 2 guide articles in 29 s",
│   │           its step "Read 2 guide articles: Vocabulary, AI cleanup";
│   │           [n] marks inline, and
│   │           "From your recordings" chips (title, date, time).
│   │           A place in the window named in bold (General › Vocabulary,
│   │           Labs › Smart spacing, Output) reads in the link colour and
│   │           opens it; other bold words are full-brightness text
│   ├── Consent the first question about recordings, before anything is
│   │           read: "Read your meetings, files, and dictations?", the model
│   │           the passages go to, Allow or Answer without recordings
│   ├── Steps   while it searches: one line per step, a check when done,
│   │           three dots on the current one, the face reading; the
│   │           dots wave (the bright one a → b → c, 380 ms a step,
│   │           `assistant::dots`) in Assistant, Files and Meetings'
│   │           Requests (and "No reply yet, asking again"), still under
│   │           Reduce Motion and in scenes, no wakeups while nothing is live
│   ├── Setup   "Now: …" (what is set: speech model and language,
│   │           dictation cleanup, meeting notes, Files cleanup, Labs on,
│   │           Vocabulary terms), then round one as one form: four
│   │           questions, captioned chips in three columns, preselected
│   │           from the settings, one Continue. Each question wraps to
│   │           the card's width, its hint on its own line under it; a
│   │           captioned chip's long label takes a second line. Follow-up rounds (Yes /
│   │           No, at most four a round) with a quiet "Skip to the plan".
│   │           Answered rounds stay as small chosen chips. Then
│   │           "Suggested changes, 6 changes, 5 selected": rows grouped
│   │           by area (Dictation, Code, Vocabulary, Meetings, Files,
│   │           Models), each a
│   │           checkbox (accent fill and a check, or an empty box with a
│   │           grey border), name, where, why, from → to (a long value
│   │           in three lines with "Show all"), "You chose …"
│   │           over the person's own value, a warning where setup isn't
│   │           finished, "Then: …"; a folded "Already set: 6 things ›";
│   │           Not now and "Apply 5 changes"; after, a mark per row and
│   │           "4 of 5 changes applied" with Undo. Vocabulary terms are
│   │           counted apart: "3 changes and 81 terms, 3 and 81
│   │           selected", "Apply 3 changes and 81 terms"
│   ├── Coming  "Looking in the guide" as a step going on (three moving
│   │           dots, like a search), then the draft as it streams, and
│   │           "Preparing a change" the same way while a card is read;
│   │           the face reads (eyes drift along a line) until it lands
│   ├── Field   "Ask about VibeHex or your recordings", the model and reasoning menus (for this
│   │           conversation only), Send, or Stop while answering
│   └── Connect with no model to ask, stands in for the field:
│               Codex (Sign in with ChatGPT, or its install command),
│               OpenCode (Use OpenCode while it runs), OpenRouter or your
│               own server and Built in (macOS) open Output › AI cleanup,
│               or the Labs row while AI cleanup's lab is off
└── Settings
    ├── Model           Runs on (default "Same as AI cleanup"); Reasoning
    │                   while it runs on Codex
    ├── Conversations   Keep on this computer; Clear conversations… then
    │                   Really clear all?
    └── What it reads   Your current settings (toggle, on: the model reads
                        switches, choices, numbers, shortcuts, and speech models);
                        Your meetings, files, and dictations (toggle)
Opened from an answer                            // rune_ui::assistant::jump
├── Header      Back to Assistant (chevron, face, words) before the title
├── Bar         an item cited more than once: "From the answer: <question>",
│               "1 of 2", up and down between places, a cross to clear
└── Place       tinted in the accent, round corners, the answer's [n] by the
                speaker; inside, the cited phrases at full strength over a
                thin soft accent underline and the rest of the passage muted
                (no fill behind the words; without
                cited phrases the passage reads whole); others cited fainter;
                the first cited words a third of the way down the pane,
                however deep in a long passage. A dictation: its History
                row tinted with the [n] before its title
Opened from a place link                         // rune_ui::assistant::places
├── Header      Back to Assistant (not on the Labs page)
└── Titles      the section and row the path names, their words lit in
                the accent; the deepest lands a third of the way down, its
                Advanced fold opened when it is under one; a lab: Labs on
                its group, its card's title lit. Lit until another pane
```

## Language

- Assistant's default language (`assistant::language`) is the speech
  recognition language when it is a specific one (`transcription.language`,
  or the one language its speech model knows, GigaAM's Russian), else the
  window's. It drives Assistant's own words, never the rest of the window:
  the Chat view's feed (`rune_ui::assistant::chat` builds it inside
  `i18n::speaking`): the start page and its example questions, What's new,
  setup's questions, chips, "Now:", and plan, the consent card, steps,
  "From the guide", card buttons and statuses, the field's placeholder;
  and the words presses write into a conversation (setup, the preset,
  What's new's question, a card's apply), each under `language::speak`.
  The worker builds setup's words and "Now:" under it too. The header,
  Settings view, model menus, and the connect panel keep the window's.
- `i18n::speaking` sets a column for one thread (`i18n::speaking.rs`),
  read by every `tr`; `tr_window` gives a place's name in the window's
  words inside it: a card's where and name, "General › Vocabulary", the
  preset's shortcut places, a pane in "Now set to…, in Output". A language
  with a table (Polish, Chinese, Japanese, German, Spanish, Ukrainian,
  French, Portuguese, Russian; Cantonese reads Traditional Chinese) shows
  its rows; any other (Italian) shows English.
- The prompt says the default language by name (`prompt::default_language`):
  setup's questions, sentences, and plans when nothing was typed. It isn't
  the language of an answer: a question clearly in another language is
  answered in the question's, whatever the earlier turns were; a short
  unclear one keeps the conversation's. A question's prompt also ends on
  that rule (`prompt::LATEST`, the line after `<question>`): a model that
  has just read English pages and Russian turns otherwise answers an
  English question in Russian.
- Setup's follow-up questions (each Labs article's `ask:` and `ask_if:`
  line) have rows in every language (`i18n/vibehex/follow.rs`, a test
  holds every one).
- Scene `assistant-start-russian`: an English window dictating Russian.
  The setup scenes play the owner's settings (Orukeet, Russian), so
  their questions and chips show in Russian too.

## Behaviour

- One question is answered at a time, on Assistant's own thread
  (`assistant::worker`), never behind a Files job. Stop keeps what came.
- Search (`assistant::retrieve`, BM25 with a Russian glossary) picks the
  articles a model reads: 3 for a local model, 6 otherwise, with the guide's
  table of contents and the conversation so far (`assistant::prompt`). The
  model names its sources on a last line the pane turns into chips.
- The model is Assistant's own: `assistant.engine`, else the engine
  dictation's AI cleanup uses. The field's menus change it for one
  conversation; nothing here changes what dictation runs on.
- Codex starts only when a question set to Codex is asked, Sign in with
  ChatGPT is pressed, or the field's model menu opens to list models.
  Whether Codex is installed is looked up off the UI thread (a file search,
  no process); OpenCode is asked whether it runs only while Assistant would
  use it, while the Chat view shows in a visible window: every 5 s while the
  connect panel shows, else every minute.
- The face: pointed at, it blinks once (0.34 s); pressed, it squishes and
  springs back (0.46 s); while an answer comes its eyes read. Under Reduce
  Motion nothing moves and reading eyes rest to one side.

- Places in answers (`rune_ui::assistant::strong`, `places`): a finished
  answer's `**…**` that names a place becomes a link before it renders.
  Names match the window's words, translated or English, ignoring case: the
  shown panes, Labs and each lab's title, and the paths of the settings cards
  know (`changes::names::WHERE`); a path resolves through its first part, a
  pane's Settings view, then titles to light. A one-word name other than a
  pane or a lab needs its path, and a name two places share stays bold.
  Bold inside code, links, or with markup in it is left alone, and a draft
  while it streams is not rewritten. Every other bold word, here and in
  Files' and meetings' answers (`files::session::markdown`), is drawn in the
  base text colour over the softer reading colour, as a highlight, so it
  takes no hand cursor or Tab stop.

## Setting changes as cards

- An answer may suggest setting changes. The model writes them after its
  answer as fenced `hex-change` lines (key, value, why; or `options` and
  `suggest` for a choice); the app reads them (`assistant::changes::parse`),
  never shows them raw (the draft stops at the fence, and "Preparing a
  change…" stands in while the rest comes), and turns each line into a card
  under the answer, or refuses it. A choice card names the chosen value
  on its button, cut with "…" past 28 letters (the option above has it
  whole), and puts "Suggested" under a name too long to stand beside.
- A value in a card or a plan row is a chip (`rune_ui::assistant::value`,
  Assistant's own; no other pane draws it). One that fits a line is as wide
  as its words. A longer one (instructions, a long model's name, a long
  way a term is heard) wraps in a chip 220 pt wide, to three lines; past
  three a quiet "Show all" under them unfolds it in place and "Show less"
  folds it back (owner decision 2026-11-12), per value and for the
  session. A chip never gives up room: a from → to pair wider than the
  room beside the row's words stands one under the other, with ↓.
- The prompt lists only the keys a card may change, with their values,
  where they live in the window, and, with `assistant.send_settings` on,
  their value now (`assistant::changes::catalog`; a shortcut as the
  window's keycaps show it, ⌥⇧A, or none). A large model reads every
  such key; a local one reads the keys its articles name, the shortcuts of
  the Labs among them, and a core set. With the values sent, the prompt
  asks for an answer for this person, in order: the model looks up every
  feature its pages offer in the now column first, and one that is on and
  helps opens the answer, with their own shortcut or value; one that is
  off is offered as a card after it; what doesn't fit their setup goes
  last or not at all.
  Never offered: the update channel, API keys, sign-ins, folders and paths,
  microphones, app and site lists, calendar ids, Assistant's own settings.
- Pressing a card's button is the only way it changes anything. The save
  goes through `control::settings::check`, the path `hex settings set`
  takes, with what the key needs first (a Labs feature, Meetings) in the
  same save ("Also turns on…", "Change both"); a speech model or language
  goes through the service, which downloads it first and switches when
  done. A shorter retention says what goes (older meetings to the Trash,
  older dictations deleted) and its button is Danger.
- Only the newest card still waiting gets the Primary button. Undo puts
  back everything the save changed while the conversation stays open this
  session; another conversation, a quit, or a hand change ends it, and the
  card then says Applied with Show in Labs (or its pane).
- What a card shows follows the settings each frame: applied, declined,
  already set, changed by hand since, downloading, a macOS permission to
  ask, a Codex sign-in, or held by the organization or the OS.
- A change to the value the setting has now is never a card, compared as
  the engine it runs (`names::same`: `codex` equals `codex:<AI cleanup's
  model>`, empty equals none), and neither is a setting this computer
  can't have (another OS's Labs feature, one an organization holds, one
  that needs a notch this screen lacks): the gate drops both (below). A
  remote model gets one silent round (`prompt::rewrite`) to write its whole
  reply again, told in `<already_set>` that the value is set and in
  `<not_here>` that the setting isn't available, so its words claim no
  difference and offer no card there isn't; a built-in model's card just
  goes. A card's from side names the engine with its model (Codex
  (GPT-6.1 Terra)).
- What the model is told of this install is one typed value,
  `assistant::facts::Facts`, made in one place (`Facts::of`) from the
  settings and what `facts::gather` read off the computer when the
  question was sent or setup moved on (never per frame; a scene's own
  stand-ins in scenes). Every fact is the effective value, from the
  resolver the window and the runtime use, never a raw key: the speech
  models downloaded (`transcription::models::is_installed`, by the
  picker's names, the one in use marked), the recognition language, the
  engine dictation's cleanup, Files, and Assistant run on
  (`files::rewrite::Engine::resolve`), who writes meeting notes
  (`meeting::generator`'s choice: with `meetings.notes.writer` in use,
  `meetings.summaries` reads as the fallback, not used now; a writer
  saved while Labs › Clean up on your own server is off isn't the
  writer), Codex's sign-in, whether OpenCode runs, the Labs on, each
  shortcut set as its keycaps show it, the Vocabulary (below), and the
  hardware. A fact that couldn't be read is `Fact::Unknown`
  and reads "unknown", never "none" or "no". Whether an OpenRouter key is
  saved and which local servers are installed are for the app's own rules
  (setup) and are never written for a model. The window passes only what
  lives in its state (`facts::Seen`: the settings being typed, the setups
  kept, what Output and the pane last heard of Codex and OpenCode).
- The Vocabulary fact (`facts::Vocabulary`) says who reads how much of the
  list, in one line: how many terms it holds, how many of them the speech
  model in use takes while it transcribes (`models::vocabulary_terms_read`,
  the count behind the sheet's "This model reads the first N of M terms";
  for a Parakeet-family model only with Labs › Vocabulary biasing on), and
  whether Clean up with AI is on in the Global mode: on, cleanup reads the
  whole list with its sounds-like spellings (`processor::vocabulary_lines`
  has no cap), so no term is lost; off, the line says how many terms are
  used nowhere. `Vocabulary: 97 terms; while transcribing, Orukeet is
  biased toward only the first 64 (Labs › Vocabulary biasing is on); AI
  cleanup is on and reads all 97 …`. Told only "the first 64", a model
  answered that the other terms don't work.
- One formatter writes them (`facts::text::told`): the `<state>` block, a
  fact a line, only with `assistant.send_settings` on, and "Not on this
  Mac: …" either way. Engine ids and Codex efforts go in the catalog's now
  column as stored. `rune_ui::assistant::facts_tests` holds each fact that
  mirrors a row (models, notes writer, engines, shortcuts, the terms the
  speech model reads) to what the
  window's own code shows for the same settings.
- The hardware comes from `desktop::hardware`: `Screens: 1 external, no
  notch`, `Pointing devices: Magic Mouse, no extra buttons`, `Computer
  model: Mac16,11` (`hw.model`, raw: a `MacNN,N` id doesn't say MacBook or
  desktop). Screens come from `NSScreen` on the main thread (a notch is a
  top safe-area inset, built in by `CGDisplayIsBuiltin`); pointing devices
  from IOKit's HID registry without opening one (no Input Monitoring), the
  owner's name dropped ("Anna's Magic Mouse" is Magic Mouse). Apple's mice
  and trackpads have no extra buttons; another reporting more than two, or
  not saying, may.
- Each answer keeps what its model was told (`Turn::knew`, from
  `facts::text::knew`): the same lines, and what else went along by name
  only ("6 guide articles; your cleanup instructions; 2 of 94 Vocabulary
  entries; 3 excerpts of your recordings"), never the instructions,
  entries, or recordings themselves. It lives in memory for the session
  (never saved with the conversation), goes to the debug log, and shows
  under the answer's steps as one more fold, "What Assistant knew ›",
  plain selectable text. A guide answer of this session gets the fold
  line "Read the guide in 7 s ›" to hold it; an answer from an earlier
  session has no record.
- A Labs article may say what it needs (`needs: [notch]`,
  `[mouse_buttons]`): one known unmet keeps the feature out of setup's
  follow-ups (the app's and a model's), the plan (the rules' lines and a
  model's), and the answers' suggestions: the prompt says "Not on this
  Mac: Notch indicator (needs a screen with a notch)", and asked about it
  directly the model says what it needs. Unknown hardware leaves nothing
  out: setup asks the article's `ask_if:` ("If your mouse has extra
  buttons, …"), and the plain `ask:` once the need is known met.
- No card stands in for what the model doesn't know, or only lets the
  person look something up (Labs › Downloaded models only "to see the
  list"): where it can't tell, it names the place to look, no card.
- A line the app refuses (no such key, a value it doesn't take, a key
  never changed from a card) shows as a muted row, never an action. Codex,
  OpenCode, OpenRouter, and your own server get one silent round to fix
  their refused lines (terms included); a built-in model doesn't. The next
  question tells the model what became of each card.
- The instructions card (the Global mode's prompt) shows the change line by
  line, with Save instructions, Edit first, and Not now. Edit first opens
  the proposed text in the card's own text box (never Output's editor);
  Save instructions then saves the edited text through the same check, and
  Cancel edit goes back to the lines changed. Undo is as for any card.
- Two parts of the Global mode are keys of their own for cards:
  `dictation_processing.default_mode.prompt` (its instructions) and
  `dictation_processing.default_mode.cleanup` (Clean up with AI), each read
  from and written back into the mode (`changes::parts`).

## The gate: what is offered

Everything Assistant proposes passes one gate before the pane sees it
(`assistant::offer`): a setting card from an answer, every row of a plan
(the app's rules', a follow-up's Yes, a model's lines, the preset's), a
follow-up question, a term to add, the instructions card.
`control::settings::check` is the one way a change is applied; the gate is
the one way one is offered. `changes::resolve`, `setup::plan`,
`setup::follow`, and `setup::preset` hold no rules of their own: each asks
the gate, saying who proposes (`Origin`: an answer, a plan, the preset).

- One verdict a proposal (`Verdict`): show it, show it unchecked
  (`Held`: you chose the value there is, it shortens a retention, Codex is
  signed out, its engine isn't ready), or drop it with why (`Why`).
- A setting (`Gate::setting`), in order: the key exists and no card ever
  changes it; the preset asked for where it isn't offered makes no card;
  this VibeHex has the setting on this OS and no organization holds it;
  this computer can use it (a notch, a mouse's extra buttons); the value is
  one a save takes (the same `check`); it differs from the value now; in a
  plan, it wasn't declined in an earlier setup while the setting is as it
  was then; in an answer, it isn't a switch that only changes what a list
  shows (Labs › Downloaded models, the picker's sort and filter, Settings
  search, History stats, Benchmark details) for a question with no word
  that asks for a change (`offer::lookup`); in a plan or the preset it
  comes unchecked for the reasons above. A choice card's options pass the
  same rules, each on its own.
- A typed question is the person's own ask: a card it leads to is offered
  though a setup once declined it, and has no checkbox to leave unchecked
  (a shorter retention is its card's own warning and Danger button). The
  preset is asked for by name, so "You chose" doesn't hold its rows.
- A term (`Gate::term`): one `hex vocabulary add` takes, that Vocabulary
  lacks, that a setup didn't decline (a plan only), and that passes the
  term rule below unless it is the person's own (a follow-up's Yes, the
  preset, a term the question itself names).
- A follow-up question (`Gate::ask`): about a setting there is and a card
  may change, here, on this computer, not on once round one is saved, not
  answered or declined before unless the app picked it itself.
- Three reasons are a model's mistake and show as a refused row it is told
  (no such key, a value a save doesn't take, a key never changed from a
  card); every other drop is silent. All of them are kept with the turn,
  as the last lines of "What Assistant knew": `Proposed and not offered:`,
  then a line each, `labs.picker_downloaded = true: only a way to look
  something up`, `term Web: an ordinary word heard right, not a term`,
  `ask labs.notch_indicator: this computer can't use it`. The debug log
  gets the same lines without the terms, which may be words the person
  said.
- What a card shows once it is there (a value changed by hand since, an
  organization's lock set later) is not the gate's: that is the card's live
  state (`changes::state`).
- `offer::tests` holds each rule as a row of a table, by who proposes; the
  scenarios run through the worker, so through the same gate, and their
  report lists what it dropped (`dropped <scenario> step N: …`).

## Vocabulary terms as a card

- To add names a transcript gets wrong, the model writes a `hex-vocab`
  block, one JSON line per term:
  `{"term":"Kubernetes","sounds_like":["кубернетис","кубер"],"why":"…"}`
  (`{"vocabulary_add":[…]}`, the older shape, reads too). Each term is
  checked as `hex vocabulary add` checks it (`changes::vocab`); an empty or
  too long one is refused like any line.
- A term the vocabulary has, in any case, is dropped by the app and never
  named back to the model. So is an ordinary English word, or a Title Case
  phrase of ordinary words, offered on its capital alone (Web, New
  Session): it needs a spelling that shows it is misheard, the rule the
  use's terms pass (`recordings::terms::passes`). A term the question
  itself names is the person's ask and is offered.

## Why a word came out wrong

- `intent::cleanup::wants` reads from the words (every language the app
  speaks: промпт, инструкции, prompt, cleanup, клинап, очистка, «вместо»,
  «заменяет», «дописывает», «распозналось», словарь…) whether a question,
  or one of the two before it, is about what cleanup or recognition did to
  the person's words. With `assistant.send_settings` on it then reads
  (`assistant::personal`): `<cleanup_instructions>` (the Global mode's, each
  mode's with its replacements and own terms count, Files' own, each cut at
  4,000 characters) and `<vocabulary total="N">`, the entries whose term or
  "sounds like" the question and the last two turns name, at most 30. The
  cleanup article found brings the instructions too; a Vocabulary article,
  the entries. The prompt (`DIAGNOSE`) asks for the cause: an entry whose
  spelling catches the word, a replacement, a line of the instructions, or
  the speech model, with a card for the instructions when they are it.
- "Why does it write X instead of Y" (`intent::cleanup::written`: «пишет X
  вместо Y», «вместо Y пишет X», "X rather than Y", statt, au lieu de, a
  quoted form first) is also a question the dictations may answer, once
  the person has let them be read (`intent.written`, never sure, so it
  brings no consent card): the History search looks for X in what was
  written, the ones that wrote the whole form first.
- A dictation sent to the model reads "Heard: …" (the History entry's
  `raw_text`) and "Wrote: …" (`final_text`) when cleanup changed it, each
  cut to 450 characters around the words searched (what was heard at the
  same place as in what was written); otherwise its text, 900 at most.
  With such a dictation the prompt (`HEARD_WROTE`) asks the model to quote
  both and name the cause: wrong in Heard is the speech model or a
  Vocabulary spelling, changed only in Wrote came after transcription (a
  replacement, the instructions, a Vocabulary entry whose exact "sounds
  like" spelling was heard, which the app writes as its term in code before
  cleanup, or a term the cleanup model wrote because what was heard sounds
  like it: `dictation::vocabulary_guard` undoes one nothing heard sounds
  like), with a card when it is a setting it sees, and a line changing the
  entry when it is one. The Vocabulary entries the dictations name come too.
- An entry already in Vocabulary changes through a `hex-vocab` line naming
  it with what changes: `{"term":"AI cleanup","remove_sounds_like":["ай"]}`,
  `add_sounds_like`, `rename` (`changes::vocab_edit`). Only an entry the
  vocabulary has, and a spelling it has, make a card; anything else is
  refused for the repair round. The card ("General › Vocabulary, Change 2
  entries") shows each entry with a checkbox, its term (old → new when
  renamed), and its spellings: kept plain, removed struck, added on the
  accent tint. Change saves through `hex vocabulary edit`'s path
  (`control::edit_vocabulary`, `VocabularyEdit`), each entry rewritten where
  it stands; one changed since the card is left out with why. Undo writes
  each back exactly as it was, unless it changed since. Scene:
  `assistant-vocab-edit`.
- A prompt carrying Vocabulary entries never searches the web
  (`worker::offline`, like recordings), and the recordings search never
  reads more of it than before. A block that names none of the entries
  (an empty Vocabulary, a question naming no term) carries nothing of the
  person's and keeps nothing off the web.
- With `send_settings` off nothing of it goes, and the model says that
  **Assistant › Settings › Your current settings** lets it read them, with
  no card.
- The card, "General › Vocabulary, Add 3 terms", lists each term with
  "sounds like: …", a checkbox, and Edit, which renames the term and
  changes its spellings in the row (two fields and Done). "Add N terms"
  counts the checked ones, beside Not now.
- Add goes through `control::edit_vocabulary`, the code `hex vocabulary
  add` runs. A term the list has by then is left out ("Already in
  Vocabulary"), never merged into, so Undo takes out exactly the terms
  the card added, wherever they are in the list by then; it holds as long
  as a settings card's Undo. After: "Added to Vocabulary" (or "2 of 3
  terms added"), Show in Vocabulary (General with the sheet open), Undo.

## Guided setup

- "Помоги мне настроить VibeHex под меня", "Set VibeHex up for me", "What
  should I turn on?", "Which Labs features are worth turning on for me?",
  «Какие Labs мне стоит включить?», «Что посоветуешь настроить?»
  (`assistant::intent::setup`: a setting-up verb with only small words
  around it, or asking which ones to turn on or set up for them) start
  guided setup; "Как настроить горячую клавишу?" and "What does Labs › Live
  text turn on?" stay guide questions.
- It opens with "**Now:** …" (`setup::now`): the speech model and its
  language, dictation cleanup (off, or its engine and model), the meeting
  notes writer, the Files cleanup engine (or "as dictation"), up to four
  Labs features that are on, and the Vocabulary count.
- Round one, four questions with fixed ids (`assistant::setup`), every
  chip preselected from the current settings (`setup::preselect`; a
  question the person touched keeps their pick):
  - `use` (several: `dictation_any`, `dictation_code` (preselected when
    Russian vibe coding, the terminal paste shortcut, or Shift paste is
    on), `meetings`, `files`);
  - `language` (`ru`, `en`, `ru_tech` "Russian with English tech terms",
    `mixed`); code and Russian preselect `ru_tech`, with the hint "for
    code, tech terms usually stay in English";
  - `cleanup`, "you wait for it": `local` ("On this Mac", "On this
    computer" off macOS), `codex`, `opencode`, `openrouter`, `custom` ("My
    own server", named LM Studio by port 1234, Ollama by 11434, or by the
    app in /Applications: `setup::engines::own_server`), `none`, each with
    a caption;
  - `notes`, "may take longer", shown only when calls or files are picked:
    `codex`, `opencode`, `openrouter`, `custom`, `local`, and on a Mac
    `apple` (Apple Intelligence, meeting notes only).
  - The Start row draws them at once, in Assistant's language, with no
    model. A typed request asks the model (`SETUP_ASK`) to word them as
    `hex-ask` lines in the message's language; missing, broken, or with
    unknown ids, the app's own show.
- Follow-up rounds (`setup::follow`). Labs articles say whom setup asks
  (`for:` uses: `dictation`, `code`, `meetings`, `files`, `ru`,
  `cleanup`) and how (`ask:`, a yes/no question; `then:` what follows);
  a test keeps them sound. The app picks the candidates: fitting the
  answers, available on this OS and build, not on already (after what
  the plan's own lines turn on, so vibe coding's features aren't asked),
  best first, at most 12; code in Russian without vibe coding first asks
  to add tech terms (GitHub, Claude Code, pull request, TypeScript,
  Docker) to Vocabulary. Rounds of four, at most three; each round has
  "Skip to the plan", and an unanswered question changes nothing. A
  remote model (`setup_follow`) may reword, drop, or add questions; an
  added one names a Labs feature (the app's Yes / No) or a setting whose
  option values a save takes, else it is dropped. A built-in model, a
  scene, or a bad reply gets the app's own questions.
- The plan (`setup::plan`), the app's rules first, at most 24 rows:
  - speech model: one that already handles the language stays. Code in
    Russian or Russian with tech terms: Orukeet or Parakeet v3, else
    Parakeet v3, never GigaAM; plain Russian: GigaAM v3 or Orukeet stay,
    else GigaAM v3; mixed: any multilingual model, else Parakeet v3;
  - the engines answered: dictation's `runs_on` with Clean up with AI
    (or it off), `meetings.notes.writer` (Apple Intelligence through
    `meetings.summaries`), `files.cleanup_engine`, and the AI cleanup
    server lab where an engine needs it;
  - Russian vibe coding for Russian and code on any cleanup engine;
    Meetings and Files for those uses; each follow-up's Yes.
  - Rows from an answer start checked. A row over a value the person set
    (not the default) may be offered but starts unchecked with "You
    chose …". An engine not ready (no OpenRouter key, no server address,
    OpenCode not running, Codex signed out) starts unchecked with where
    to finish. What already fits folds into "Already set: N things ›".
  - A model (`SETUP_PLAN`) may add up to 6 rows of its own; it can't
    change a row the rules own, and its row over the person's own value
    starts unchecked. Its refusals show beside the plan.
- Apply checks each checked row on its own against the rows before it: a
  failing one gets why ("Not changed: …") and the rest save together; the
  speech model then downloads. Undo puts back the save, the model, and the
  terms added.

- Each finished setup (its plan applied or put off, or nothing to plan)
  is kept with the conversations (`setup::past`, `Store::setups`, at most
  20, only while conversations are kept): its date, answers, the rows
  applied, and what was declined (a row left unticked on an applied plan,
  a follow-up's No; "Not now" declines only the Nos). A declined line
  (key and value) isn't offered again, by the rules, a model, or a
  follow-up, while the setting is still what it was then
  (`facts::State::declined`, `past::held`).
- A setup is kept from its first answered round on (each Continue or
  Skip), so one left half-way, or whose plan is put off, keeps its
  answers; a later round or the plan's decision updates the same record
  (`Setup::same`: its conversation and when round one was asked), keeping
  when each follow-up was answered and the settings it saw then. Each
  follow-up answered, Yes or No, is kept as asked (`Setup::asked`).
- A round is answered only once what comes next can be asked for. While
  a model is answering in another conversation and the next round or the
  plan is a model's, Continue and "Skip to the plan" are dimmed and the
  round says "The model is answering in another conversation."
  (`Flow::waits`); pressed anyway, nothing is answered or kept, and
  Continue goes on once the model is free. A round the app writes itself
  never waits. A request the worker turned down all the same becomes a
  turn that says so, with Try again.
- The record follows the plan's card (`Flow::plan_decided`, sent on every
  decision: apply, Not now, Undo, Change anyway). Applied or put off, the
  plan is decided. A save that failed decides nothing: the setup stays at
  its plan, and the Try again that goes through is the one recorded. Undo
  takes what was applied out of the record; "Change anyway" after it, or
  after Not now, puts it in. An Undo that failed leaves it saying applied.
- A follow-up answered before isn't asked again while what its Yes
  changes is still as it was then and no release dated after the answer
  names its key (`past::answered_before`, `since::changed_since`; a
  development build's unreleased "next" doesn't count). The app leaves it
  out of its candidates, a model reads it under `<answered_before>` and
  may not ask it (`follow::settled`: answered or declined keys, dropped
  from what it writes), and the round shows it folded as "Answered
  before: 2 ›", unfolding to each question with its earlier answer,
  changeable there (`Ask::before`). Those answers count as the round's:
  a Skip keeps them, and with nothing new to ask they join round one's.
- Returning setup (`setup::since`, `rune_ui::assistant::returning`): with
  a setup kept, 50+ dictations in History, or any meeting, round one is
  folded to "Has anything changed in how you use VibeHex?" with the last
  answers preselected (else the settings'), Continue and "Change
  answers". The opening is short Markdown, in the window's language:
  - "**Now:** …", then the updates since in one line: "5 updates since
    your last setup (date): Assistant, Files, Listen to the recording, …
    Ask “What's new?” to hear more." (`since::releases_after`: the notes
    dated after the last setup's day or, without one, after the day the
    first version in `assistant/versions.json` ran ("since 3.3.0-vibe.4"),
    else every one, up to the version running; Next only on a development
    build). A few headlines, split from the releases' titles, newest
    first, no versions; "Nothing new in VibeHex since your last setup"
    when there are none.
  - Only while `assistant.read_recordings` is on, "What your use shows:"
    and at most five bullets (`recordings::usage`, computed by the app):
    dictations in the last 30 days, where most go with the terminal or
    editor share (30%+ adds code to `use`), long dictations (60+ words,
    20%+ names the Codex trade-off), what cleanup keeps fixing (never ё
    put back, все → всё, which is spelling and would crowd out кодекс →
    Codex; and never a "sounds like" spelling Vocabulary lists written as
    its term, which is the list at work in code before cleanup, not a
    thing it lacks), meetings and their language. Without leave it says the use could show more
    (its place, Assistant › Settings, in the window's words).
  - The opener, its step, the plan's term rows' captions, and "Answered
    before" are Assistant's own words: in its default language, with rows
    in every language (`setup::since::words`, `plan::words`, and the
    controls test in `rune_ui::assistant::feed`).
  - On a remote model (Codex, OpenCode, OpenRouter, your own server) with
    the use read, the model then looks (`Stage::Look`, `SETUP_LOOK`): the
    step "Read your settings, 972 dictations and 5 meetings" shows while
    it writes and folds to one grey line with its seconds, then 2 to 4
    concrete suggestions, each tied to a fact (what is new and fits, a
    term cleanup keeps fixing), and round one folded after them, even if
    the model fails. Try again looks again. It reads `<since_last_setup>`
    (the last setup's answers, applied, declined; releases; the use's
    aggregates), the settings now, and what was shown (`<shown>`), never a
    recording's text, and writes no change lines; it writes in the default
    language, as every prompt is told (`prompt::default_language`). A
    built-in model, no engine, or a scene: the app's facts only, round one
    at once.
- Terms Vocabulary lacks (`recordings::terms`) never show at the opening;
  they wait for the plan. A term is a Latin word said beside a Russian
  one, said twice or more, that isn't an ordinary English word (a list of
  common words, `recordings/english.txt`, with plurals and -ed/-ing
  forms), unless it is shaped like a name (`GitHub`, `API`, `gpt5`);
  never a small word ("to"). A word that is a Vocabulary term, one of a
  term's words ("Code" of "Claude Code"), or a sounds-like is known. A
  phrase cleanup writes twice or more for Russian it heard ("pull
  request") is one term, as a whole. How it was heard comes only from
  fixes into exactly it, word for word or phrase for phrase.
- An ordinary word written as a name (Web, Monkey), or a Title Case
  phrase of ordinary words (New Session), is no term on its capital alone
  (`terms::ordinary`): a Russian speech model writes an English word in
  Russian letters, and cleanup writing «веб» as Web is that word heard
  right, put into English. It is a term only when it is misheard
  (`recordings::heard`): one heard word fixed into exactly it 3+ times,
  written as a name every time, and a heard form that isn't the word
  itself (compared by consonants: «вольт» for Bolt, never «монки» for
  Monkey) fixed into it twice or more. A term a model names passes the
  same rule (`terms::passes`): an ordinary word needs a sounds-like that
  isn't the word itself. In the plan the terms are rows of their
  own: the sure ones (one heard word fixed into exactly it 3+ times)
  checked, "Cleanup keeps fixing how these are heard"; the rest unchecked,
  "You say these, and Vocabulary doesn't have them" (`plan::with_terms`).
  A sure term a model's plan lists too stays checked.
  A term left unticked on an applied plan, or a follow-up's No to terms,
  isn't offered again.
- A term that is a near miss of a name VibeHex knows (`plan::app_names`:
  speech models, engines, the guide's articles and Labs, panes and
  settings) or of a Vocabulary term isn't a new term (`terms::named`): the
  same first and last letters and consonant skeletons (the first letter,
  then the rest without vowels or repeats) one letter apart, or the same
  for a short word; a name that is ordinary English (Output) is never
  matched. A near miss of a known name comes as that name with how it was
  heard ("Outkit", heard «ауткит», is offered as Orukeet); one of a
  Vocabulary term is left out, since `hex vocabulary add` can't give a
  term another spelling.
- A plan's terms, the use's, a model's, a follow-up's or the preset's, are
  one row under Vocabulary (`changes::state::Terms`,
  `rune_ui::assistant::terms`; owner decision 2026-11-12, the preset
  brings 90): a checkbox for them all, "Add 95 Vocabulary terms" ("Add 90
  of 95 Vocabulary terms" once some are unchecked) with a chevron, and
  while folded the first four by name, "and 91 more". It starts unfolded
  with five terms or fewer (`Terms::FEW`), or when a checked term's save
  failed, and folded above that; a term being edited is never folded away.
  Unfolded, each term is a row of its own, drawn as the Vocabulary card
  draws it (checkbox, term, "sounds like: …", Edit), under its why as a
  small caption where the why changes; a long term wraps in its column
  (inside a word when one word is wider) and a long list of spellings
  wraps beside it. Edit renames the term and changes its spellings in the
  row; Apply keeps an edit still open and adds what the rows say through
  `hex vocabulary add`'s path.
- The terms are counted apart from the settings (`changes::counts`, owner
  decision 2026-11-12: "4 changes" hid the preset's terms): the header
  reads "3 changes and 81 terms, 3 and 76 selected", the button "Apply 3
  changes and 76 terms", the applied line "3 of 3 changes applied, 76 of
  76 terms added". A plan with no terms reads as before ("3 changes, 2
  selected", "Apply 2 changes", "2 of 2 changes applied"); one with only
  terms, or with every setting unticked, adds terms ("81 terms, 76
  selected", "Add 76 terms", "76 of 76 terms added"). One and many are
  separate rows in all nine languages. The save, a term's own failure and
  Undo stay term by term.

- Guided setup, first and returning, and the preset are one state machine
  (`assistant::setup::flow`). `Flow` is the setup in one conversation:
  where it stands (`Step`: looking at what came since, round one, a
  follow-up round, answered and waiting, stalled, the plan, the preset's
  plan, its shortcuts, done) and everything gathered, which is the
  conversation's own turns (what was looked at, asked, answered, the plan)
  and the setups kept (answered, declined, applied). `Flow::on` is the one
  transition: a state and an `Event` (begin, a chip picked, Continue or
  Skip, "Change answers", a card decided, ask again, look again, the
  preset and its two shortcut buttons, a model that couldn't be asked)
  give the next state and the `Effect`s to perform (a turn to add, a
  round's answers, the setup's record to keep, a model to ask at a
  `Stage`). It reads no file and asks no one; `Env` carries what it may
  know (the settings, the facts, the time, whether the app writes the
  rounds itself, whether a model is busy).
  - The pane sends events and performs effects in one place
    (`rune_ui::assistant::ask::send`, `on`, `perform`), and draws a round
    or the shortcut buttons only where `flow::step` says the setup stands.
  - The worker shapes a model's reply to a stage with `flow::replied`
    (one round of follow-ups with the rest waiting, a returning setup's
    look with round one folded after it).
  - A setup's record is made in one place (`Flow::keep`, the
    `Effect::Keep`) and merged into the kept ones by `past::keep`, which
    the store uses too; records saved before the machine load as they
    were (`an_older_saved_setup_still_loads`).
  - Its paths are tables of events, effects and steps
    (`setup/flow/tests.rs`: first and returning, on this device and with a
    model, a busy model, a save that failed, Undo and "Change anyway",
    the preset), and the pane's whole paths are pinned turn by turn
    (`rune_ui/assistant/flow_pins.rs`).

## Set me up like Denis

- "Set me up like Denis", "like the developer", «сделай настройки как у
  Дениса», «как у DenisKey» (`assistant::intent::preset`: a "like"/«как у»
  followed by Denis, DenisKey, the developer or the author), or the quiet
  "Or set it up like Denis, who makes VibeHex" under the Start row, answer
  with the owner's preset, no model asked (`setup::preset`,
  `rune_ui::assistant::preset`).
- Only when Russian is the recognition language (owner decision
  2026-11-12; `setup::preset::offered`): `transcription.language` is `ru`,
  whatever the speech model, one of many languages or GigaAM v3. A model
  that knows only Russian counts by its language too: the app saves it
  with Russian whenever it is chosen, so only a settings file edited by
  hand pairs it with Auto, and that gets no preset. With Auto, or any
  other language, the preset doesn't exist for the person: no row on the
  start page, `intent::route` sends the typed request to the guide as an
  ordinary question (never to setup), the pane's `preset::begin` starts
  nothing, and the worker drops every card and plan from that answer
  (`intent::preset_not_offered`), so no model can stand in for it. The
  guide's article says whom it is for, so the answer is honest.
- `presets/deniskey.json` is written by `scripts/export-preset` from the
  owner's settings.json and shipped with `include_str!` (last written
  2026-11-12, whole: 91 settings keys, 10 shortcut keys, the prompt, 90
  terms). It is an
  allow-list over the `hex settings` registry: every Labs switch, the
  speech model and language, dictation cleanup (on/off, `runs_on`, local
  and Codex model, Codex effort), Meetings' engines, writer, efforts and
  toggles, Files' engines, efforts and toggles, `meetings.retention`, the
  Global prompt, Vocabulary with spellings, and the shortcuts. The
  Vocabulary leaves without the terms that are the owner's alone
  (`PRIVATE_TERMS`: Monki, his own app; any case): 90 terms. They are
  also the list Labs › Russian vibe coding adds (`dictation::presets`
  reads this file), so the two can't differ, and the export writes them
  once more as text to paste (`docs/vocabulary/vibe-coding-ru.txt`). The
  term rule doesn't judge the preset's terms (`Origin::Preset`), so names
  that read as ordinary words ("New Session", "AskTool", "Cancel") are
  rows like the rest. Never API
  keys or other secrets, server addresses, folders, the microphone,
  calendars, window or update state, Assistant's own settings, app lists or
  browser hosts; `scripts/test-export-preset.py` (in `scripts/check`, exit
  15) feeds it a file full of them.
- Step one is one plan under setup's rules: a key this machine can't take
  is left out and named under "Not available here"; an engine not ready
  starts unchecked with where to finish; a value already the same folds
  into "Already set" (Labs off on both sides are left out); the prompt is
  a row, the terms one folded row (above) less those the person has. All
  checked but `meetings.retention`, which moves older meetings to the
  Trash.
- Once it is applied or put off: "These are Denis's shortcuts — do they
  suit you?" with the list, Use these (a plan of their own, with Undo),
  Keep mine, or a key typed instead (an ordinary question; the model's
  change card goes through `check`). A save that failed stays at the
  plan, and the Try again that goes through leads on to the shortcuts.
  Turns carry `preset` (`settings`, `shortcuts`, `shortcuts-plan`,
  `done`).

## What's new

- VibeHex's own release notes ship in the app: `docs/releases/<X.Y.Z-vibe.N>.md`
  and `next.md` (not released yet), embedded by `build.rs` (Hex's notes in
  that folder stay out). Front matter: `version`, `date`, `title` (one
  line of what it brings), `labs:` (the `hex labs` names a release added or
  changed) and `keys:` (other `hex settings` keys); the body is short New,
  Changed, and Fixed lines. Tests hold each file to its name, a title, real
  features and keys, two lines an item, no commit or PR numbers, under 1.5k
  tokens, and every `v*-vibe.*` tag to its notes.
- A release notes itself once per new version as the window's app starts
  (`releases::history`, `<support>/assistant/versions.json`: version and
  first run, the newest 20). A development build (`HEX_LOCAL_BUILD`, debug,
  or a version that isn't `X.Y.Z-vibe.N`) never does.
- `assistant::intent::releases` reads from the words alone whether a
  question asks what's new or changed ("Что нового в последнем релизе?",
  "What changed in the latest update?", "changelog", "release notes"), names
  a version ("что появилось в версии 3.3.0-vibe.5", "vibe.4"), or asks when
  something came ("When was Files added?"). It is checked before the
  recordings, and `intent::read` never takes such a question. One that names
  meetings, calls, or dictations without a version stays with them;
  updating and which version they run stay with the guide.
- `releases::select` picks the notes, newest first: every release since the
  version run before; the one running when that is unknown; the one named;
  every release for "when"; on a development build, Next and the latest
  release; the latest when the running version has none.
- The prompt adds a `<releases>` block saying which and why, and the rule:
  one sentence on what it covers (a development build says so), then New,
  Changed, Fixed in the question's language, then For you: every one of
  the things from those releases that fit the person's settings (the catalog's
  now column, with `assistant.send_settings`) and are off, each with a
  `hex-change` card, never one already on, and a word when a change touches
  something they use. The keys those releases name join the catalog a small
  model reads.
- After an update (two weeks from the first run of a new release, when an
  earlier one ran here) a new conversation shows "Updated to <version>"
  over Set VibeHex up for me (`rune_ui::assistant::whats_new`). A press
  asks "What's new since my last version?" in Assistant's language,
  answered from the notes whatever its words; once such a question is asked
  the row goes. There is no speech bubble on the face to put it in.

## Questions about recordings

- `assistant::intent` reads from the question's words alone, against a
  `now` passed in, whether it is about recordings: kinds named (meetings,
  calls, dictations, files, in English and Russian), a time (yesterday, last
  week, in November, on the 24th, the last five meetings), and what was said
  or decided. "How do I…", "можно ли…" without a time stay with the guide, so
  the consent card never comes up for them; so does saying how dictation
  should work for them ("мне важно удобство диктовки", "accuracy matters",
  "помоги настроить"), which speaks of dictating, not of what was dictated,
  unless it also asks to find or recall it ("найди мои диктовки, где…",
  "что я диктовал вчера"). A question that only may be
  about recordings searches them (once allowed) and goes to the guide when
  nothing matches.
- A follow-up that only refers back ("ответь на последний вопрос", «а там
  ещё что-то?», "tell me more", "and the other ones") carries the newest
  earlier question's intent, past other follow-ups (`intent::carried`): the
  search runs again for that question, and the prompt tells the model the
  excerpts answer it. One naming something new, asking how VibeHex works,
  or following a guide question stays with the guide. A follow-up that
  points at what was found and narrows it («а в какой из них было про
  поиск?», "which of them mention the release?"; `follow::narrows`)
  carries that intent too, with its own words added to the search.
- Who or what writes the notes («кто у меня пишет заметки встреч»,
  "which model writes my notes") asks how VibeHex is set, so it stays
  with the guide unless it names a time.
- Where a message goes is one function, `intent::route` (the preset
  where it is offered, setup, the release notes, the recordings, the guide), and what a
  recordings question may read another, `intent::reads`; the pane and the
  eval's scenarios both call them.
- While `assistant.read_recordings` is not `true` such a question shows the
  consent card first; nothing is read. Allow sets it and asks; Answer
  without recordings answers from the guide, telling the model it may not
  read them.
- `assistant::recordings` is the only module under `assistant` and
  `rune_ui::assistant` that reads personal data, through
  `recordings::Allowed`, which exists only while the setting is on (a test
  holds every other file to it). It reads meetings through `hex mcp`'s
  `meeting::query::Library` (notes, and the best passages by the meeting
  search), Files records, and the History snapshot, in the time asked about,
  within a budget: 3 meetings, 2 passages each, 5,000 characters for a
  built-in model; 6, 4, and 24,000 otherwise.
- Everything in the time asked about is searched; "the last few" (a plural
  «последние», "last few", five unless a number is given; «последнюю», one)
  are the newest of the matches, newest first, and the step counts all it
  searched. Without words to search for, the newest.
- A word counts however a transcript writes it (`recordings::spoken`): a
  Latin name apart at its capitals ("vibe hex") and as it sounds in
  Cyrillic, joined and apart, with the last consonant voiced or not
  (вайбхекс, вайп хекс), each also with a case ending (вайб хексе); and,
  from the person's Vocabulary, the term and its "sounds like" spellings,
  whichever of them was asked («кубер» finds Kubernetes, «вайб хекс» the
  term it spells). They reach the meeting search as vocabulary aliases
  (a several-word form as adjacent words). The Vocabulary is read only for
  the search and never goes to the model.
- The prompt carries them as a numbered `<recordings>` block. A transcript
  excerpt (a meeting passage, a Files stretch) has one numbered line per
  phrase, `[2.3]` being line 3 of excerpt 2; notes, whole texts, and
  dictations have none. The model cites the lines that say a fact, `[2.3]` or
  `[2.3-2.5]`, or an excerpt without lines by `[n]` (`recordings::cite`
  reads them; junk in brackets and Markdown links stay as written). The
  answer still shows one `[n]` per excerpt where citations stand together,
  and says plainly, in the question's language, when the recordings don't
  hold the answer. The streaming draft never shows a half written `[2.3-`.
- A cited line keeps its time and its words: it starts at its first timed
  word (or its segment's start in a transcript without word times) and ends
  with its segment, and its words are a byte range in the meeting's turns
  (`meeting::coalesced_ranges`) or the Files Original
  (`files::text::segment_ranges`). Unknown ids are dropped; a citation with
  no valid lines tints its passage only. A turn keeps its steps, seconds,
  and these places.
- A mark or chip opens its place (`jump`): a meeting on Transcript, the
  passage tinted and exactly its cited phrases bright over the rest of it
  muted, until the person leaves
  the pane; a Files text on its Original the same way, also while Listen to
  the recording plays (the playing word underlined over the tint); a
  dictation selected in History, its row tinted with the [n]. The lit
  phrase lands about a third of the way down the pane, whatever its height
  (measured from the panel), stopping at the text's start and end. No
  searched words are lit. Several places of one item step through the bar.
  A citation with no place of its own (an excerpt of a meeting's notes,
  `Ref::notes`, or an item cited whole) makes no other place current: notes
  open the meeting on Summary at its top, anything else opens the item at its
  top; the item's other places stay faintly lit, nothing lands, and the bar
  counts them without "1 of", its arrows starting at the first or the last.
  Back to Assistant and Cmd+[ (Ctrl+[) return to the conversation
  where it was scrolled; choosing another pane drops them. The mouse's back
  button doesn't yet (Rune gives other buttons only to elements that also
  take primary presses).
- A meeting's or a file's Requests session is the Assistant tied to that
  item: a question there that reaches past it (`intent::beyond`, read
  against the item's time, title, and text) runs the same search with the
  item left out (`Sources::without`), the same consent card, steps, marks,
  and chips; Back to Requests returns to that session (`jump::Origin`). See
  Meetings › Questions Past The Meeting.
- Recordings eval: `tests/fixtures/assistant/recordings` (ten meetings, three Files
  texts, fifteen dictations, ten naming VibeHex, half in Cyrillic), 28 questions with expected recordings (recall
  must stay at 90% or more, now 100%), 8 questions whose cited line must be
  offered and map to its time (now 8 of 8; word-timed, segment-only, and
  Files transcripts), 13 guide questions that must not route, and 4
  follow-ups that must read the earlier question's recordings again (the
  guide questions, followed up, must not; `scripts/check assistant::recordings`).

## Agent mode

On a remote engine (Codex, OpenRouter, your own server, OpenCode) a turn can
read step by step instead of being handed what the router guessed the
question needs (`assistant::agent`). A built-in model, a setup stage, the
"like Denis" preset, and Files' and Meetings' Requests always answer in one
request, as before; so does a remote engine agent mode is off for.

```ts
assistant.agent (text, no row; `hex settings set assistant.agent …`)
├── ""       the engines in agent::VERIFIED: those a live run of the
│            scenarios passed on (the table below)
├── off      none
├── all      every remote engine
└── a list   of codex, openrouter, custom (your own server), opencode
HEX_ASSISTANT_AGENT=1 | 0   overrides it for one run of the app or the eval
```

- **First request.** A guide question's: the guide's contents, the three
  articles that match best (the app's own pre-read, the first line of the
  trace), the catalog's core lines and `<state>`, the earlier turns, the
  question. Nothing of the person's own is in it, whatever it asks. So a
  question the guide answers still takes one request.
- **What the model is told** (`agent::prompt`). The base instructions with
  their three guide-only rules swapped (`SWAPS`: it answers from what it
  reads too, asks to read before saying the guide doesn't cover something,
  and what the person's recordings hold is a question about VibeHex), then
  `RULES`: reading works only by replying with the block (the tools are
  not functions of its own to look for), what to read for which kind of
  question, never to say something can't be read or what it holds before a
  result said so (`<state>` gives counts, never contents), and the tools.
  Under the question, one line (`ASK`): answer now if what is above
  answers it, else reply with only a block. The app's own reading of the
  question goes along as `<hint>` lines saying what to ask for, which the
  model may overrule: a question the router reads as about recordings
  (sure, or maybe), about release notes, a word written as another
  (`intent::cleanup::written`), or the cleanup instructions
  (`intent::cleanup::instructions`). A hint reads nothing. Under the
  results: how to read what was read, a line to cite the excerpts by
  number once there are any, and "answer now, or ask to read more". All
  of it came from the first live pass, where a model at low effort mostly
  answered without asking.
- **Asking to read** (`agent::protocol`). A reply that is one fenced
  `hex-read` block, a JSON object a line (`{"tool":"guide_read","ids":[…]}`),
  is a request, not an answer: the app runs each call, puts the results
  under the question as `<read step="N">` with a `<call>` and a
  `<result tool="…">` each, and asks again. Prose around a block is
  dropped and the block run. A line that isn't an object naming a tool, a
  tool nobody has, a call past the limits: each gets a result saying so,
  and the turn goes on. A result is data: it can't close its own block
  (a closing tag in it is taken apart). The draft shows nothing while a
  reply may still be a block.
- **Limits** (`agent::Limits`). 6 requests a turn, the last told to answer
  with a block in it cut out; 4 calls a block, 12 a turn; 9,000 characters
  a result and 60,000 a turn (16,000 on your own server, whose window may
  be short); a recordings search within `Budget::LARGE` (`SMALL` on your
  own server); 300 s of model time. A silent request is asked again once,
  twice a turn at most. Stop is looked for between calls and requests, and
  keeps the draft and the trace.
- **The answer** goes through `settle` as any other: cards, the gate, the
  repair round, `Sources:`, citations.

The tools (`agent::table`, run by `agent::tools::Desk`); none changes
anything, a change is still a card the person presses:

| Tool | Gives | Held by |
| --- | --- | --- |
| `guide_search`, `guide_read` | article ids and titles; articles whole with their settings' lines | nothing |
| `facts` | `<state>` again | `send_settings` |
| `settings_get` | catalog lines by key or prefix | only what `catalog::sendable` allows: never a key in `catalog::NEVER`, a secret, URL, path, text or JSON value, or a setting this computer lacks; with `send_settings` off, lines without the now column and the not-sent text |
| `cleanup_instructions` | every mode's instructions and replacements | `send_settings`; web off after |
| `vocabulary_lookup`, `vocabulary_list` | the entries some words name; the terms | `send_settings`; web off after an entry is read |
| `recordings_search`, `meetings_list`, `files_list` | numbered excerpts; lists with ids | consent; web off once anything is found |
| `meeting_notes`, `meeting_transcript`, `file_read`, `dictation_read` | one recording's notes or a stretch of its text, by id or by an excerpt's number | consent; web off after |
| `usage_insights` | counts, shares, apps: never a dictation's text | consent; web off after |
| `release_notes` | the notes since the version run before, one version's, or all | nothing |

- **Recordings** are read only in `assistant::recordings::read`, every
  function of which takes `&Allowed`; `assistant_code_names_no_personal_data`
  holds the rest of the code to that. Excerpts are numbered on one shelf for
  the turn (`recordings::Shelf`), so `[2.3]` means the same line in every
  result and in the answer.
- **Consent mid-answer.** A question the router is sure is about recordings
  still shows the card before the turn, as today. Otherwise a recordings
  tool called while `assistant.read_recordings` isn't `true` parks the turn
  (`agent::live`): the pane shows the same card under the steps so far, and
  the wait is not counted in the answer's seconds. Allow sets the setting
  and the turn reads on. Answer without recordings saves the no, the tool's
  result tells the model it may not read them, and that conversation isn't
  asked again until VibeHex quits (`agent::live::declined`); Stop asks
  nothing and decides nothing.
- **The web** (`agent::net`). One function decides before each request
  whether it may search (Codex with Web on): never once the turn has read
  anything of the person's own (an excerpt, a list that named a recording,
  what their use shows, their cleanup instructions, a Vocabulary entry),
  never when an earlier answer of the conversation did
  (`Turn::read_recordings`, which reads the trace too), never after a no on
  the card, and never, from the first request on, for a question the app
  itself reads as about their own (`agent::own`: the router would search
  their recordings for it, or its prompt would hold the Vocabulary entries
  the question names). So the agent's first request may search only where
  the router's could. A request that may not is the plain call, with the
  web's rules in the instructions replaced by "Web search is off for this
  answer". The turn keeps `Web {held_back}` only when the web could have
  helped: it was on earlier in the turn and what was read since turned it
  off, or only an earlier answer keeps it off; a question about their own
  never had the web to lose, and nothing is said, as with the router.
  Searches made before that stay with the turn. The eval holds every
  scenario, router's and agent's, to it (`checks::no_web_after_personal`).
- **The trace.** `Turn::trace` is what the turn did, in order
  (`agent::trace::Did`): articles by id (`app` for the pre-read), a guide
  search, settings by key, the instructions, Vocabulary entries as counts,
  a recordings search as its steps, notes, a transcript or a file by
  title, a dictation, usage, release notes by version, the card's answer,
  and a call refused with why. Names and counts, never text. A build that
  doesn't know a step reads it as `Unknown` and shows nothing for it, and
  an older build ignores the field, so `recordings::Step` gets no new
  variants.
- **In the pane** (`agent::words` says it, `rune_ui::assistant::recall`
  draws it). While it runs: each thing read with a check, then one line of
  what goes on: "Looking in the guide" only while the pre-read is all it
  has or a guide tool runs, "Searching your recordings", "Reading your
  settings" and the like while a tool runs, "Thinking it over" between
  reads. Finished: the fold counts only what tools read ("Read 3 meetings
  and your settings in 9 s"), so a plain guide answer reads "Read the
  guide in 5 s" with the articles laid out listed under it; a no on the
  card and a refused call have a small cross, not a check. `What Assistant
  knew` counts the articles and the excerpts the turn ended with.

Engines a live run passed on (`scripts/assistant-eval` with
`HEX_ASSISTANT_AGENT=1`, compared with `=0` on the same scenarios), which
is what puts one into `agent::VERIFIED`:

| Engine | Default | Live run |
| --- | --- | --- |
| Codex (`codex`) | agent | 13 November 2026, GPT-6-Luna at low effort, 37 scenarios: 36 pass, 333 of 334 checks, against 37 of 37 through the router the same night |
| OpenRouter (`openrouter`) | router | not run: no key on the machine the runs were made on |
| Your own server (`custom`) | router | not run: the models at hand were small local ones, not loaded |
| OpenCode (`opencode`) | router | not run: what a request costs depends on the provider behind it |

The one check Codex missed, `07-words-wrong` leading with the person's
shortcut, reads nothing and is as unsteady through the router at low
effort (asked alone three times each right after: the router missed it
three times, the agent once). The first agent pass that night held 28 of
37: the model answered without asking; what it is told now (above) came
of that, and a reply that was a call with its fence written wrong, shown
as the answer in the third pass, is read as a call since.

What a step costs, same run: an answer that read (20 of 44 turns) showed
its first step line after 9 s, its first word after 15 s, and ended after
19 s (medians), where the router, which reads before it asks, had the
first word after 5 s and ended after 9 s. An answer in one request had its
first word after 8 s against 6 s. The agent's requests carried 50 k
characters a turn against 39 k (medians). A model at higher effort asks
more readily; run the scenarios again before naming another engine here.

## Settings keys

| Key | Default | Meaning |
| --- | --- | --- |
| `assistant.enabled` | `true` | The sidebar row |
| `assistant.engine` | none | Where answers run, in `files.request_engine`'s form; none follows AI cleanup |
| `assistant.effort` | empty (medium) | Codex reasoning: low, medium, high |
| `assistant.agent` | empty | Which remote engines read step by step (Agent mode): empty for the verified ones, `off`, `all`, or a list of `codex`, `openrouter`, `custom`, `opencode`. No row in the window |
| `assistant.keep_conversations` | `true` | Keep conversations on this computer |
| `assistant.read_recordings` | none | May search meetings, Files texts, and dictations for questions about them; none until first asked |
| `assistant.send_settings` | `true` | The model reads the current switches, choices, numbers, shortcuts, and speech models with each question; never API keys, paths, or lists |
| `assistant.web` | `true` | Codex may search the web when the guide has no answer; a conversation's Web toggle overrides it |

## The web

```ts
Web on (assistant.web, or the field's Web toggle for one conversation)
├── Engine     Codex only, on a second app-server of its own
│              (codex::running_as(true)) started with web_search="live"
│              and features.code_mode_host=true (isolation::SEARCHING;
│              Codex 0.167 searches through code mode, whose host Codex
│              runs per process); every other isolation override, and
│              the person's MCP servers off, stay. Dictation cleanup,
│              Files, meeting notes, other answers, setup's questions and
│              plan: the first app-server, where both are off. Built in,
│              OpenRouter, your server, OpenCode: no toggle, no search
├── Prompt     web::RULES after the rest: the guide first, the web only
│              for what it doesn't cover (LM Studio, Ollama, OpenRouter),
│              official sources, nothing of the person's in a search, no
│              search for questions about their recordings unless asked,
│              each page cited as a Markdown link
├── Guard      worker::offline, in code, not the prompt: no web search
│              when the prompt holds the person's recordings (meetings,
│              dictations, Files texts). A recordings question, carried
│              follow-up, or declined one: the plain call, nothing said.
│              A later question in a conversation an earlier answer of
│              which read them (Turn::read_recordings): the plain call,
│              and Web {held_back} folds to "Read the guide in N s, not
│              the web". The settings snapshot may go along
├── Steps      codex.rs passes each `webSearch` item (item/started,
│              item/completed) to worker::seen → Answering.web → the
│              pane's live steps; the turn keeps Web {guide, searches,
│              pages, sources, held_back} (serde default, left out when
│              empty)
└── Sources    the http(s) links the answer cites, one per site, else the
               pages it opened; only when it searched. Links in the answer
               open in the browser too
```

Why Codex alone: OpenRouter's web plugin (`:online`) bills each search on the
person's OpenRouter credits and streams its citations in another shape, and
OpenCode answers through its own tools; neither is wired.

Code mode, checked 12 November 2026 on Codex 0.167: its `exec` tool runs
JavaScript "in a fresh V8 isolate … no Node, no file system, no network access,
no console" (Codex's own tool description), reaching only the thread's other
tools. With the isolation those are `web__run` (the search), `apply_patch`
(refused: read-only sandbox, approvals never given), the MCP resource tools (no
MCP server runs), and the clock; shell, exec, apps, plugins, browser and
computer use, and image reading (`features.view_image=false`, added for every
thread) are off. A thread's own `config` can't turn code mode on: with
`"features.code_mode_host": true` there, the model's `exec` still failed with
"code-mode host is disabled", hence the second process. Live check, 12 November
2026: `cargo test --bin hex -- --ignored --nocapture
searches_the_web_on_the_installed_codex` (one paid request) streamed a
`webSearch` item (`exec-…` id, query "site.lmstudio.ai/docs start local server
Developer Start server"), and the answer cited lmstudio.ai's server docs as a
link. `the_persons_mcp_servers_never_start` (free) checks both processes.

"From the web" takes only Markdown links and bare addresses in prose, never one
in code, and only public sites: `http://127.0.0.1:1234/v1`, `localhost`, IP
addresses and local names are a setting's value, not a source.

## Privacy

What reaches a model is the question, the conversation so far, and guide
text; never history, meetings, Files texts, or keys, except the passages
`recordings` found once the person allowed it, and, with
`assistant.send_settings`, settings' values and, for a question about what
cleanup did to a word, the cleanup instructions and the Vocabulary entries
it names. Keys never. Kept
conversations live in `<support>/assistant/conversations.json`, readable only
by this account, the 50 most recently used; with keeping off they last until
VibeHex quits. Clear conversations deletes them all.

Checks: `scripts/check assistant` (store, worker, retrieve, prompt, intent,
facts and the window's rows they mirror, the scenarios offline,
releases and their notes,
recordings and its eval, jump, settings keys, changes and their cards,
setup, vocabulary terms, and the reply corpus
`tests/fixtures/assistant/replies`: replies strong and weak models write,
each with the parse it must make); scenes `assistant`,
`assistant-answering`, `assistant-start`, `assistant-start-russian`, `assistant-connect`,
`assistant-settings`, `assistant-consent`, `assistant-steps`, `assistant-agent-consent`,
`assistant-recordings`, `assistant-steps-open`, `assistant-knew`, `meetings-cited`,
`meetings-cited-notes`, `meetings-cited-long`, `files-cited`, `files-cited-listening`, `history-cited`,
`assistant-places`, `assistant-place-opened`, the cards'
`assistant-cards`, `assistant-cards-more`, `assistant-cards-held`,
`assistant-cards-prompt`, `assistant-cards-prompt-edit`,
`assistant-applied`, `assistant-vocab-card`, `assistant-vocab-applied`,
`assistant-change-both`, `assistant-change-coming`, `assistant-whats-new`,
`assistant-updated`, setup's
`assistant-setup-ask`, `assistant-setup-follow`, `assistant-setup-plan`,
`assistant-setup-kept`, `assistant-setup-result`, `assistant-setup-preset`,
`assistant-setup-preset-shortcuts`, the terms' `assistant-plan-terms`,
`assistant-plan-terms-open`, `assistant-setup-preset-terms`
(`rune_ui::tests::assistant_terms`, Linux, presses the fold and both
checkboxes and holds a long term and a long list inside their row),
the web's `assistant-web`, `assistant-web-off`, `assistant-web-held`,
`assistant-web-answering`, and their goldens; `scripts/check control::settings`
for `check` and what a key needs;
`rune_ui::tests::assistant_jump` (Linux) drives a jump and Back by click and
Cmd+[, and the landing at two window heights.

## Quality loop

Every problem reported about Assistant becomes a scenario in the change
that fixes it: the fix isn't done while no scenario fails without it. A
problem in what the pane draws (a step line standing without its dots) is
held the same way by a pane test or a scene: the scenarios run the core
path, from the route to the parsed turn.

- **Scenarios** (`src/assistant/eval`, fixtures in
  `tests/fixtures/assistant`): each is a made-up install (settings, facts,
  hardware, and synthetic recordings: Russian speech with English terms,
  never anybody's data), a typed message or a setup step, and assertions
  on what came of it: the route, what the model was sent, and the parsed
  turn (cards, asks, plan rows, steps, sources, language), never the
  wording. Three checks hold for every step of every scenario: no error,
  no card that changes a value to itself, nothing this computer can't use
  offered. A step goes the way the pane sends it (`intent::route`,
  `intent::reads`, the same `Question`, the worker with its parse, gates,
  and repair round). A setup step is what the setup machine
  (`setup::flow`) asks a model when sent the events a person's presses
  send (`eval/stages.rs`: begin, round one answered, the follow-ups
  skipped), so a returning setup's follow-ups and plan carry what came
  since and what was answered before, and a first setup's plan carries no
  terms from use, as in the app. The catalog a prompt carries is this OS's, so played
  replies name features every OS has, and a scenario about a Mac-only
  feature (`scenarios::MAC_ONLY`) runs on a Mac alone. The worker is one
  per process: the scenarios and its tests share one lock
  (`eval::SERIAL`) for runs that keep them in one (`cargo test`).
- **Offline**, in `scripts/check` (`scripts/check assistant::eval` alone):
  the scenarios with a reply a model is played as writing, often a wrong
  one (a card for a value already set, a question about a notch on a Mac
  mini). It holds the routing, the prompt, the gates, the term heuristics
  on the mixed-language fixture, and the returning setup's opener. Run it
  for every change under `src/assistant` or `rune_ui/assistant`.
- **Live**, `scripts/assistant-eval`, opt in and never in CI: the same
  scenarios on a real engine (`HEX_ASSISTANT_EVAL_ENGINE`: `codex`,
  `codex:<model>`, `local:<id>`, `openrouter`, `custom`, `opencode`; Codex
  at low effort unless `HEX_ASSISTANT_EVAL_EFFORT` says otherwise). It
  prints a table (pass or fail per step, the cards, time to the first
  event and in all, the prompt's size), writes a JSON report
  (`HEX_ASSISTANT_EVAL_OUT`), and exits non-zero on a failed check;
  `HEX_ASSISTANT_EVAL_DUMP` names a folder for what each step sent the
  model, to read when one fails (a file a request: `10.txt`, `10-2.txt`).
  `HEX_ASSISTANT_AGENT=1` runs the remote engine as an agent and `=0`
  through the router, whatever `assistant.agent` says; the table then
  holds each step's requests, the time to its first step line (`read`),
  and the tools it called, and the JSON its trace. Scenarios 21 to 31 are
  the agent's (`eval/agent.rs`): offline they are played as an agent
  (`the_agent_scenarios_hold_offline`), wrong replies included (a tool
  nobody has, endless reads, prose around a block, a call before consent);
  live they hold the trace and the answer on either path. A
  remote engine is paid per request: run it whole after a change to the
  prompt, the catalog, the facts, or a gate, and before a release; after a
  fix, only what failed (`HEX_ASSISTANT_EVAL_ONLY=08,11-returning`).
  `HEX_ASSISTANT_EVAL_SUITE=questions` asks the guide questions of
  `tests/fixtures/assistant/questions.jsonl` instead, then setup and a
  vocabulary request, and reports the cards each made.
- **Replay**, `scripts/assistant-replay`, local only: the app's own
  heuristics over the real data on this machine, printed: the facts
  Assistant would be told, the setups kept and what isn't asked again,
  what the use shows, the terms setup would offer with how each was
  heard, the returning opener, the follow-ups, and the app's plan. No
  model, nothing sent, nothing written; recordings only when Assistant may
  read them. Run it when a heuristic changes (terms, usage, the opener,
  what is asked again): fixtures can't show what real use does to it.
- A check that fails on a real engine is fixed where it is ours (a prompt
  rule, a gate, the routing) and never by weakening it; a check that was
  wrong is changed with the reason written beside it.

## Surfaces, scenes, and tests

Fixtures are short and mostly English, and layout breaks on what they
don't hold: a long question, a model with a long name, a German word no
line can break, 95 terms. Two things hold the pane's layout on every OS,
with no golden image:

- **The layout check** (`rune_ui/assistant/layout/check.rs`) reads a laid
  out window's element tree and the glyphs it would draw, and fails when a
  box leaves its parent's or a glyph leaves every text's box, when a text
  is cut by a box above it or by its own "…" to less than half of itself,
  and when two siblings that show something overlap. A scroll container is
  excepted by its key, and so is a text with a control that unfolds it
  beside it ("Show all", `Rules::unfolds`); what the pane hangs outside or
  cuts short on purpose is named with its reason in `layout::RULES`, never
  skipped silently.
- **The matrix** (`rune_ui/assistant/layout/matrix.rs`) puts every surface
  below through that check in English, Russian, and German, with its
  fixture as it is and with long content (`long_fixture`: long questions,
  hints, reasons and option labels, a 49-letter model name, 95 terms, eight
  ways a term is heard), at the window's default width and its narrowest
  (860). Every other Assistant scene goes through the check as its fixture
  is. Three long cells are scenes with golden images.

A new surface gets a row in `matrix::SURFACES`, a test, and a line here; a
new scene gets a line here. `layout::inventory` fails when a scene or a
surface in code is missing from this list, or the list names one that is
gone.

| Surface | Scenes | Test in `layout::matrix` |
| --- | --- | --- |
| Start page | `assistant-start` | `start_page` |
| Composer | `assistant` | `composer` |
| Answer with steps and what it knew | `assistant-steps-open`, `assistant-knew` | `answer_with_steps_and_what_it_knew` |
| Setting card | `assistant-cards` | `setting_card` |
| Apply N changes | `assistant-cards-more`, `assistant-change-both` | `apply_n_changes` |
| Plan with its terms folded | `assistant-plan-terms` | `plan_with_its_terms_folded` |
| Plan with its terms unfolded | `assistant-plan-terms-open` | `plan_with_its_terms_unfolded` |
| Plan with long values | `assistant-long-values`, `assistant-long-values-open` | `plan_with_long_values` |
| Vocabulary edit card | `assistant-vocab-edit` | `vocabulary_edit_card` |
| Follow-up round | `assistant-setup-follow`, `assistant-setup-follow-ru` | `follow_up_round` |
| Returning opener | `assistant-setup-returning` | `returning_opener` |
| Consent card | `assistant-consent` | `consent_card` |
| Consent card mid-answer | `assistant-agent-consent` | `consent_card_mid_answer` |
| No model page | `assistant-connect` | `no_model_page` |

The other scenes, each through `every_scene_is_laid_out_whole`:

| Surface | Scenes |
| --- | --- |
| Start page in Russian | `assistant-start-russian` |
| Answer being written | `assistant-answering`, `assistant-web-answering` |
| Answer from recordings | `assistant-steps`, `assistant-recordings` |
| Answer from the web | `assistant-web`, `assistant-web-off`, `assistant-web-held` |
| Places named and opened | `assistant-places`, `assistant-place-opened` |
| Assistant › Settings | `assistant-settings` |
| Card held, applied, coming | `assistant-cards-held`, `assistant-applied`, `assistant-change-coming` |
| Instructions card | `assistant-cards-prompt`, `assistant-cards-prompt-edit` |
| Vocabulary card | `assistant-vocab-card`, `assistant-vocab-applied` |
| What's new | `assistant-whats-new`, `assistant-updated` |
| Setup's first round, plan, and result | `assistant-setup-ask`, `assistant-setup-plan`, `assistant-setup-kept`, `assistant-setup-result` |
| Returning setup's plan | `assistant-setup-returning-plan` |
| Set me up like Denis | `assistant-setup-preset`, `assistant-setup-preset-shortcuts`, `assistant-setup-preset-terms` |
| Long content, with golden images | `assistant-long-follow-german`, `assistant-long-plan-russian`, `assistant-long-choice-russian` |
