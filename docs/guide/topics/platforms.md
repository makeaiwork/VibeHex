---
topic: platforms
title: What differs on macOS, Windows, and Linux
keys: [transcription_device, enable_on_launch, show_dock_icon, paste_with_shift, share_dictation_shortcut, labs.mouse_shortcuts]
os: every OS
---

# What differs on macOS, Windows, and Linux

The window, the panes, and the speech models are the same everywhere. The
differences come from what each system allows.

## Where VibeHex runs

- **macOS**: Apple silicon Macs, macOS 15 or newer.
- **Windows**: x64 and ARM64, in beta. An x64 PC needs a 2013 or newer
  processor with AVX2, and a graphics driver with Vulkan. ARM64 uses the
  processor.
- **Linux**: x86_64 and ARM64, in beta, on X11 and on Wayland.

## Where it lives and how it listens

- macOS: the menu bar, plus a Dock icon (General › Advanced › Application ›
  Show Dock icon). It listens as soon as setup is done, and the menu bar is its
  control.
- Windows and Linux: the system tray, where Linux needs a tray in your
  desktop. General has a status card with Enable and Disable, and Advanced ›
  Application › Enable on launch.

## Permissions

Only macOS gates dictation behind Microphone and Accessibility. Windows asks
only if its microphone switch is off. Linux on GNOME or KDE with Wayland has the
desktop ask once to allow pasting. See Getting started.

## Shortcuts

- The defaults differ: Option, Ctrl+Space, Alt+Space.
- macOS and Windows can require the left or right modifier key; Linux cannot.
  Linux needs a key besides modifiers, such as Space.
- Mouse buttons work as a shortcut on Windows and Linux X11, and on a Mac
  after turning on Labs › Mouse shortcuts. Wayland has none.
- Windows can pass the shortcut on to other apps: Labs › Share dictation
  shortcut with other apps.
- On Wayland, the desktop may need to grant the shortcut, or you bind the `hex`
  commands yourself.

## Speech and pasting

On Windows and Linux, General › Advanced › Dictation › Speech engine picks
Auto, GPU, or CPU; a Mac chooses for itself. Terminals take Ctrl+Shift+V:
General › Advanced › Output › Terminal paste on Linux, Labs › Terminal paste
on Windows, and nothing to set on a Mac.

## Mac only

Meetings (recording calls, with calendar reminders), Files, the Built in AI
cleanup models, Apple Intelligence meeting notes, and the Hex Microphone
driver behind Voice Filter are for Mac. So are these Labs rows: Live
chapters, Subtitle export, Stop when the call ends, Movable meeting capsules,
Listen to the recording, Smart spacing, Copy
dictation only, Keep dictation on clipboard, Copy recent dictations, Mouse
shortcuts, Also pause the browser, Second dictation shortcut, Stop shortcut,
Sided modifier chords, Tap or hold, Hide menu bar icon, Add to vocabulary from
a selection, and Cleanup progress. Voice Filter also exists on Linux, where
it creates a virtual microphone while it is on, and not on Windows. Labs › Skip
long pauses and Labs › Special keys alone are for Windows and Linux.

**Asked as.** "Does VibeHex work on Windows?"; "Can I record calls on Linux?";
«Есть ли версия для Windows?»; «Почему у меня нет вкладки Meetings?».

<!-- sources: src/rune_ui/platform.rs:809-852; src/labs.rs:on_this_os; docs/features/README.md:1457-1796,225-240; docs/windows.md; src/transcription/cpu_floor.rs; src/app_settings/hotkeys.rs:619-636; AGENTS.md:687-724; src/rune_ui/general.rs:880-910,1030-1100 -->
