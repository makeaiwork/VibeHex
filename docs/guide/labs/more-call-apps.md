---
feature: more-call-apps
key: labs.more_call_apps
title: More call apps
group: Meetings
os: every OS
vibehex: false
---

# More call apps

**What it does.** VibeHex offers to record calls it recognizes. With this on, it also recognizes Signal, Telegram, WeChat, LINE, Webex, Element, and the Phone app. On a Mac it also names calls in browser pages of Google Voice, Messenger, Teams, FaceTime, GroupMe, and Webex.

**Turn it on when.** You take calls in those apps or pages and want recording offered.

**Leave it off when.** You don't want offers for them.

**Needs.** Labs › Meetings (beta) on. Browser pages are read on macOS only.

**Good to know.** Signal, Telegram, WeChat, LINE, Element, Messenger, and GroupMe wait about 20 seconds before offering, because voice notes use the microphone too. Webex and Phone wait 2 seconds. The apps, Google Voice, Messenger, and GroupMe then appear in Meetings › Settings › Detected calls, each set to Ask, Always record, or Never offer.

**Where.** Labs › More call apps (`labs.more_call_apps`).

**Asked as.** "Record Telegram calls"; "Can it offer to record Signal calls?"; «Записывать звонки в Телеграме»; «Запись звонков в Signal».

<!-- sources: src/rune_ui/labs.rs:480-485; src/microphone_activity.rs:55-62, 128-140; src/meeting/detection.rs:16-20, 48-62, 272-290, 817-828, 856-871; src/app_settings/meetings.rs:272-297; src/rune_ui/platform_settings.rs:925-977; src/rune_ui/platform.rs:848-849; src/control/settings.rs:543-546 -->
