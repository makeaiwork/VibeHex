---
feature: release-notes-link
key: labs.release_notes_link
title: Release notes link
group: Dictation
os: every OS
vibehex: false
---

# Release notes link

**What it does.** Adds a Release notes link under the version at the bottom of the sidebar. It opens, on GitHub, the release page of the version you are running.

**Turn it on when.** You want to see what your current version brought.

**Leave it off when.** Nothing to lose; off, the footer is as before.

**Needs.** A published build. Builds made from source show no link.

**Good to know.** Stable VibeHex builds don't list this lab; only dev builds do.

**Where.** Labs › Release notes link (`labs.release_notes_link`).

**Asked as.** "What changed in my version?"; «Что нового в этой версии».

<!-- sources: docs/features/labs.md:36-40, 52, 363-381; src/rune_ui/sidebar.rs:652-690, 748-756; src/updater/notes.rs:16-19, 88-113; src/labs.rs (vibehex_hides, shipped) -->
