---
feature: voice-filter
key: experimental.hex_microphone
title: Voice Filter
group: Sound and interface
os: [macOS, Linux]
vibehex: false
---

# Voice Filter

**What it does.** Adds a virtual microphone called Hex Microphone and a Voice Filter pane. If you choose Hex Microphone as the input in a call app, the app hears your voice through VibeHex, and while you dictate VibeHex changes what the call hears (muffled by default, or muted, quiet, and other voices). So colleagues don't hear you dictating.

**Turn it on when.** You dictate during video calls and don't want the call to hear it.

**Leave it off when.** You never dictate while a call app is listening.

**Needs.** macOS or Linux (not Windows). On a Mac, installing the Hex Microphone driver in the Voice Filter pane, which asks for your administrator password. On Linux, PipeWire.

**Good to know.** While the driver is installed the row stays on and can't be switched off: its text says "Uninstall the driver in Voice Filter to turn it off." An organization's lock can also hold the row off. Off hides the pane. The pane has a Try it check that plays what the call would hear.

**Where.** Labs › Voice Filter (`experimental.hex_microphone`), then the Voice Filter pane in the sidebar.

**Asked as.** "How do I stop the call hearing my dictation?"; "What is Hex Microphone?"; «Как сделать, чтобы собеседники не слышали диктовку»; «Не отключается Voice Filter».

<!-- sources: docs/features/labs.md:42, 106-108, 115, 1373-1383; docs/features/README.md:1123-1156; src/labs.rs (name, settings_key, on_this_os, description); src/rune_ui/labs.rs:196-213; src/app_settings.rs:1690-1697 (managed lock) -->
