---
topic: history
title: History
keys: [history_retention, paste_last_hotkey, labs.keep_recordings]
os: every OS
---

# History

History is the list of your recent dictations: what was pasted, what the speech model first heard, and where. It saves you when a paste went to the wrong place or you need a sentence again. It keeps text only, unless you turn on Keep recordings.

## See what it keeps

Open **History** in the sidebar. Each row shows the text, its kind (Dictation; Raw for Dictate without processing; Printed for `hex dictation stop --print`, which is never pasted), the app and the time. Select one to see the full text, the **Raw transcript** (if cleanup changed it), the app, the mode used and the timing. **Copy** puts the text on the clipboard; **Delete** removes one entry.

## Choose how long it keeps things

The **Keep** menu at the top offers Off, 24 hours, 7 days, 30 days and Forever. The default is 7 days. Older entries disappear on their own. Whatever you choose, History never holds more than 10,000 entries or about 16 MB of text. **Off** stops saving new dictations; it does not stop the diagnostic logs.

## Search

Type in **Search history**. It looks in the pasted text, the raw transcript, the app name and the mode, and shows up to 200 matches.

## Paste the last dictation again

The **Paste last dictation** shortcut (General › Output, Option-Shift-V on a Mac, Ctrl+Alt+V on Windows and Linux) and the menu-bar item of the same name paste the newest dictation again.

## Transcribe again with another model

Turn on **Labs › Keep recordings**. From then on, each new dictation saved to History also keeps its audio on this computer. Open such an entry, find **Saved recording**, and open **Transcribe again** to pick any installed model. Some models ask for a language first. The new text appears with **Copy** and **Replace**, which swaps it into the entry. Entries from before you turned it on have no audio. Audio shares the entry's retention, and all recordings together are capped at 512 MB (about four and a half hours); past that, the oldest entries keep their text and lose their audio.

## Clear everything

Press **Clear all**, then **Really clear all?** to confirm. This also deletes the kept audio.

**Asked as.** "Where can I see my old dictations?"; "I lost what I dictated"; «Где посмотреть прошлые диктовки?»; «Как заново расшифровать запись другой моделью?»; «Как очистить историю?»

<!-- sources: docs/features/README.md:660-692; docs/features/labs.md:790-834; docs/features/transcript-cli.md:42-50; src/history.rs:23-95, 108-150, 405-515; src/history/recordings.rs:1-30; src/rune_ui/history.rs:395-470, 520-560, 590-880; src/rune_ui/general.rs:918-932; src/app_settings/hotkeys.rs:658-670; src/desktop/tray_menu.rs:160 -->
