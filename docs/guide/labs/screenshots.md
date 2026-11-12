---
feature: screenshots
key: labs.screenshots
title: Screenshots
for: [code]
ask: Take screenshots you can mark up and talk over?
then: Free ⌘⇧4 in Keyboard Shortcuts › Screenshots, allow Screen Recording
group: Annotate
os: macOS
vibehex: false
---

# Screenshots

**What it does.** Press ⌘⇧4, then drag an area, press Space to pick a window, or click for the whole screen. The shot is copied, saved as a PNG (on the Desktop unless you pick a folder), and shown as a corner card. Click the card to mark it up or talk over it.

**Turn it on when.** You want screenshots with marks and spoken notes.

**Leave it off when.** macOS's own screenshots are enough.

**Needs.** Screen Recording permission. macOS takes ⌘⇧4 first: turn off its screenshot shortcuts in Keyboard Shortcuts › Screenshots (the card has an Open Keyboard Settings button) or record another key. Record screen works only while Labs › Annotate is on.

**Good to know.** A Captures pane appears in the sidebar after History and lists the last 30 days of shots and recordings. Off by default.

**Where.** Labs › Screenshots (`labs.screenshots`); its card has Shortcut (`screenshots.hotkey`), Record screen (`screenshots.record_hotkey`, unbound) and Saves to (`screenshots.folder`).

**Asked as.** "Take screenshots with VibeHex"; "Where do my screenshots go?"; «Как делать скриншоты в VibeHex»; «Куда сохраняются скриншоты».

<!-- sources: src/rune_ui/labs.rs:607-616, 893-905; src/rune_ui/labs/screenshots.rs:11-70; src/screenshots.rs:1-15, 53; src/rune_ui/sidebar.rs:88-91, 143; src/rune_ui/labs.rs:35-40; src/rune_ui/captures.rs:1-3; src/rune_ui/screenshot_thumbnail.rs:1-5; src/app_settings/screenshots.rs:14-46; src/labs.rs:490; src/control/settings.rs:259-276, 598-601 -->
