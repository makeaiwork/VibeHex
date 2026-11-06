# Files (Labs, macOS)

Labs › Files (`labs.files`, off by default, added in VibeHex) adds a Files pane:
drop or choose any audio or video file, and Hex transcribes it, cleans it up,
and reworks the text with your own requests. Each result is a tab of the
file's text. Mockups: [docs/design/files](../design/files/README.md).

```ts
Labs › Files on (macOS)                         // labs::Feature::Files, rune_ui::files
├── Sidebar  Files joins after Meetings; its header switch is Files | Settings
├── Add      FILES-ADD
│   ├── Drop files on the pane -> each supported one (files::decode::EXTENSIONS:
│   │   mp3, m4a, aac, wav, aif(f), caf, flac, ogg, opus, m4b, mp4, mov, m4v,
│   │   3gp, webm) is added and queued; others -> "<name> isn't an audio or
│   │   video file Hex can read."; the dashed zone lights while a drag is over
│   └── Choose file… -> the open dialog starts in files.last_folder, which the
│       chosen file's folder replaces
├── Queue    FILES-QUEUE (files::worker, one "hex-files" thread, one job at a time)
│   ├── Transcribe -> Extracting audio (scrawl-helper audio, AVFoundation, 16 kHz
│   │   mono WAV in <support>/files/decoding, 30 min limit) -> Transcribing (the
│   │   meeting chunker on dictation's engine, yielding to dictation; Settings'
│   │   own model loads its own copy) -> the Original tab, with timestamps
│   ├── Clean up after transcription on -> Cleanup queued right after
│   ├── Each row: "1 h 12 min, 2nd in line" with Queued; "Extracting audio" or
│   │   "Transcribing, 54 min" with its percent; "42 min, Structure 45%" while
│   │   a request runs; a thin bar under every running job; when done
│   │   "42 min, 3 versions"; a failure in red
│   ├── Stop (Remove while queued) ends a job between chunks or pieces
│   └── The queue empties -> the built-in engine goes back to its 8K window
├── Tabs     FILES-TABS (rune_ui::files::tabs): at most three, Original,
│   │        Cleanup, Requests; every version and job lands in its kind's tab
│   ├── A tab with more than one shows its count; the open one adds ▾, which
│   │   lists them newest first under day headers (Today, Yesterday, a date):
│   │   name (a typed request in quotes, two lines at most), model ↳ source
│   │   tab, time, ✓ on the open one; a job shows its stage and percent
│   ├── Clicking a tab opens the version last opened there (until Hex quits),
│   │   else its newest; a new version opens at once
│   ├── The version line ends with "1 of 3" while its tab holds more
│   ├── Original: Timestamps | Plain text (files.show_timestamps opens it),
│   │   Transcribe again ▾ ("Transcribe the audio again with": downloaded
│   │   models, "Same model" on the one that made the first Original; "Adds a
│   │   version to Original"; More models in General); a model that takes a
│   │   language asks for it first in a row under the line, starting at the
│   │   saved one, with Transcribe and Cancel; Clean up (a small button
│   │   with sparkles, as compact as Transcribe again) while no Cleanup was
│   │   made from it
│   ├── Cleanup: "<model> in an 8K window", Clean up again ▾ (Built in,
│   │   downloaded / Set up in Output › AI cleanup / More models in Output ›
│   │   AI cleanup)
│   ├── Requests: the request's name as a tag, "Made from <tab> by <model>,
│   │   32K window"; each request tab is a session, a chat: a typed request
│   │   as a bubble (a chip's request shows no bubble), its answer, then each
│   │   follow-up as a bubble with its answer. Every answer ends in one row:
│   │   Copy and Ask again as quiet icons in the grey of the model and the
│   │   time, the model, Stopped in amber when stopped (what came is kept),
│   │   and the day and time. A follow-up reads
│   │   the same source tab with the conversation so far (earlier answers cut
│   │   at 6,000 characters); while it runs, "<model> is answering" under the
│   │   streaming draft. Saved in the version: engine, effort, follow_ups
│   ├── A three-step version: a note with × (closed for that version for good)
│   └── A running job's tab turns a ring before its name and shows "Made from
│       Cleanup by Qwen3 4B, 32K window", then a card: title and percent,
│       Stop, a thin bar, and for three steps each step with a green tick,
│       a turning ring, or an empty ring; under it what the job writes so
│       far, as Markdown with a cursor ("Summary of part 2 so far:", "Outline
│       so far:", "Text so far:"; its last ~1,500 characters), streamed from
│       the built-in engine, OpenRouter, or Your server as it arrives
│       (OpenCode answers whole, so its draft grows a piece at a time); the
│       chips and field stay, faded, saying "Wait for <name>, or stop it"
├── Header   The name in up to two lines (a long word breaks anywhere), then the
│            day and time (24-hour where the Mac uses it), ⏱ length, 🎙 model;
│            list rows wrap names the same way
├── Actions  Copy ▾ (as Markdown / as plain text), Edit (Save marks it edited),
│            … (Show in Finder, Open original with the app that opens it
│            named on the right; Save as… .md or .txt; Delete this version…
│            (not the first Original), Delete file from list… (red icon);
│            then where the file is, "…/<folder>/<name>" with a long name cut
│            in its middle, or "File not found" with the first two off), each
│            delete confirmed under the tabs; the file on disk is never touched
├── Ask      FILES-ASK: floats over the bottom of the text, which scrolls on
│            under it and fades out above it; once text is under it the
│            field lifts off with the floating shadow. On an Original or
│            Cleanup tab: quick chips (files.quick_prompts) and Recent ▾
│            (last 10 typed requests, <support>/files/recent.json) as pills,
│            then "Ask about this text" -> starts a session in Requests. On a
│            session: "Ask a follow-up". The field: Return sends, Shift+Return
│            a new line; under the text a model pill ("Model for the next
│            session" / "for this session": Codex's models, read from Codex
│            when it opens, then the engines Settings offers) and, on Codex,
│            a Low / Medium / High pill; the round button turns Stop while an
│            answer comes
└── Settings Rows lead with icons; each choice is borderless and shows what it
             resolves to in grey ("Same as dictation  Parakeet v3"). Model
             (Same as dictation or a downloaded one), Language (files.language,
             only for a model that takes a hint: Same as dictation or one of
             its languages; a new model choice clears it), Use vocabulary, Show
             timestamps; Cleanup: Clean up after transcription, Runs on (Same
             as dictation / built in / OpenRouter / Your server / Codex once
             Output › AI cleanup chose it / OpenCode),
             Use the Default mode's prompt (its start in italics while on;
             off, Files' own prompt); Requests › Runs on (files.request_engine:
             Same as cleanup or any of those), Reasoning (files.request_effort,
             only while requests run on Codex; Medium when unset); Quick requests
             (the chips' names and the prompts' language, Default or Edited,
             Edit…) -> its own page: ‹ Settings, the language as a tag with
             why (the speech model's language: chosen, or the only one the
             model speaks; else the app's), the list (Edited in amber, + Add
             request) beside Name and the prompt, then Default prompt /
             Edited / Your own request, Remove, Reset to default, Save.
             Structure, Summary and Action items start from Hex's prompt in
             that language (app_settings::prompt_defaults,
             prompts/quick.json: en ru uk pl de es pt fr ja zh zh-Hant,
             English for any other); saved with an empty prompt they follow
             the language, and one saved with its old English default
             follows it too
```

```ts
How much a model reads                          // files::rewrite
├── Cleanup  always pieces of ~400 words (dictation/cleanup_pieces) in the 8K window;
│            a failed piece keeps its text and the version says which
└── Request  counted with the built-in server's /tokenize: instructions + text +
             room for an answer as long as the text (1K–8K tokens)
             ├── fits 8K -> 8K; fits 32K -> the server restarts at 32K
             │   (flash attention, q8_0 KV cache), and stays there until the queue
             │   empties
             └── neither -> 3 steps over ~2,500-word parts: summaries -> outline
                 -> each part with the outline -> joined
             Other servers estimate tokens (3 characters each): OpenRouter and
             OpenCode read 128K, Codex 250K (a long file whole), Your server 8K
             (1,000-word parts)
```

Records: `<support>/files/<id>.json`, owner-only, one per file (name, path,
length, versions with segments, model, window, parts, edited, note closed).
Only the path is kept, never a copy of the file. No retention or caps: a file
stays until Delete file from list….

Checks: `files::store`, `files::text`, `files::rewrite`
(`requests_take_the_usual_window_then_the_wide_one_then_three_steps`, the
three-step fake generator and its drafts, a draft's tail),
`dictation::cleanup_server` (a streamed answer's text so far),
`files::worker` (tab names), `files::requests` (a stop keeps what came, a
follow-up carries the session); scenes `files`, `files-session` (a session on
Codex with a follow-up streaming),
`files-working`, `files-original`, `files-summary`, `files-versions` (the
open tab's list), `files-more` (the … menu), `files-again` (Transcribe
again's language), `files-empty`, `files-settings`, `files-settings-prompt`
(Files' own prompt), `files-quick` (the Quick requests page on Russian); no goldens yet.
`app_settings::files_settings` checks how files.language and the model
combine.

Gaps: macOS only (decoding needs AVFoundation). The queue doesn't survive a
quit: a file that was transcribing lists as Stopped, with Try again. Text
renders through rune-kit's `Markdown` (its `markdown` feature,
`pulldown-cmark`), selectable. The full name and request show on no hover
(Rune has no tooltip here); the row's accessibility label holds them. Not yet
verified end to end on a Mac with real files.
