---
feature: gnome-clipboard-restore
key: labs.gnome_clipboard_restore
title: Restore clipboard on GNOME
group: Output
os: Linux
vibehex: false
---

# Restore clipboard on GNOME

**What it does.** VibeHex pastes dictation through the clipboard, then puts back what you had copied. A Wayland desktop without clipboard-control support, such as GNOME, doesn't allow that, so your copied text is lost after each paste. This lab restores it through XWayland.

**Turn it on when.** You use GNOME on Wayland and want your clipboard back after dictating.

**Leave it off when.** Your desktop already restores the clipboard. On desktops that support it, the lab changes nothing.

**Needs.** Linux, a Wayland desktop without clipboard-control support, and XWayland already running for your account. VibeHex never starts XWayland; without it, nothing is restored.

**Good to know.** While the lab is off on such a desktop, General › Advanced › Output shows a note that points here. The lab is off by default.

**Where.** Labs › Restore clipboard on GNOME (`labs.gnome_clipboard_restore`), in the Output group.

**Asked as.** "My clipboard is lost after dictating on GNOME"; "Dictation overwrites what I copied"; «После диктовки пропадает скопированное»; «Буфер обмена не восстанавливается после вставки».

<!-- sources: src/labs.rs:1262-1268, 473-477; src/linux/clipboard.rs:64-80, 140-147, 150-167, 169-181; src/rune_ui/labs.rs:650-655; src/rune_ui/general.rs:982-996; src/rune_ui/general/search.rs:57-86; src/labs/group.rs:79; src/control/settings.rs:607-611; docs/features/labs.md:148-149 -->
