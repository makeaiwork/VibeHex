---
feature: gnome-annotate
key: labs.gnome_annotate
title: Annotate on GNOME
group: Annotate
os: Linux
vibehex: false
---

# Annotate on GNOME

**What it does.** GNOME on Wayland can't show Annotate's floating parts, so a capture records with no capsule or ink. This lab shows the capsule, the ink and the picker through XWayland. They take the mouse but never keyboard focus.

**Turn it on when.** You use Annotate on GNOME on Wayland.

**Leave it off when.** Your desktop already shows Annotate's capsule and ink.

**Needs.** Linux, a Wayland desktop without layer-shell, such as GNOME, and XWayland already running; VibeHex never starts it. On other desktops the lab does nothing.

**Good to know.** Esc can't stop drawing there: use the capsule's pen button, or let go of a held Annotate key. A tap of the key stops the recording. VibeHex checks for XWayland each time a part is about to show.

**Where.** Labs › Annotate on GNOME (`labs.gnome_annotate`).

**Asked as.** "Annotate shows nothing on GNOME"; "No capsule or ink when I record on Wayland"; «Annotate не показывает капсулу на GNOME»; «Нет пера и капсулы при записи экрана в Wayland».

<!-- sources: docs/features/labs.md:174-181; docs/features/capture.md:174-186; src/rune_ui/labs.rs:640-672 (row on Linux, subtitle with Esc note); src/linux/annotate_overlays.rs:1-17 (capsule, ink, picker; pointer only; never starts XWayland); src/linux/meeting_capsules.rs:15-40 (Wayland without layer-shell, XWayland running, checked each show); src/labs.rs:476, 1286-1292 (Linux only); src/labs/group.rs:102 (Annotate); src/control/settings.rs:623-626 -->
