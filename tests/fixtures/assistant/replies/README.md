# Assistant reply corpus

Replies as models write them, each with the parse the app must make of it.
`src/assistant/changes/corpus.rs` reads every `*.md` here but this one.

Each file starts with a header between two `---` lines, then the reply as the
model wrote it. Every header field is optional; a field present is checked.

| Field | What is checked |
|---|---|
| `body` | the first line of what the pane shows |
| `proposals` | setting lines, `key=value` as JSON, `;` between them (a choice's `options` as its value) |
| `plan` | how many lines came from a `hex-plan` block |
| `terms` | vocabulary terms, `,` between them |
| `asks` | the `hex-ask` question ids, `,` between them |
| `own_asks` | `true` when setup falls back to the app's own questions |
| `follow_ups` | the follow-up questions setup keeps from a `hex-ask` block, `,` between them |
| `malformed` | lines in a change block that weren't a change |
| `cards` | cards made against default settings |
| `refused` | refusals as `key:why` (`unknown`, `invalid`, `never`, `malformed`), `,` between them |

Whatever the header says, the body never shows a change block.
