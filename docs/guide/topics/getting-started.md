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

A short setup opens by itself. Its first page is **Choose a speech model**,
and nothing downloads until you press **Download** there:

- **Dictation language** starts on your system's first language (Auto is
  there too); change it and the list follows.
- Up to three models for that language, the recommended one first and
  selected, each with its badge, one line about it, and the download's size.
  English offers Parakeet Unified English, Cohere Transcribe, and Whisper
  large-v3-turbo; Russian offers GigaAM v3, Whisper large-v3-turbo, Parakeet
  v3, and Orukeet fourth, without a badge. Model files already on this
  computer follow as "On this computer", with **Use this model**.
- **All models…** opens the full picker; **Choose** there only selects a row,
  and Download on the page starts it.
- **Continue without a model** asks for the permissions and lets you in with
  the language you chose kept. Dictation then waits until General › Speech
  model offers Download or Add model file….

After Download the setup goes on while the model arrives (its name and
percent show on top): a demo, the permissions your system needs, and a
practice game, "Your turn". Only the practice waits for the model, on
"Almost ready". Back returns to the list with the download still running; ×
on its row cancels it, and Download on another row replaces it. Skip exists
on the demo, the practice, and Almost ready (the download goes on, and
Settings › General shows it). If you quit part-way, setup resumes. At the
end, "VibeHex is ready" says where VibeHex lives: the menu bar, system tray,
or app launcher. **Set up with Assistant** there starts VibeHex and opens
Assistant's few questions about how you work, then one list of changes;
it needs no model connected.

To see it again, use Labs › Run setup again; settings, models, and
permissions stay.

## Permissions

- **macOS** asks for Microphone, so it can hear you, and Accessibility, so
  VibeHex notices the shortcut and pastes into other apps (macOS 27 calls it
  "Device Control and Data Access"). It never asks for Input Monitoring. If a
  row stays grey after you switched it on, press "Still grey? Fix it"; if it
  says "Needs a restart", press Restart VibeHex. A permission that goes missing
  later shows under General › Permissions needed.
- **Windows** asks nothing, unless the microphone switch for desktop apps is
  off in Windows Settings; then the "Let VibeHex hear you" page has an Open
  Settings button that opens that page.
- **Linux** asks nothing from VibeHex itself. On GNOME or KDE with Wayland, a
  page "Let VibeHex paste for you" has your desktop ask once to allow the paste
  keystroke (it may call it remote desktop). VibeHex never sees your screen.
  If you refuse, it copies your words and you press Ctrl+V.

## The sidebar

From the top, the panes appear in this order:

- **General**: shortcut, speech model, language, vocabulary, microphone;
  Advanced unfolds more.
- **Output**: what VibeHex does to your words before they are pasted:
  Vocabulary, Replace words, and the rules some apps use instead.
- **History**: past dictations to search, copy, or delete.
- **Captures**: the screenshots and recordings Labs › Screenshots saved. It
  shows only while that lab is on (Mac).
- **Meetings**: recording calls, once Labs › Meetings (beta) is on.
- **Files** (Mac): transcribing audio or video files, once Labs › Files is on.
- **Annotate**: drawing on the screen while you record it and talk.
- **Voice Filter** shows only in dev builds of VibeHex, and stays while its
  microphone driver is installed.
- **Labs**: optional features. **About**: version, source code, the developer.

**Assistant**, the in-app helper, sits at the bottom, apart from the others,
and is on by default. The old Modes pane is never shown.

The window comes in eleven languages: English, Polish, Simplified and
Traditional Chinese, Japanese, German, Spanish, Ukrainian, Brazilian
Portuguese, French, and Russian. Choose one in General › Advanced ›
Application › Interface language, or leave it on System to follow your
computer. You can dictate in Russian whichever window language you pick.

## Labs

Labs is a store of optional, unfinished features. Each is off by default;
there is no master switch. At the top, a search field ("Search N labs") finds
a lab by its name, its description, or its group. **Discover** lists every
lab and **Turned on** only the ones you switched on. Chips narrow the list by
group: All, Dictation, Output, History, Meetings, Annotate, Shortcuts, Models
(a group appears only if it has a lab on your system), and a VibeHex chip for
the labs VibeHex added to Hex. Those carry a star, which says "Added in
VibeHex" when you point at it. The newest labs show first under "New in
Labs". Each card has its own switch. At the bottom, Run setup again and Send
feedback.

**Asked as.** "How do I start using VibeHex?"; "Why does it ask for
Accessibility?"; «Зачем VibeHex просит универсальный доступ?»; «Как пройти
настройку заново?».

<!-- sources: docs/features/README.md:157-243; AGENTS.md:687-724; src/rune_ui/platform_settings.rs:28-131; src/rune_ui/onboarding_kit/permissions.rs:190-330; src/rune_ui/onboarding_kit/first_run.rs:505-540; src/rune_ui/onboarding_kit/landing.rs:150-210; src/rune_ui/sidebar.rs:71-147, 270-300, 440-490; src/rune_ui/labs.rs:31-48, 360-366, 610-616, 801-805; src/rune_ui/labs/store.rs:1-60, 150-250, 410-475; src/rune_ui/labs/group.rs:10-20; src/rune_ui/general.rs:1000-1115; src/desktop/i18n.rs:26-42; src/rune_ui/files.rs:139; src/rune_ui/meetings.rs:158; src/labs.rs:460-495 -->
