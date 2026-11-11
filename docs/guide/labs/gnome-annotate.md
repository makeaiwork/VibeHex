---
feature: gnome-annotate
key: labs.gnome_annotate
title: Annotate on GNOME
group: Annotate
os: Linux
vibehex: false
---

# Annotate on GNOME

**What it does.** GNOME on Wayland can't show Annotate's floating parts, so a capture records with no capsule or ink. This lab shows them through XWayland.

**Turn it on when.** You use Annotate on GNOME on Wayland.

**Leave it off when.** Your desktop already shows Annotate's capsule and ink.

**Needs.** Linux, a Wayland desktop without layer-shell, such as GNOME, and XWayland already running; VibeHex never starts it.

**Good to know.** Esc can't stop drawing there: use the capsule's pen button, or let go of a held Annotate key. A tap of the key stops the recording.

**Where.** Labs › Annotate on GNOME (`labs.gnome_annotate`).

**Asked as.** "Annotate shows nothing on GNOME"; «Annotate не показывает капсулу на GNOME».

<!-- sources: docs/features/labs.md:174-181; docs/features/capture.md:174-186; src/rune_ui/labs.rs:667-672; src/control/settings.rs:623-626 -->
