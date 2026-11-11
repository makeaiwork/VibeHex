---
feature: keyboard-model-switching
key: keyboard_model_switching.enabled
title: Switch model with keyboard language
group: Models
os: [macOS, Windows, Linux (X11 only)]
vibehex: false
---

# Switch model with keyboard language

**What it does.** If you type in several languages, changing the keyboard
layout also changes the speech model and language VibeHex dictates with. You
decide the pairs once: for each layout, pick a model and a language, or
"Don't switch".

**Turn it on when.** You switch between, for example, an English and a Russian
layout and want each to use the model that suits it.

**Leave it off when.** You use one language. Choosing a model by hand always
works either way.

**Needs.** macOS, Windows, or Linux in an X11 session (not Wayland). Mapped
models must already be downloaded; VibeHex never downloads one for this, and
Apple Speech can't be mapped. A new layout starts as "Don't switch".

**Good to know.** The switch happens a moment after the layout changes. Turning
it off leaves the current model as it is.

**Where.** Labs › Switch model with keyboard language
(`keyboard_model_switching.enabled`). The pairs: General › Advanced ›
Dictation, one row per layout.

**Asked as.** "Use a different model for Russian and English layouts";
"Switch the language automatically when I change keyboard layout";
«Переключать модель вместе с раскладкой клавиатуры»; «Диктовка не меняет язык
при смене раскладки».

<!-- sources: src/labs.rs:314,366,421,481-483; src/rune_ui/labs.rs:289-297; docs/features/README.md:531-585 (Switch models with keyboard language); docs/features/labs.md:47; docs/cli.md:378-386; src/rune_ui/general.rs:880-890; src/rune_ui/settings.rs:1001,1053; src/rune_ui/platform.rs:60-66 -->
