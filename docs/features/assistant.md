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
├── Header      Chat | Settings; in Chat, the conversation picker and +
├── Chat
│   ├── New     "Ask anything about VibeHex", "Set VibeHex up for me" (an
│   │           accent-tinted row: a sliders tile, a line on what it does,
│   │           a chevron), four example questions, and what the guide
│   │           holds (Labs features and topics)
│   ├── Feed    the person's bubble; the answer as Markdown beside the face,
│   │           then "From the guide" chips naming the articles it drew on,
│   │           then Copy, Ask again, the model, Stopped, the time.
│   │           A failed answer says why and offers Try again.
│   │           From recordings: "Read 4 meetings and 1 file in 9 s ›" over
│   │           the answer (a click lists the steps), [n] marks inline, and
│   │           "From your recordings" chips (title, date, time).
│   │           A place in the window named in bold (General › Vocabulary,
│   │           Labs › Smart spacing, Output) reads in the link colour and
│   │           opens it; other bold words are full-brightness text
│   ├── Consent the first question about recordings, before anything is
│   │           read: "Read your meetings, files, and dictations?", the model
│   │           the passages go to, Allow or Answer without recordings
│   ├── Steps   while it searches: one line per step, a check when done,
│   │           three dots on the current one, the face reading
│   ├── Setup   setup's questions one at a time beside the face: the
│   │           question, its hint, option chips (a chosen one tinted with
│   │           a check); Continue under one that takes several. Answered
│   │           ones stay as small chosen chips. Then "Suggested changes,
│   │           6 changes, 5 selected": a row per change (checkbox, name,
│   │           where, why, from → to, what it takes), Not now, and
│   │           "Apply 5 changes"; after, a mark per row and "4 of 5
│   │           changes applied" with Undo
│   ├── Coming  "Looking in the guide…", then the draft as it streams;
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
                        switches, choices, numbers, and speech models);
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
  under the answer, or refuses it.
- The prompt lists only the keys a card may change, with their values,
  where they live in the window, and, with `assistant.send_settings` on,
  their value now (`assistant::changes::catalog`). A large model reads every
  such key; a local one reads the keys its articles name and a core set.
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

## Vocabulary terms as a card

- To add names a transcript gets wrong, the model writes a `hex-vocab`
  block, one JSON line per term:
  `{"term":"Kubernetes","sounds_like":["кубернетис","кубер"],"why":"…"}`
  (`{"vocabulary_add":[…]}`, the older shape, reads too). Each term is
  checked as `hex vocabulary add` checks it (`changes::vocab`); an empty or
  too long one is refused like any line.
- The model never sees the vocabulary (`assistant.send_settings` leaves it
  out), so a term the vocabulary has, in any case, is dropped by the app
  and never named back to the model.
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
  should I turn on?" (`assistant::intent::setup`: a setting-up verb with
  only small words around it, or asking what to turn on) start guided
  setup; "Как настроить горячую клавишу?" stays a guide question.
- Three questions, with fixed ids (`assistant::setup`): `use` (several:
  `dictation_any`, `dictation_code`, `meetings`, `files`), `language`
  (`ru`, `en`, `mixed`), and `cloud` (`chatgpt`, `this_mac` ("Only on this
  computer" off macOS), `no_cleanup`).
  - The Start row draws them at once, in the window's language, with no model.
  - A typed request asks the model (`SETUP_ASK`) to write them as `hex-ask`
    lines in the message's language. Missing, broken, or with unknown ids,
    the app's own questions show under its words.
- A single choice answers on a click; several wait for Continue. The
  answers become one turn with `answers` (shown as chips, never a bubble),
  and the model is asked (`SETUP_PLAN`) for at most 8 `hex-plan` lines.
- Without a usable plan, the rules make one (`setup::baseline`), each
  checked against real keys, models, and this OS:
  - a speech model for the language when the one in use doesn't suit it
    (GigaAM v3 for Russian, Parakeet v3 for mixed);
  - Russian vibe coding for dictating code in Russian or mixed, with
    ChatGPT allowed;
  - with ChatGPT and dictation: cleanup on Codex, and Clean up with AI on;
  - No cleanup: Clean up with AI off;
  - Meetings on, and with ChatGPT its notes on Codex;
  - Files;
  - for dictation: Smart spacing, and Quick add to Vocabulary (unchecked);
  - code in Russian or mixed without vibe coding: one unchecked Vocabulary
    row of tech terms (GitHub, Claude Code, pull request, TypeScript,
    Docker) less those the vocabulary has.
  Rows already set are left out.
- The plan is one card (`CardKind::Plan`). A row the OS or an organization
  holds is left out; a shorter retention and a Codex row while signed out
  start unchecked (the latter with Sign in with ChatGPT in the row).
  Downloads show their size and the OS permission a row needs is named in
  the footer before anything is applied.
- Apply checks each checked row on its own against the rows before it: a
  failing one gets why ("Not changed: …") and the rest save together; the
  speech model then downloads. Undo puts back the save, the model, and the
  terms added.

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
- Recordings eval: `tests/fixtures/assistant/recordings` (ten meetings, three Files
  texts, five dictations), 28 questions with expected recordings (recall
  must stay at 90% or more, now 100%), 8 questions whose cited line must be
  offered and map to its time (now 8 of 8; word-timed, segment-only, and
  Files transcripts), and 13 guide questions that must not route
  (`scripts/check assistant::recordings`).

## Settings keys

| Key | Default | Meaning |
| --- | --- | --- |
| `assistant.enabled` | `true` | The sidebar row |
| `assistant.engine` | none | Where answers run, in `files.request_engine`'s form; none follows AI cleanup |
| `assistant.effort` | empty (medium) | Codex reasoning: low, medium, high |
| `assistant.keep_conversations` | `true` | Keep conversations on this computer |
| `assistant.read_recordings` | none | May search meetings, Files texts, and dictations for questions about them; none until first asked |
| `assistant.send_settings` | `true` | The model reads the current switches, choices, numbers, and speech models with each question; never API keys, paths, or lists |

## Privacy

What reaches a model is the question, the conversation so far, and guide
text; never history, meetings, Files texts, or keys, except the passages
`recordings` found once the person allowed it. Keys never. Kept
conversations live in `<support>/assistant/conversations.json`, readable only
by this account, the 50 most recently used; with keeping off they last until
VibeHex quits. Clear conversations deletes them all.

Checks: `scripts/check assistant` (store, worker, retrieve, prompt, intent,
recordings and its eval, jump, settings keys, changes and their cards,
setup, vocabulary terms, and the reply corpus
`tests/fixtures/assistant/replies`: replies strong and weak models write,
each with the parse it must make); scenes `assistant`,
`assistant-answering`, `assistant-start`, `assistant-connect`,
`assistant-settings`, `assistant-consent`, `assistant-steps`,
`assistant-recordings`, `assistant-steps-open`, `meetings-cited`,
`meetings-cited-notes`, `meetings-cited-long`, `files-cited`, `files-cited-listening`, `history-cited`, the cards'
`assistant-cards`, `assistant-cards-more`, `assistant-cards-held`,
`assistant-cards-prompt`, `assistant-cards-prompt-edit`,
`assistant-applied`, `assistant-vocab-card`, `assistant-vocab-applied`,
`assistant-change-both`, `assistant-change-coming`, setup's
`assistant-setup-ask`, `assistant-setup-plan`, `assistant-setup-result`,
and their goldens; `scripts/check control::settings`
for `check` and what a key needs;
`rune_ui::tests::assistant_jump` (Linux) drives a jump and Back by click and
Cmd+[, and the landing at two window heights.

The live eval, `scripts/assistant-eval`, is opt in and never in CI: with
`HEX_ASSISTANT_EVAL_ENGINE` set (`codex`, `codex:<model>`, `local:<id>`,
`openrouter`, `custom`, `opencode`) it asks that engine the guide questions
of `tests/fixtures/assistant/questions.jsonl`, then setup's questions, a
plan for fixed answers, and a vocabulary request, through `worker::ask`
with the repair round, and reports what each reply made: its cards, what
was refused, and the expected changes it missed. A remote engine is paid
per request. Not yet verified end to end by asking a real model from a
running app.
