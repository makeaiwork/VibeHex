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
│   ├── New     "Ask anything about VibeHex", four example questions, and
│   │           what the guide holds (Labs features and topics)
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
    └── What it reads   Your meetings, files, and dictations (toggle)
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

## Questions about recordings

- `assistant::intent` reads from the question's words alone, against a
  `now` passed in, whether it is about recordings: kinds named (meetings,
  calls, dictations, files, in English and Russian), a time (yesterday, last
  week, in November, on the 24th, the last five meetings), and what was said
  or decided. "How do I…", "можно ли…" without a time stay with the guide, so
  the consent card never comes up for them. A question that only may be
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
- Eval: `tests/fixtures/assistant/recordings` (ten meetings, three Files
  texts, five dictations), 26 questions with expected recordings (recall
  must stay at 90% or more, now 100%), 8 questions whose cited line must be
  offered and map to its time (now 8 of 8; word-timed, segment-only, and
  Files transcripts), and 11 guide questions that must not route
  (`scripts/check assistant::recordings`).

## Settings keys

| Key | Default | Meaning |
| --- | --- | --- |
| `assistant.enabled` | `true` | The sidebar row |
| `assistant.engine` | none | Where answers run, in `files.request_engine`'s form; none follows AI cleanup |
| `assistant.effort` | empty (medium) | Codex reasoning: low, medium, high |
| `assistant.keep_conversations` | `true` | Keep conversations on this computer |
| `assistant.read_recordings` | none | May search meetings, Files texts, and dictations for questions about them; none until first asked |

## Privacy

What reaches a model is the question, the conversation so far, and guide
text; never history, meetings, Files texts, or keys, except the passages
`recordings` found once the person allowed it. Keys never. Kept
conversations live in `<support>/assistant/conversations.json`, readable only
by this account, the 50 most recently used; with keeping off they last until
VibeHex quits. Clear conversations deletes them all.

Checks: `scripts/check assistant` (store, worker, retrieve, prompt, intent,
recordings and its eval, jump, settings keys); scenes `assistant`,
`assistant-answering`, `assistant-start`, `assistant-connect`,
`assistant-settings`, `assistant-consent`, `assistant-steps`,
`assistant-recordings`, `assistant-steps-open`, `meetings-cited`,
`files-cited`, `files-cited-listening`, `history-cited` and their goldens;
`rune_ui::tests::assistant_jump` (Linux) drives a jump and Back by click and
Cmd+[, and the landing at two window heights. Not yet verified
end to end by asking a real model from a running app.
