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
│   │           a chevron), "Or set it up like Denis, who makes VibeHex ›"
│   │           (quiet text under it), four example questions, and what
│   │           the guide holds (Labs features and topics)
│   ├── Feed    the person's bubble; the answer as Markdown beside the face,
│   │           then "From the guide" chips naming the articles it drew on,
│   │           then Copy, Ask again, the model, Stopped, the time.
│   │           A failed answer says why and offers Try again.
│   │           From recordings: "Read 4 meetings and 1 file in 9 s ›" over
│   │           the answer (a click lists the steps), [n] marks inline, and
│   │           "From your recordings" chips (title, date, time)
│   ├── Consent the first question about recordings, before anything is
│   │           read: "Read your meetings, files, and dictations?", the model
│   │           the passages go to, Allow or Answer without recordings
│   ├── Steps   while it searches: one line per step, a check when done,
│   │           three dots on the current one, the face reading
│   ├── Setup   "Now: …" (what is set: speech model and language,
│   │           dictation cleanup, meeting notes, Files cleanup, Labs on,
│   │           Vocabulary terms), then round one as one form: four
│   │           questions, captioned chips in three columns, preselected
│   │           from the settings, one Continue. Follow-up rounds (Yes /
│   │           No, at most four a round) with a quiet "Skip to the plan".
│   │           Answered rounds stay as small chosen chips. Then
│   │           "Suggested changes, 6 changes, 5 selected": rows grouped
│   │           by area (Dictation, Code, Meetings, Files, Models), each a
│   │           checkbox (accent fill and a check, or an empty box with a
│   │           grey border), name, where, why, from → to, "You chose …"
│   │           over the person's own value, a warning where setup isn't
│   │           finished, "Then: …"; a folded "Already set: 6 things ›";
│   │           Not now and "Apply 5 changes"; after, a mark per row and
│   │           "4 of 5 changes applied" with Undo
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
                        switches, choices, numbers, shortcuts, and speech models);
                        Your meetings, files, and dictations (toggle)
Opened from an answer                            // rune_ui::assistant::jump
├── Header      Back to Assistant (chevron, face, words) before the title
├── Bar         an item cited more than once: "From the answer: <question>",
│               "1 of 2", up and down between places, a cross to clear
└── Place       tinted in the accent, round corners, the answer's [n] by the
                speaker, the cited phrases lit inside; others cited fainter;
                a third of the way down the pane. A dictation: its History
                row tinted with the [n] before its title
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

## Setting changes as cards

- An answer may suggest setting changes. The model writes them after its
  answer as fenced `hex-change` lines (key, value, why; or `options` and
  `suggest` for a choice); the app reads them (`assistant::changes::parse`),
  never shows them raw (the draft stops at the fence, and "Preparing a
  change…" stands in while the rest comes), and turns each line into a card
  under the answer, or refuses it.
- The prompt lists only the keys a card may change, with their values,
  where they live in the window, and, with `assistant.send_settings` on,
  their value now (`assistant::changes::catalog`; a shortcut as the
  window's keycaps show it, ⌥⇧A, or none). A large model reads every
  such key; a local one reads the keys its articles name, the shortcuts of
  the Labs among them, and a core set. With the values sent, the prompt
  asks for an answer for this person: a feature that does what they ask
  and is on comes first, with their own shortcut or value; one that is off
  is offered as a card; what doesn't fit their setup goes last or not at all.
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
  - The Start row draws them at once, in the window's language, with no
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
  (`catalog::State::declined`, `past::held`).
- Returning setup (`setup::since`, `rune_ui::assistant::returning`): with
  a setup kept, 50+ dictations in History, or any meeting, round one is
  folded to "Has anything changed in how you use VibeHex?" with the last
  answers preselected (else the settings'), Continue and "Change
  answers". Above it, after "Now:", "**Since your last setup (date):**"
  (or "What your use shows:"): releases since
  (`since::releases_after`, a stub until the releases notes land), and,
  only while `assistant.read_recordings` is on, the use
  (`recordings::usage`, computed by the app: dictations kept and in 30
  days, top apps, terminal or editor share (30%+ adds code to `use`),
  long dictations (60+ words, 20%+ names the Codex trade-off), what
  cleanup keeps fixing, Latin terms said in Russian or shaped like terms
  and missing from Vocabulary (as one "Add N terms" card, each with how
  it was heard), meetings and their language); without leave it says
  the use could show more. The model reads the same as aggregates in
  `<since_last_setup>` (last setup's answers, applied, declined;
  releases; use), never a recording's text.

## Set me up like Denis

- "Set me up like Denis", "like the developer", «сделай настройки как у
  Дениса», «как у DenisKey» (`assistant::intent::preset`: a "like"/«как у»
  followed by Denis, DenisKey, the developer or the author), or the quiet
  "Or set it up like Denis, who makes VibeHex" under the Start row, answer
  with the owner's preset, no model asked (`setup::preset`,
  `rune_ui::assistant::preset`).
- `presets/deniskey.json` is written by `scripts/export-preset` from the
  owner's settings.json and shipped with `include_str!`. It is an
  allow-list over the `hex settings` registry: every Labs switch, the
  speech model and language, dictation cleanup (on/off, `runs_on`, local
  and Codex model, Codex effort), Meetings' engines, writer, efforts and
  toggles, Files' engines, efforts and toggles, `meetings.retention`, the
  Global prompt, Vocabulary with spellings, and the shortcuts. Never API
  keys or other secrets, server addresses, folders, the microphone,
  calendars, window or update state, Assistant's own settings, app lists or
  browser hosts; `scripts/test-export-preset.py` (in `scripts/check`, exit
  15) feeds it a file full of them.
- Step one is one plan under setup's rules: a key this machine can't take
  is left out and named under "Not available here"; an engine not ready
  starts unchecked with where to finish; a value already the same folds
  into "Already set" (Labs off on both sides are left out); the prompt is
  a row, the terms one "Add N terms" row less those the person has. All
  checked but `meetings.retention`, which moves older meetings to the
  Trash.
- Once it is applied or put off: "These are Denis's shortcuts — do they
  suit you?" with the list, Use these (a plan of their own, with Undo),
  Keep mine, or a key typed instead (an ordinary question; the model's
  change card goes through `check`). Turns carry `preset` (`settings`,
  `shortcuts`, `shortcuts-plan`, `done`).

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
  or following a guide question stays with the guide.
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
  passage tinted and exactly its cited phrases lit, until the person leaves
  the pane; a Files text on its Original the same way, also while Listen to
  the recording plays (the playing word underlined over the tint); a
  dictation selected in History, its row tinted with the [n]. The lit
  phrase lands about a third of the way down the pane, whatever its height
  (measured from the panel), stopping at the text's start and end. No
  searched words are lit. Several places of one item step through the bar.
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

## Settings keys

| Key | Default | Meaning |
| --- | --- | --- |
| `assistant.enabled` | `true` | The sidebar row |
| `assistant.engine` | none | Where answers run, in `files.request_engine`'s form; none follows AI cleanup |
| `assistant.effort` | empty (medium) | Codex reasoning: low, medium, high |
| `assistant.keep_conversations` | `true` | Keep conversations on this computer |
| `assistant.read_recordings` | none | May search meetings, Files texts, and dictations for questions about them; none until first asked |
| `assistant.send_settings` | `true` | The model reads the current switches, choices, numbers, shortcuts, and speech models with each question; never API keys, paths, or lists |

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
`files-cited`, `files-cited-listening`, `history-cited`, the cards'
`assistant-cards`, `assistant-cards-more`, `assistant-cards-held`,
`assistant-cards-prompt`, `assistant-cards-prompt-edit`,
`assistant-applied`, `assistant-vocab-card`, `assistant-vocab-applied`,
`assistant-change-both`, `assistant-change-coming`, setup's
`assistant-setup-ask`, `assistant-setup-follow`, `assistant-setup-plan`,
`assistant-setup-kept`, `assistant-setup-result`, `assistant-setup-preset`,
`assistant-setup-preset-shortcuts`,
and their goldens; `scripts/check control::settings`
for `check` and what a key needs;
`rune_ui::tests::assistant_jump` (Linux) drives a jump and Back by click and
Cmd+[, and the landing at two window heights.

The live eval, `scripts/assistant-eval`, is opt in and never in CI: with
`HEX_ASSISTANT_EVAL_ENGINE` set (`codex`, `codex:<model>`, `local:<id>`,
`openrouter`, `custom`, `opencode`) it asks that engine the guide questions
of `tests/fixtures/assistant/questions.jsonl`, then setup's questions, its
follow-ups and a plan for fixed answers, and a vocabulary request, through `worker::ask`
with the repair round, and reports what each reply made: its cards, what
was refused, and the expected changes it missed. A remote engine is paid
per request. Not yet verified end to end by asking a real model from a
running app.
