---
feature: api-processing
key: labs.api_processing
title: Process API transcripts
group: Output
os: every OS
vibehex: false
---

# Process API transcripts

**What it does.** Apps on this computer can get transcripts from VibeHex's local API, normally as raw words. With this on, an app can ask for the text a dictation would paste under Global mode: its replacements, corrections, AI rewrite and transformations.

**Turn it on when.** An app or script you use should get finished text from VibeHex.

**Leave it off when.** Nothing uses the local API.

**Needs.** An app that asks for processed text.

**Good to know.** It never pastes, writes History or touches the clipboard. One request is processed at a time.

**Where.** Labs › Process API transcripts (`labs.api_processing`).

**Asked as.** "My app gets raw text from the API"; «Почему через API приходит сырой текст».

<!-- sources: docs/features/labs.md:89-95, 1710-1758; src/rune_ui/labs.rs:766-771; src/labs.rs:1384-1389; src/control/settings.rs:672-677 -->
