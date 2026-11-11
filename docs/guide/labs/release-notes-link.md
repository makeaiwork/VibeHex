---
feature: release-notes-link
key: labs.release_notes_link
title: Release notes link
for: []
ask: Show a link to this version's release notes?
group: Dictation
os: every OS
vibehex: false
---

# Release notes link

**What it does.** Adds a Release notes link at the bottom of the sidebar, under the version and the update status. It opens the GitHub release page of the version you are running, with no update needed.

**Turn it on when.** You want to see what your current version changed.

**Leave it off when.** Nothing to lose; off, the sidebar footer is as before.

**Needs.** A published build whose version is a VibeHex release (like 3.3.0-vibe.6) or a plain 3.3.0. A build made from source or a test build shows no link, and neither does a -dev version such as 3.3.0-dev-4.

**Good to know.** Only dev builds of VibeHex list this lab. Off by default.

**Where.** Labs › Release notes link (`labs.release_notes_link`).

**Asked as.** "What changed in my version?"; "Where are the release notes?"; «Что нового в этой версии»; «Где посмотреть список изменений».

<!-- sources: src/rune_ui/labs.rs:420-424; src/labs.rs:259-277 (vibehex_hides), 530-531; src/labs/group.rs:56; src/rune_ui/sidebar.rs:640-700, 748-756; src/updater/notes.rs:12-14, 88-113, 235-250; src/control/settings.rs:365-368 -->
