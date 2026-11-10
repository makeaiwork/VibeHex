---
feature: model-in-place
key: labs.model_in_place
title: Use model files where they are
group: Speech models
os: every OS
vibehex: false
---

# Use model files where they are

**What it does.** Normally "Add model file…" copies your own speech model
(a GGUF file) into VibeHex. With this on, the picker also offers "Use model
file where it is…", which uses the file at its current place without copying
it. Several accounts on one computer can share a single large file.

**Turn it on when.** You have a big model file in a shared folder, or several
users who would each otherwise keep a copy.

**Leave it off when.** You have one account; copying is fine.

**Needs.** A GGUF model file. It is checked and fingerprinted once.

**Good to know.** The button sits beside Add model file in All speech models.
If another account changes or removes the file, the model shows as Changed or
Missing. Forget removes only VibeHex's record; the file stays where it is.
Turning it off removes the button; Add model file copies as before.

**Where.** Labs › Use model files where they are (`labs.model_in_place`), then
General › Speech model › All speech models.

**Asked as.** "Add a model without copying it"; "Share one model between
macOS accounts"; «Как использовать файл модели без копирования»; «Одна модель
на несколько пользователей».

<!-- sources: docs/features/labs.md:759-774 (Use model files where they are); docs/features/own-models.md:52-72; src/rune_ui/model_import.rs:76-97; src/rune_ui/picker.rs:529-541; src/rune_ui/labs.rs:494-498 -->
