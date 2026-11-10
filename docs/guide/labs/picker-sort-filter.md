---
feature: picker-sort-filter
key: labs.picker_sort_filter
title: Sort and filter models
group: Speech models
os: every OS
vibehex: false
---

# Sort and filter models

**What it does.** The list of speech models is long. With this on, the full
list (All speech models) gets a row under its title: a sort, By
recommendation, By size, or By name, and a switch "Only models that support"
your language ("Only models that detect the language automatically" when the
language is Auto).

**Turn it on when.** You look for a small model, or one by name, or want to
hide models that cannot write your language.

**Leave it off when.** The usual order is enough.

**Needs.** Nothing else.

**Good to know.** By size puts the smallest file first and models with no
known size last. Both only change what you see: they are not saved, reset
when the picker reopens, and never change your language or model. If the switch hides the model in use or one being
downloaded, a line above the list still names it.

**Where.** Labs › Sort and filter models (`labs.picker_sort_filter`), then
General › Speech model › All speech models.

**Asked as.** "Sort the speech models by size"; "Show only models for my
language"; «Как отсортировать модели по размеру»; «Как показать только модели
для русского».

<!-- sources: docs/features/labs.md:434-455; src/rune_ui/picker.rs:55-100, 318-346, 529-547, 636-680; src/rune_ui/labs.rs:477-481 -->
