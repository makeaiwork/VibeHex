---
feature: mouse-shortcuts
key: labs.mouse_shortcuts
title: Mouse shortcuts
group: Shortcuts
os: macOS
vibehex: false
---

# Mouse shortcuts

**What it does.** Lets you use a mouse button, such as a side button, as a
dictation shortcut, alone or with a modifier. Click Change shortcut in a
shortcut row, then press the button.

**Turn it on when.** Your mouse has extra buttons and you would rather
dictate with one of them than with the keyboard.

**Leave it off when.** You only use the keyboard. Off, the recorder ignores
mouse buttons and clicks reach the window as usual.

**Needs.** macOS. Windows and Linux (X11) already accept mouse buttons
without this switch.

**Good to know.** The left button (Mouse 1) needs a modifier, because alone
it would swallow every click. The right and middle buttons are Mouse 2 and
3; side buttons are Mouse 4 and up. A press that VibeHex uses does not reach
the app in front. Turning the switch off puts a mouse-button shortcut back to
its default, and General says so once.

**Where.** Labs › Mouse shortcuts (`labs.mouse_shortcuts`). Used in General's
shortcut rows.

**Asked as.** "Can I dictate with a mouse button?"; "Mouse side button as the
shortcut"; «Назначить кнопку мыши на диктовку»; «Не записывается кнопка
мыши в шорткат».

<!-- sources: docs/features/labs.md:49,147-152; docs/features/dictation.md:23-50; src/rune_ui/macos_host.rs:196-206; src/app_settings/shortcut_repair.rs:103-115; src/control/settings.rs:1068-1076,1411; src/rune_ui/labs.rs:323-327 -->
