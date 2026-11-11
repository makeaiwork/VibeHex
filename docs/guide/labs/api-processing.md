---
feature: api-processing
key: labs.api_processing
title: Process API transcripts
for: []
ask: Let your own scripts get cleaned-up transcripts from VibeHex?
group: Output
os: every OS
vibehex: false
---

# Process API transcripts

**What it does.** Apps on this computer can get transcripts from VibeHex's local API, normally as raw words. With this on, an app that adds `process=global` to its request also gets the text a dictation would paste under Global mode: your replacements, corrections, AI rewrite and transformations. The raw text stays in the answer.

**Turn it on when.** An app or script you use should get finished text from VibeHex.

**Leave it off when.** Nothing uses the local API.

**Needs.** An app that asks for processed text, and the running VibeHex app; `hex service` can't process. With the lab off, such a request is refused.

**Good to know.** No app or website mode applies, only Global. It never pastes, writes History or touches the clipboard. One request is processed at a time; one that arrives meanwhile is turned away as busy. A stage that fails keeps its input.

**Where.** Labs › Process API transcripts (`labs.api_processing`).

**Asked as.** "My app gets raw text from the API"; "Apply my corrections to API transcripts"; «Почему через API приходит сырой текст»; «Как получить через API текст с заменами и AI-правкой».

<!-- sources: docs/features/labs.md:89-95, 1710-1758; src/local_api/processing.rs:1-9, 71-80 (one slot), 122-157 (gate, process=global, refusals 409/403/429), 195-217 (macOS processor, Global context only); src/rune_ui/labs.rs:766-771 (row, no OS limit); src/labs.rs:1384-1389, 345, 534 (on_this_os every OS, audience Everyone); src/labs/group.rs:84 (Output); src/control/settings.rs:672-677 -->
