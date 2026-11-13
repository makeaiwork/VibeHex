---
feature: vocabulary-quick-add
key: labs.vocabulary_quick_add
title: Add to vocabulary from a selection
for: [dictation, code]
ask: Fix a misheard word by selecting it and pressing a shortcut?
group: Dictation
os: macOS
vibehex: true
---

# Add to vocabulary from a selection

**What it does.** Fixes a misheard word where you see it. Select the wrong
word in any app and press a shortcut (Option+Shift+A by default). A card opens
with Heard (your selection) and Should be (the right spelling). Add saves it
to your Vocabulary and, where the text can be edited, replaces the selection.

**Turn it on when.** The same names keep coming out wrong and you would rather
fix them on the spot than open Vocabulary.

**Leave it off when.** The shortcut clashes with one you use (you can change it).

**Needs.** macOS. It reads the selection through Accessibility, or copies it
with Command+C when an app gives no access, then restores your clipboard.

**Good to know.** If AI cleanup wrote the word, the card says so and Heard
shows what the speech model heard. Cancel, Escape, or a click elsewhere saves nothing.

**Where.** Labs › Add to vocabulary from a selection
(`labs.vocabulary_quick_add`). Shortcut: General › Output › Add selection to
vocabulary (`vocabulary_add_hotkey`), shown only while the lab is on.

**Asked as.** "Add a misheard word to the Vocabulary from any app"; «Как
быстро добавить неправильно написанное слово в словарь»; «Выделил слово, хочу
исправить и запомнить».

<!-- sources: docs/features/labs.md:1166-1214; docs/features/vocabulary.md:36-38; src/labs.rs:310 (macOS), 402; src/rune_ui/labs.rs:299-302; src/rune_ui/general.rs:931-946; src/app_settings/hotkeys.rs:678-690 -->
