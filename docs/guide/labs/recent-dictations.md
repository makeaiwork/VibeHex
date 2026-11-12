---
feature: recent-dictations
key: labs.recent_dictations
title: Copy recent dictations
for: [dictation, code]
ask: Copy any of your last five dictations from the menu bar?
group: History
os: macOS
vibehex: false
---

# Copy recent dictations

**What it does.** Adds a "Copy Recent Dictation" submenu to VibeHex's
menu-bar menu, under Paste Last Dictation. It lists your last five
dictations; clicking one copies its full text to the clipboard.

**Turn it on when.** You need a dictation from a few minutes ago, not just
the very last one, for example after a wrong paste or when you dictated into
the wrong app.

**Leave it off when.** Paste Last Dictation is enough for you.

**Needs.** macOS and History turned on: the list comes from History, within
its retention. Nothing else.

**Good to know.** Nothing is pasted, only copied. Each entry shows one short
line (about 48 characters). Dictations printed with `hex dictation stop
--print` and empty ones are skipped. With History retention Off, or nothing
to list, the submenu stays hidden.

**Where.** Labs › Copy recent dictations (`labs.recent_dictations`).

**Asked as.** "Get back an older dictation"; "Where are my last
dictations in the menu bar?"; «Как достать одну из последних диктовок»;
«Нужна не последняя диктовка, а предыдущая».

<!-- sources: docs/features/labs.md:72, docs/features/labs.md:632-652; src/labs.rs:308; src/rune_ui/labs.rs:381-388; src/macos_rune/status_item.rs:246-253, 557-602; src/history.rs:446-463 -->
