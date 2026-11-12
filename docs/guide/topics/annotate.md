---
topic: annotate
title: Annotate
keys: [capture.enabled, capture.hotkey, capture.transcriber, capture.folder, capture.media_days, capture.show_overlays_in_recordings]
os: macOS, Windows, Linux (not on every feature)
---

# Annotate

Annotate records your screen while you draw on it and talk over it, then copies the result so you can paste it into a chat or send it to an OpenCode session. It shows a developer or an AI agent exactly what is wrong.

## Is it on?

Annotate is on by default and is built into VibeHex on macOS, Windows and Linux. Look for **Annotate** in the sidebar. Its switch is at the top of the pane; off, VibeHex records nothing and keeps no shortcut.

## Record and draw

The **Shortcut** is Option-Shift-S on a Mac and F9 on Windows and Linux. Change it in the pane.
1. Tap the shortcut to start. Talk as you go.
2. Hold the shortcut and move the pointer to draw. No click is needed, and on macOS and Windows holding Shift draws too. Or click the pen on the small bar, drag, and click it again or press Esc to stop.
3. Tap the shortcut again to stop. The annotation is copied, ready to paste.

Your voice is transcribed with the same model you dictate with. On a Mac, **Transcription › Transcriber** can switch to Apple's recognizer.

## Permissions

On a Mac, Annotate needs **Screen Recording** (the pane has a row for it; macOS also asks at the first annotation), the **Microphone** for your voice, and **Accessibility** for the shortcut. If one is missing, a small helper opens instead of recording. On Linux the desktop's screen-share dialog asks at the first capture. Windows asks nothing.

## Send it

Paste the result anywhere, or click **Send to…** on the capsule to choose an OpenCode session. Sending needs OpenCode to be running.

## Where captures go

**Storage › Annotations folder** is, by default, Scrawl's own folder: `~/Library/Application Support/scrawl/captures` on a Mac, and a `scrawl/captures` folder in the local data folder on Windows and Linux. Press **Reveal** to open it. On a Mac, the folder button picks another; on Windows and Linux that is only possible in the settings file for now. **Keep** is Video for 7 days (default), Video for 30 days, or Forever. The video and audio go first; the annotation itself is removed after 30 or 90 days.

## Limits

Windows has no `hex annotate` command. On Linux, Wayland gives no area selection, and Shift does not draw. On a Mac, if the Scrawl app is running, it keeps the shortcut until you press **Use VibeHex for annotations**.

**Asked as.** "How do I draw on my screen and send it?"; "What is the F9 key for?"; «Как записать экран с комментариями?»; «Какой хоткей у Annotate?»

<!-- sources: build.rs:8-16; src/app_settings.rs:445-516, 558-565; src/app_settings/hotkeys.rs:703-719; src/rune_ui/scrawl.rs:15-130, 200-380, 380-445; src/rune_ui/sidebar.rs:121-133; src/rune_ui/platform.rs:755-790; src/capture_host/permissions.rs:1-30; crates/capture/src/handoff.rs:66-77; docs/features/capture.md:1-160 -->
