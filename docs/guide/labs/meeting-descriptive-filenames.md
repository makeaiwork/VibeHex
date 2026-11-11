---
feature: meeting-descriptive-filenames
key: labs.meeting_descriptive_filenames
title: Descriptive meeting file names
group: Meetings
os: every OS
vibehex: false
---

# Descriptive meeting file names

**What it does.** A meeting's exported Markdown is named by its date and time, and that name stays. With this on, once notes give the meeting a title, or you rename it, the file is renamed to the date, time and title.

**Turn it on when.** You browse meeting files in the folder and want names you recognize.

**Leave it off when.** Another tool expects each file to keep its first name.

**Needs.** Meetings, and notes or a rename to give a title.

**Good to know.** A taken name gets " (2)", never an overwrite.

**Where.** Labs › Descriptive meeting file names (`labs.meeting_descriptive_filenames`).

**Asked as.** "Name meeting files after the meeting"; «Почему файл встречи называется просто датой».

<!-- sources: docs/features/labs.md:140-143, 1836-1876; src/rune_ui/labs.rs:462-467; src/labs.rs:1323-1327; src/control/settings.rs:637-641, 908 -->
