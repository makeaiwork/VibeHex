---
feature: share-shortcut
key: share_dictation_shortcut
title: Share dictation shortcut with other apps
group: Shortcuts
os: Windows
vibehex: false
---

# Share dictation shortcut with other apps

**What it does.** Normally VibeHex keeps the dictation shortcut's key to
itself, so no other app sees it. With this on, the key also reaches the app in
front, so apps such as Discord can detect the same key, for example for their
own push-to-talk.

**Turn it on when.** Another app has to react to the same key you use to
dictate.

**Leave it off when.** You don't need that. Off, only VibeHex sees the
shortcut.

**Needs.** Windows. Nothing else.

**Good to know.** Both apps now react to one press, so a key that types
something or does something in the app in front will do it while you
dictate. Pick a shortcut that is harmless there. It applies to the dictation
shortcut.

**Where.** Labs › Share dictation shortcut with other apps
(`share_dictation_shortcut`).

**Asked as.** "Discord doesn't see my dictation key"; "Use the same key for
Discord push-to-talk and dictation"; «Discord не видит мою клавишу
диктовки»; «Чтобы другие приложения тоже получали нажатие клавиши».

<!-- sources: src/labs.rs:294,424,486; src/rune_ui/labs.rs:523-529; src/rune_ui/platform.rs:116; src/windows/input.rs:265-266,975-1035 (share_trigger); docs/features/README.md:775-776; docs/features/labs.md:51 -->
