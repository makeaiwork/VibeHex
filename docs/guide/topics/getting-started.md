---
topic: getting-started
title: Getting started
keys: [dictation_hotkey, microphone, enable_on_launch]
os: every OS
---

# Getting started

VibeHex is a dictation app, a fork of the open-source Hex. You hold a
shortcut, speak, let go, and the words are typed into whatever app has your
cursor. A speech model that VibeHex downloads once turns your voice into text
on your own computer.

## What the first launch does

A short setup opens by itself: a demo, the permissions your system needs, the
speech model download (it starts at once, with the model recommended for your
system's first language), and a practice game, "Your turn". Skip exists only
on the demo and the practice. If you quit part-way, setup resumes. At the end,
"Hex is ready" says where VibeHex lives: the menu bar, system tray, or app
launcher.

To see it again, use Labs › Run setup again; settings, models, and
permissions stay. If the download fails, Continue without a model lets you in,
and dictation waits until General › Speech model offers Download or Add model
file.

## Permissions

- **macOS** asks for Microphone, so it can hear you, and Accessibility, so
  VibeHex notices the shortcut and pastes into other apps (macOS 27 calls it
  "Device Control and Data Access"). It never asks for Input Monitoring. If a
  row stays grey after you switched it on, press "Still grey? Fix it"; if it
  says "Needs a restart", press Restart Hex. A permission that goes missing
  later shows under General › Permissions needed.
- **Windows** asks nothing, unless the microphone switch for desktop apps is
  off in Windows Settings; then "Let Hex hear you" opens that page.
- **Linux** asks nothing from VibeHex itself. On GNOME or KDE with Wayland, a
  page "Let Hex paste for you" has your desktop ask once to allow the paste
  keystroke (it may call it remote desktop). VibeHex never sees your screen.
  If you refuse, it copies your words and you press Ctrl+V.

## The sidebar

- **General**: shortcut, speech model, language, vocabulary, microphone;
  Advanced unfolds more.
- **Output**: what happens to words after they are heard: Vocabulary, Replace
  words, AI cleanup, rules per app.
- **History**: past dictations to search, copy, or delete.
- **Meetings** and **Files** (Mac, from Labs): recording calls, transcribing
  audio or video files.
- **Labs**: optional features. **About**: version, source code, the developer.

The windows come in ten languages, Ukrainian among them but not yet Russian
(General › Advanced › Application › Interface language). You can dictate in
Russian all the same.

## Labs

Labs lists optional, unfinished features. Each is off by default; there is no
master switch. A star marks the features VibeHex added to Hex, and the
"VibeHex only" switch shows just those.

**Asked as.** "How do I start using VibeHex?"; "Why does it ask for
Accessibility?"; «Зачем VibeHex просит универсальный доступ?»; «Как пройти
настройку заново?».

<!-- sources: docs/features/README.md:157-243; AGENTS.md:687-724; src/rune_ui/platform_settings.rs:28-131; src/rune_ui/onboarding_kit/permissions.rs; src/rune_ui/sidebar.rs; src/rune_ui/labs.rs:28-44,752-770; src/labs.rs:341; src/desktop/i18n.rs:26-38 -->
