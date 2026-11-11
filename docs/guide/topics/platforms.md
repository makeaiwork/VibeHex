---
topic: platforms
title: What differs on macOS, Windows, and Linux
keys: [transcription_device, enable_on_launch, show_dock_icon, paste_with_shift, share_dictation_shortcut, meetings.enabled, labs.mouse_shortcuts, labs.keep_on_clipboard, labs.copy_only, labs.tap_or_hold, labs.terminal_paste_shortcut]
os: every OS
---

# What differs on macOS, Windows, and Linux

The window and the speech models are the same everywhere, and so are most
panes. The differences come from what each system allows.

## Where VibeHex runs

- **macOS**: Apple silicon Macs, macOS 15 or newer.
- **Windows**: x64 and ARM64, in beta. An x64 PC needs a 2013 or newer
  processor with AVX2. Its graphics driver should have Vulkan 1.2; without it
  VibeHex still starts and speech runs on the CPU, and General › Advanced ›
  Dictation › Speech engine says so. ARM64 uses the processor.
- **Linux**: x86_64 and ARM64, in beta, on X11 and on Wayland.

## Where it lives and how it listens

- macOS: the menu bar, plus a Dock icon (General › Advanced › Application ›
  Show Dock icon). It listens as soon as setup is done, and the menu bar is its
  control.
- Windows and Linux: the system tray, where Linux needs a tray in your
  desktop. General has a status card with Enable and Disable, and Advanced ›
  Application › Enable on launch.

## Permissions

macOS needs the Microphone and Accessibility permissions before you dictate.
Windows needs only its microphone switch for desktop apps on, and asks only if
it is off. Linux on GNOME or KDE with Wayland has the
desktop ask once to allow pasting. See Getting started.

## Shortcuts

- The defaults differ: Option on a Mac, Ctrl+Space on Windows, Alt+Space on
  Linux (Ctrl+Alt+Space on GNOME and KDE Plasma, which use Alt+Space
  themselves).
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

## Meetings

Labs › Meetings (beta) is listed on macOS, Windows, and Linux. Windows and
Linux record as a beta, with no permission prompts and no calendar. `hex labs
on meetings` works on a Mac only; elsewhere use `hex settings set
meetings.enabled on`. Labs › Live chapters and Labs › Subtitle export are
listed there too, but switch them in the window: `hex labs` and `hex settings`
refuse them off a Mac. See the Meetings article.

## Labs that exist on some systems only

- **Mac only**: Stop when the call ends, Movable meeting capsules, Import
  audio (dev builds), Screenshots, Annotate 15 s replay (dev builds, macOS 27),
  ⌘Q closes Settings (dev builds), Files, Listen to the recording, Smart spacing, Mouse shortcuts, Caps Lock dictation, Also pause
  the browser, Second dictation shortcut, Stop shortcut, Sided modifier chords,
  Copy recent dictations, Hide menu bar icon, Notch indicator, Add to
  vocabulary from a selection, Cleanup progress, and Lower every app. Meetings'
  calendar reminders, the Built in AI cleanup models, and Apple Intelligence
  meeting notes are Mac only too.
- **Mac and Windows**: Keep dictation on clipboard and Copy dictation only.
- **Mac and Linux**: Tap or hold.
- **Linux and Windows**: Skip long pauses, Terminal paste shortcut, and
  Special keys alone (a row on Linux; on Windows turn it on with `hex labs on
  special-keys`).
- **Linux only**: Section key alone, Restore clipboard on GNOME, Transform
  typed text, Floating meeting capsules on GNOME, Annotate on GNOME, and, on
  Wayland, Live typing.
- **Windows only**: Terminal paste, Share dictation shortcut with other apps,
  and OpenCode in WSL.

Voice Filter, with its Hex Microphone, is for dev builds only, on Mac and
Linux. A stable build keeps only an Uninstall for a driver already installed.

**Asked as.** "Does VibeHex work on Windows?"; "Can I record calls on Linux?";
«Есть ли версия для Windows?»; «Почему у меня нет вкладки Meetings?».

<!-- sources: app/Info.plist:36-37; docs/windows.md:34, 54-57; src/windows/vulkan.rs:1-40; src/rune_ui/general.rs:1240-1285; src/rune_ui/platform.rs:70-130, 809-852; src/labs.rs:279-284, 460-495; src/rune_ui/labs.rs:422-505, 520-530, 560-575, 785-880; src/rune_ui/fun.rs:285-300, 464-490; src/labs/cli.rs:104-113; src/control/settings.rs:944-956; docs/features/meetings.md:128-215; docs/features/labs.md:73, 607-640; src/rune_ui/settings.rs:221; src/app_settings/hotkeys.rs:645-655; src/transcription/cpu_floor.rs:11; src/dictation/cleanup_server/engine.rs; src/rune_ui/output/ai_cleanup.rs:188-195 -->
