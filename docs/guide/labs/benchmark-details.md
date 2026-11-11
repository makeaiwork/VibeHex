---
feature: benchmark-details
key: labs.benchmark_details
title: Benchmark details
for: []
ask: Show where each speech model's accuracy figure comes from?
group: Models
os: every OS
vibehex: false
---

# Benchmark details

**What it does.** Shows where a model's accuracy figure comes from. Under
each model in the picker's full list, one line gives the raw error rate (such
as "7.57% CER", lower is better), then the language, the test set, the source,
and the test conditions. Anything the source did not record reads "Dataset
unknown", "Source unrecorded", or "Conditions unknown".

**Turn it on when.** You compare models and want to know whether two figures
were measured the same way.

**Leave it off when.** The simple "wrong words" figures are enough.

**Needs.** Nothing else. Models with no benchmark record, such as Apple
Speech, show nothing extra.

**Good to know.** A "Most accurate" badge stays only when its figure is
comparable with the other recommended models' (same measure, language, and
test set), so with this on the badge can disappear. Some figures are published by
others, not measured in VibeHex.

**Where.** Labs › Benchmark details (`labs.benchmark_details`), then
General › Speech model › All speech models.

**Asked as.** "Where does the accuracy number come from?"; "What does CER
mean in the model list?"; «Откуда взялись цифры точности моделей»; «Что значит
CER и WER».

<!-- sources: docs/features/labs.md:457-486; src/rune_ui/picker.rs:233-262, 265-300; src/rune_ui/picker/list.rs:273-286; src/rune_ui/labs.rs:483-487 -->
