---
feature: gnome-meeting-capsules
key: labs.gnome_meeting_capsules
title: Floating meeting capsules on GNOME
group: Meetings
os: Linux
vibehex: false
---

# Floating meeting capsules on GNOME

**What it does.** GNOME on Wayland can't show VibeHex's floating meeting capsules, so a call's recording offer opens the Meetings pane instead. This lab shows them again through XWayland.

**Turn it on when.** You record calls on GNOME on Wayland.

**Leave it off when.** Your desktop already shows the capsules.

**Needs.** Linux, a Wayland desktop without layer-shell, such as GNOME, and XWayland already running; VibeHex never starts it.

**Good to know.** If they still can't open, the Meetings pane opens as before.

**Where.** Labs › Floating meeting capsules on GNOME (`labs.gnome_meeting_capsules`).

**Asked as.** "The recording offer doesn't appear on GNOME"; «На GNOME не появляется предложение записать звонок».

<!-- sources: docs/features/labs.md:168-173; docs/features/meetings.md:178-195; src/rune_ui/labs.rs:661-666; src/control/settings.rs:618-621 -->
