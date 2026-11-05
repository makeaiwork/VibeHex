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
├── Tabs     FILES-TABS: Original, Cleanup, each request; a job shows as "Name…"
│   ├── Original: Timestamps | Plain text (files.show_timestamps opens it),
│   │   Transcribe again ▾ (downloaded models; new tab "Original (<model>)"),
│   │   Clean up while no Cleanup was made from it
│   ├── Cleanup: "<model> in a 8K window", Clean up again ▾ (Built in,
│   │   downloaded / Set up in Output › AI cleanup / More models in Output ›
│   │   AI cleanup); new tab "Cleanup (<model>)"
│   ├── Request: "Made from <tab> by <model>, 32K window"
│   └── A three-step version: a note with × (closed for that version for good)
├── Actions  Copy ▾ (as Markdown / as plain text), Edit (Save marks it edited),
│            … (Save as… .md or .txt, Reveal file in Finder, Delete this
│            version… (not the first Original), Delete file from list…), each
│            delete confirmed under the tabs; the file on disk is never touched
├── Ask      FILES-ASK: quick chips (files.quick_prompts), Recent ▾ (last 10
│            typed requests, <support>/files/recent.json), "What should I do
│            with this text?" -> works on the open tab and opens a new one
└── Settings Model (Same as dictation or a downloaded one), Use vocabulary,
             Show timestamps, Clean up after transcription, Runs on (Same as
             dictation / built-in / OpenRouter / Your server / OpenCode), Use
             the Default mode's prompt or Files' own, Quick requests
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
`files-working`, `files-original`, `files-summary`, `files-empty`,
`files-settings` (no goldens yet).

Gaps: macOS only (decoding needs AVFoundation). The queue doesn't survive a
quit: a file that was transcribing lists as Stopped, with Try again. Markdown renders through a small block renderer
(headings, lists, quotes, code, rules), with inline marks dropped; rune-kit's
`Markdown` needs its `markdown` feature (`pulldown-cmark`). Many tabs overflow
the card's width. Not yet verified end to end on a Mac with real files.
