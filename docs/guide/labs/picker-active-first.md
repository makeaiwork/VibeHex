---
feature: picker-active-first
key: labs.picker_active_first
title: Model in use first
for: []
ask: List the model you're using first in the model picker?
group: Models
os: every OS
vibehex: false
---

# Model in use first

**What it does.** In the speech model picker the model you use now can be
hard to find among many. With this on it comes first: the first column in
Compare, and the top of its group in All speech models. The order is fixed
when the picker opens and does not jump while you choose or download.

**Turn it on when.** You open the picker to check or change the model you
already use.

**Leave it off when.** You like the usual order.

**Needs.** The model in use must be installed; otherwise nothing moves.

**Good to know.** If you added your own model file and use it, "Your model
files" goes above the language's group. With Sort and filter models set to By
size or By name, the list keeps that exact order, and By recommendation puts
your model first.

**Where.** Labs › Model in use first (`labs.picker_active_first`), then
General › Speech model. Related: Labs › Sort and filter models
(`labs.picker_sort_filter`).

**Asked as.** "Show the model I'm using at the top"; "Where is my current
speech model in the list?"; «Где в списке моя текущая модель»; «Показать
используемую модель первой».

<!-- sources: docs/features/labs.md:733-757 (the Compare and Every model text, near Model in use first); src/rune_ui/picker.rs:220-231; src/rune_ui/labs.rs:488-492 -->
