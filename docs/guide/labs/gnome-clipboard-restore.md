---
feature: gnome-clipboard-restore
key: labs.gnome_clipboard_restore
title: Restore clipboard on GNOME
group: Output
os: Linux
vibehex: false
---

# Restore clipboard on GNOME

**What it does.** VibeHex pastes through the clipboard, then puts back what you had copied. GNOME on Wayland doesn't allow that, so your copied text is lost. This lab restores it through XWayland.

**Turn it on when.** You use GNOME on Wayland and want your clipboard back.

**Leave it off when.** Your desktop already restores the clipboard.

**Needs.** Linux, and XWayland already running for your account. VibeHex never starts it.

**Good to know.** While the lab is off on such a desktop, General › Output shows a note pointing here.

**Where.** Labs › Restore clipboard on GNOME (`labs.gnome_clipboard_restore`).

**Asked as.** "My clipboard is lost after dictating on GNOME"; «После диктовки пропадает скопированное».

<!-- sources: docs/features/labs.md:148-149; docs/features/dictation.md:547-562; docs/features/README.md:1877; src/rune_ui/labs.rs:650-655; src/rune_ui/general.rs:978-995 -->
