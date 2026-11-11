---
topic: history
title: History
keys: [history_retention, paste_last_hotkey, labs.keep_recordings]
os: every OS
---

# History

History is the list of your recent dictations: what was pasted, what the speech model first heard, and where. It saves you when a paste went to the wrong place or you need a sentence again. It keeps text only, unless you choose Keep audio too.

## See what it keeps

Open **History** in the sidebar. Each row shows the text, its kind (Dictation; Raw for Dictate without processing; Printed for `hex dictation stop --print`, which is never pasted), the app and the time. Select one to see the full text, the **Raw transcript** (if cleanup changed it), and its facts: **Application**, **Processing** (the cleanup profile and how long it took), **Speech model** (the model that heard it, with the language chosen for that run) and **Latency**. **Copy** puts the text on the clipboard; **Delete** removes one entry.

## Choose how long it keeps things

The menu at the top of the pane reads like "Kept 7 days" (or "Kept forever", or "History off"). Open it to pick Off, 24 hours, 7 days, 30 days or Forever. The default is 7 days. Older entries disappear on their own. Whatever you choose, History never holds more than 10,000 entries or about 16 MB of text. **Off** ("Keeps no new dictations") stops saving new dictations; it does not stop the diagnostic logs.

## Search

Type in **Search history**. It looks in the pasted text, the raw transcript, the app name, the speech model and the processing profile, and shows up to 200 matches.

## Paste the last dictation again

The **Paste last dictation** shortcut (General › Advanced › Output, Option-Shift-V on a Mac, Ctrl+Alt+V on Windows and Linux) and the menu-bar or tray item of the same name paste the newest dictation again.

## Transcribe again with another model

In the retention menu, choose **Keep audio too** (it carries a Lab badge and is the same switch as **Labs › Keep recordings**). From then on, each new dictation saved to History also keeps its audio on this computer. Open such an entry, find **Saved recording**, and open **Transcribe again** to pick any installed model. Some models ask for a language first. The new text appears with **Copy** and **Replace**, which swaps it into the entry.

Replace keeps the text it swapped out. The entry then lists up to 5 **Earlier versions**, newest first, each with its model and age, **Copy** and **Use this version** to bring it back. Entries from before you chose Keep audio too have no audio. Audio shares the entry's retention, and all recordings together are capped at 512 MB (about four and a half hours); past that, the oldest entries keep their text and lose their audio.

## Clear everything

Open the retention menu and choose **Clear all history…**. The menu then gives way to a red **Clear all history** button and **Cancel**; press the button to confirm. This also deletes the kept audio.

**Asked as.** "Where can I see my old dictations?"; "I lost what I dictated"; «Где посмотреть прошлые диктовки?»; «Как заново расшифровать запись другой моделью?»; «Как очистить историю?»

<!-- sources: docs/features/README.md:660-692; docs/features/labs.md:790-834; docs/features/transcript-cli.md:42-50; src/history.rs:29-35, 49-100, 160-175, 540-552; src/history/versions.rs:1-15; src/history/recordings.rs:24; src/rune_ui/history.rs:316-331, 515-565, 702-800, 880-930, 1140-1215; src/rune_ui/general.rs:830-845; src/rune_ui/general/search.rs:62-80; src/app_settings/hotkeys.rs:683-700; src/desktop/tray_menu.rs:160; src/control/settings.rs:340-344 -->
