---
topic: dictation-shortcuts
title: Dictation shortcut, HUD, and pasting
keys: [dictation_hotkey, shortcut_mode, double_tap_lock, double_tap_only, cancel_hotkey, paste_last_hotkey, indicator_position, labs.keep_on_clipboard, labs.copy_only, labs.terminal_paste_shortcut, terminal_paste_hotkey, labs.caps_lock_dictation]
os: every OS
---

# Dictation shortcut, HUD, and pasting

You press one shortcut, speak, and VibeHex types the result where your cursor
is. This page covers the shortcut, its modes, cancelling, the indicator,
pasting, and the language.

## Change the shortcut

In General › Shortcut press Change shortcut, then the keys you want (Esc
cancels). The default is Option on a Mac, Ctrl+Space on Windows, and Alt+Space
on Linux (Ctrl+Alt+Space on GNOME and KDE). On Linux Wayland without shortcut
support there is no recorder: bind the `hex dictation` commands in your
desktop's keyboard settings.

## Choose how it works

General › Shortcut mode:

- **Hold or double-tap** (default): hold, speak, let go. Or tap twice quickly
  to record hands-free, then press once to finish.
- **Hold**: only hold and release.
- **Tap**: press to start, press again to finish and paste.
- **Double-tap**: only the double-tap; the shortcut must include a key, such
  as Option+Space, not a lone modifier.

A press shorter than about a third of a second is thrown away as an accident.
With a lone modifier such as Option, pressing any other key while it is held
drops the dictation, so your normal keyboard shortcut still works.

## Cancel a dictation

Only Tap mode can cancel. A Cancel row appears under Shortcut mode, Escape by
default; it discards the recording, pasting and saving nothing. In the other
modes Escape goes to your app and every dictation is pasted. The exception is
Labs › Caps Lock dictation (Mac): Escape cancels it in any shortcut mode.

## The indicator

While you dictate, a small indicator at the top or bottom of the screen shows
your voice's level, then that it is transcribing. It never takes focus. Choose
Top, Bottom, or Off in General › Advanced › Output › Dictation HUD.

## Paste it again

General › Advanced › Output › Paste last dictation holds the shortcut that
pastes the last finished dictation again (Option+Shift+V on a Mac, Ctrl+Alt+V
elsewhere). The menu bar or tray has the same item.

## How pasting works

VibeHex puts the text on the clipboard, sends the paste keystroke, and about
half a second later restores what you had copied, unless you copied something
new meanwhile. On Windows the dictation stays out of Win+V history. Terminals
need Ctrl+Shift+V: General › Advanced › Output › Terminal paste on Linux, Labs ›
Terminal paste on Windows. On a Mac or Windows, Labs › Keep dictation on clipboard
leaves the text there (on Windows it then shows in Win+V history), and Labs ›
Copy dictation only copies without pasting.

## A separate shortcut for terminals

On Linux and Windows, turn on Labs › Terminal paste shortcut. General ›
Shortcut then shows a Terminal paste shortcut row, empty until you record a
key. It dictates like the main shortcut but always pastes with Ctrl+Shift+V,
so terminals and ordinary apps each get a key. On Linux it needs X11 or a
desktop with the shortcuts portal.

## Language

General › Language lists the languages of the chosen speech model; a model with
one language has no Language row. Some models detect the language (Auto).
Choosing a model that lacks your language switches to English when the model
has it, else to its first language, and says so under Language; pick the model
first.

**Asked as.** "How do I change the dictation shortcut?"; "Cancel a dictation";
«Как сменить горячую клавишу?»; «Как отменить диктовку?»; «Как говорить без удержания кнопки?»; «Вставить
последнюю диктовку ещё раз».

<!-- sources: docs/features/dictation.md:15-30,444-568,1143-1182; docs/features/labs.md:607-640; docs/guide/labs/terminal-paste-shortcut.md; src/rune_ui/labs.rs:520-535,860-882; src/rune_ui/general.rs:44-95,955-975,350-376; src/transcription/models.rs:309-326; src/rune_ui/picker.rs:375-430; src/recognition.rs:306-315; src/labs.rs:478-495; src/windows/paste.rs:157-200; docs/features/README.md:625-707; src/rune_ui/general.rs:60-130,920-1030; src/rune_ui/shortcut_mode.rs; src/app_settings/hotkeys.rs:619-751; src/windows/paste.rs; src/linux/paste.rs; src/paste.rs:362-390; src/macos_rune/status_item.rs:242 -->
