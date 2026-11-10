---
feature: meeting-summary-model
key: labs.meeting_summary_model
title: Choose meeting summary model
group: Meetings
os: macOS
vibehex: false
---

# Choose meeting summary model

**What it does.** When OpenCode writes your meeting titles and summaries,
VibeHex normally tries a list of fast models and uses the first one OpenCode
has. This switch adds a Summary model field where you name the one provider
and model to use, written as provider/model, for example openai/gpt-5.4-mini.

**Turn it on when.** You pay for a particular OpenCode provider or
subscription and want meeting notes to use that one.

**Leave it off when.** The automatic choice works, or Apple Intelligence
writes your notes: it is not affected.

**Needs.** Meetings, and Summarize with OpenCode turned on in Meetings ›
Settings › Advanced, with OpenCode already running on the computer.

**Good to know.** There is no fallback. If the chosen model isn't available,
the meeting shows the error instead of trying another. An empty field, or one
without the provider/ part, keeps the automatic list.

**Where.** Labs › Choose meeting summary model (`labs.meeting_summary_model`);
the field is in Meetings › Settings › Advanced (`meetings.opencode_model`).

**Asked as.** "Which model writes my meeting notes?"; "Use my own OpenCode
model for summaries"; «Как выбрать модель для итогов встречи»; «Заметки
встречи пишет не та модель».

<!-- sources: docs/features/labs.md:879-909; docs/features/meetings.md:35-42; src/meeting/generator.rs:480-505, 520-537; src/rune_ui/platform_settings.rs:405-412, 540-565; src/rune_ui/labs.rs:256-262 -->
