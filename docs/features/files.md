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
│   ├── Each row: Queued / percent, then "42 min, 3 versions"; a failure in red
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
│   │   saved one, with Transcribe and Cancel; Clean up while no Cleanup was
│   │   made from it
│   ├── Cleanup: "<model> in an 8K window", Clean up again ▾ (Built in,
│   │   downloaded / Set up in Output › AI cleanup / More models in Output ›
│   │   AI cleanup)
│   ├── Requests: the request's name as a tag, "Made from <tab> by <model>,
│   │   32K window"
│   └── A three-step version: a note with × (closed for that version for good)
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
├── Ask      FILES-ASK: under a rule, quick chips (files.quick_prompts) and
│            Recent ▾ (last 10 typed requests, <support>/files/recent.json)
│            as pills, then "What should I do with this text?" -> works on
│            the open version and adds the result to Requests
└── Settings Model (Same as dictation or a downloaded one), Use vocabulary,
             Show timestamps, Clean up after transcription, Runs on (Same as
             dictation / built-in / OpenRouter / Your server / OpenCode), Use
             the Default mode's prompt or Files' own; Requests › Runs on
             (files.request_engine: Same as cleanup or any of those); Quick
             requests
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
             OpenCode read 128K, Your server 8K (1,000-word parts)
```

Records: `<support>/files/<id>.json`, owner-only, one per file (name, path,
length, versions with segments, model, window, parts, edited, note closed).
Only the path is kept, never a copy of the file. No retention or caps: a file
stays until Delete file from list….

Checks: `files::store`, `files::text`, `files::rewrite`
(`requests_take_the_usual_window_then_the_wide_one_then_three_steps`, the
three-step fake generator), `files::worker` (tab names); scenes `files`,
`files-working`, `files-original`, `files-summary`, `files-versions` (the
open tab's list), `files-more` (the … menu), `files-again` (Transcribe
again's language), `files-empty`, `files-settings` (no goldens yet).

Gaps: macOS only (decoding needs AVFoundation). The queue doesn't survive a
quit: a file that was transcribing lists as Stopped, with Try again. Text
renders through rune-kit's `Markdown` (its `markdown` feature,
`pulldown-cmark`), selectable. The full name and request show on no hover
(Rune has no tooltip here); the row's accessibility label holds them. Not yet
verified end to end on a Mac with real files.
