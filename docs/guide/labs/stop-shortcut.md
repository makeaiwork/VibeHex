---
feature: stop-shortcut
key: labs.stop_shortcut
title: Stop shortcut
for: [dictation, code]
ask: Finish a dictation by pressing Return instead of your shortcut?
group: Shortcuts
os: macOS
vibehex: true
---

# Stop shortcut

**What it does.** Lets one key start a dictation and another finish it. By
default the finishing key is Return: start with, say, Right ⌥, say your
sentence, press Return, and the text is pasted where you were typing, as if
you had released the shortcut.

**Turn it on when.** You dictate into chat or messages and want Return to
finish and paste in one motion.

**Leave it off when.** You prefer to finish with the main shortcut. Off,
Return is always the app's.

**Needs.** macOS. Nothing else.

**Good to know.** It acts only while a dictation is recording; otherwise Return
works normally. It also works while holding the shortcut's modifier. Shift
plus Return and other keys behave as before. The shortcut itself still
finishes too. VibeHex refuses a key another VibeHex shortcut uses.

**Where.** Labs › Stop shortcut (`labs.stop_shortcut`), marked as added in
VibeHex. The key: General › Stop shortcut (`stop_hotkey`), with a restore
button for Return. That row shows only while the lab is on.

**Asked as.** "Press Enter to finish dictation"; "Stop and paste with
Return"; «Завершить диктовку клавишей Enter»; «Как остановить запись и
вставить текст другой клавишей».

<!-- sources: docs/features/labs.md:93,338-370; src/rune_ui/general.rs:102-112; src/app_settings/hotkeys.rs:722-734; src/app_settings.rs:819,1404-1408; src/labs.rs:341-353 -->
