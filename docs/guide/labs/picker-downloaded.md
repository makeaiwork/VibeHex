---
feature: picker-downloaded
key: labs.picker_downloaded
title: Downloaded models only
group: Models
os: every OS
vibehex: false
---

# Downloaded models only

**What it does.** Shows what the speech models cost you in disk space, and
lets you hide the ones you have not downloaded. In All speech models a row
says "Downloaded models take" a size, with a switch "Only downloaded models".

**Turn it on when.** You want to free space and need to see which models are
on the computer, or the full list is too long to scan.

**Leave it off when.** You browse the whole catalog.

**Needs.** Nothing else.

**Good to know.** The size counts only the files VibeHex downloaded into its
own models folder. Files another app downloaded, and model files used where
they are, are not counted. With the switch on, each row keeps its Delete
button, except the model in use and one being prepared (Forget for a file used where it is), and the list leaves out "On this
computer". The switch is not saved and resets when the picker reopens.

**Where.** Labs › Downloaded models only (`labs.picker_downloaded`), then
General › Speech model › All speech models.

**Asked as.** "How much space do the models take?"; "Show only the models I
downloaded"; «Сколько места занимают модели»; «Как удалить ненужные модели».

<!-- sources: docs/features/labs.md:765-785; src/rune_ui/picker.rs:98-99, 166-195, 529-547, 700-740; src/rune_ui/labs.rs:500-505 -->
