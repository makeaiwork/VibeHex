---
feature: opencode-in-wsl
key: labs.opencode_in_wsl
title: OpenCode in WSL
group: Output
os: Windows
vibehex: false
---

# OpenCode in WSL

**What it does.** The AI rewrite of dictations can run through OpenCode, a separate program. VibeHex normally finds it only in Windows. With this on, when none answers there, it also uses OpenCode running in your default WSL distribution.

**Turn it on when.** You run OpenCode inside WSL.

**Leave it off when.** OpenCode runs in Windows; that one wins anyway.

**Needs.** Windows, WSL, and OpenCode running in its default distribution.

**Good to know.** Looking for it can start WSL. Off, VibeHex never looks into WSL.

**Where.** Labs › OpenCode in WSL (`labs.opencode_in_wsl`).

**Asked as.** "VibeHex can't see OpenCode in WSL"; «Как подключить OpenCode из WSL».

<!-- sources: docs/features/labs.md:191-194, 2518-2600; src/rune_ui/labs.rs:866-872; src/labs.rs:1344-1350; src/control/settings.rs:446-450 -->
