---
feature: meeting-descriptive-filenames
key: labs.meeting_descriptive_filenames
title: Descriptive meeting file names
for: [meetings]
ask: Name meeting files after their titles instead of placeholders?
group: Meetings
os: every OS
vibehex: false
---

# Descriptive meeting file names

**What it does.** VibeHex exports each meeting as a Markdown file named by its date, time and the title it had then, often a placeholder such as "Meeting — 2:18 PM". That name stays. With this on, once notes or live chapters give the meeting a title, or you rename the meeting, the file is renamed to the date, time and the new title.

**Turn it on when.** You browse meeting files in the folder and want names you recognize.

**Leave it off when.** Another tool expects each file to keep its first name.

**Needs.** Meetings, and either notes or a rename to give a title.

**Good to know.** The date and time stay as they were. A taken name gets " (2)", never an overwrite, and unsafe characters become dashes. Titles are cut at 80 characters. If the rename fails, the old name stays. It is off by default.

**Where.** Labs › Descriptive meeting file names (`labs.meeting_descriptive_filenames`), in the Meetings group.

**Asked as.** "Name meeting files after the meeting"; "Rename the exported Markdown to the title"; «Почему файл встречи называется просто датой»; «Как переименовать файл встречи по её названию».

<!-- sources: src/meeting/export_name.rs:1-40, 100-160; src/meeting.rs:150, 1269-1325; src/meeting/notes.rs:995-1006; src/meeting/chapters.rs:915-925; src/rune_ui/labs.rs:462-467; src/labs.rs:1320-1327; src/control/settings.rs:637-641; docs/features/labs.md:1836-1876 -->
