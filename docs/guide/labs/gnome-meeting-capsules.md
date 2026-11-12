---
feature: gnome-meeting-capsules
key: labs.gnome_meeting_capsules
title: Floating meeting capsules on GNOME
for: []
ask: Show meeting capsules over your call on GNOME Wayland?
group: Meetings
os: Linux
vibehex: false
---

# Floating meeting capsules on GNOME

**What it does.** GNOME on Wayland can't show VibeHex's floating meeting capsules, so when a call is detected the recording offer opens the Meetings pane instead, with a note. This lab shows the offer and the recording indicator over your call again, through XWayland.

**Turn it on when.** You record calls on GNOME on Wayland and want the capsules over the call.

**Leave it off when.** Your desktop already shows the capsules, or you are happy to start recordings from the Meetings pane.

**Needs.** Linux, a Wayland desktop without layer-shell support, such as GNOME, and XWayland already running for your account. VibeHex never starts it.

**Good to know.** The capsules take clicks on their buttons but never keyboard focus. If they still can't open, the Meetings pane opens as before and its Start recording button records. While the lab is off, that note names the lab. It is off by default.

**Where.** Labs › Floating meeting capsules on GNOME (`labs.gnome_meeting_capsules`), in the Meetings group.

**Asked as.** "The recording offer doesn't appear on GNOME"; "Meeting capsule doesn't float over my call"; «На GNOME не появляется предложение записать звонок»; «Капсула записи встречи не показывается поверх звонка».

<!-- sources: src/labs.rs:1277-1283, 473-477; src/linux/meeting_capsules.rs:1-40; src/meeting/runtime.rs:142-162; src/rune_ui/labs.rs:661-666; src/rune_ui/meetings.rs:534; src/labs/group.rs:97; src/labs/description.rs:135-137; src/control/settings.rs:617-621; docs/features/meetings.md:178-195 -->
