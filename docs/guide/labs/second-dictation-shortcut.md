---
feature: second-dictation-shortcut
key: labs.second_dictation_shortcut
title: Second dictation shortcut
for: [dictation]
ask: Add a second dictation key, such as one on an external keyboard?
then: Record its key in General › Second dictation shortcut
group: Shortcuts
os: macOS
vibehex: false
---

# Second dictation shortcut

**What it does.** Adds a second shortcut that dictates exactly like the main
one. Useful when the main shortcut is Globe/Fn on the laptop keyboard and you
also want a key on an external keyboard.

**Turn it on when.** You switch between the laptop keyboard and an external
one, or want a spare key for dictating.

**Leave it off when.** One shortcut is enough. Off, nothing is watched for it.

**Needs.** macOS. After turning it on, record a key in the new row; it starts
empty and can be cleared.

**Good to know.** It follows the main shortcut's Shortcut mode and double-tap
settings and pastes with the same processing. Paste last dictation still
belongs to its own shortcut. VibeHex refuses a key another VibeHex shortcut
already uses. If one shortcut started a recording, only that one finishes it.
Turning the switch off or clearing the key during a recording cancels it.

**Where.** Labs › Second dictation shortcut
(`labs.second_dictation_shortcut`). The recorder: General › Second dictation
shortcut, under Shortcut (`second_dictation_hotkey`), shown only while
the lab is on.

**Asked as.** "Another dictation key for my external keyboard"; "Two shortcuts
for dictation"; «Вторая клавиша для диктовки»; «Хочу диктовать и с Fn, и с
клавиши внешней клавиатуры».

<!-- sources: docs/features/labs.md:61-63,313-336; src/rune_ui/general.rs:84-100; src/app_settings.rs:1421-1430; src/rune_ui/labs.rs:344-353 -->
