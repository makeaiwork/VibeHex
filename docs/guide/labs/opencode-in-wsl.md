---
feature: opencode-in-wsl
key: labs.opencode_in_wsl
title: OpenCode in WSL
for: []
ask: Use OpenCode running inside WSL when Windows has none?
group: Output
os: Windows
vibehex: false
---

# OpenCode in WSL

**What it does.** Anything in VibeHex that uses OpenCode, a separate app, looks for it only in Windows. With this on, when no Windows OpenCode answers, VibeHex also uses one running in your default WSL distribution. A Windows OpenCode that answers always wins.

**Turn it on when.** You run OpenCode inside WSL and see "OpenCode isn't running".

**Leave it off when.** OpenCode runs in Windows, or you don't use it.

**Needs.** Windows, WSL 2, and OpenCode 2 already running in the default distribution.

**Good to know.** It covers the AI cleanup rewrite (Output › AI cleanup, Runs on set to OpenCode), meeting titles and summaries (Meetings › Settings › After a meeting › Summaries set to OpenCode) and the model list under it, Annotate's Send to…, and the Assistant's OpenCode check. Looking for OpenCode can start WSL, and a slow start may first read as "isn't running"; try again. Off, VibeHex never looks into WSL.

**Where.** Labs › OpenCode in WSL (`labs.opencode_in_wsl`).

**Asked as.** "VibeHex can't see OpenCode in WSL"; "Use OpenCode from Ubuntu on Windows"; «Как подключить OpenCode из WSL»; «Пишет, что OpenCode не запущен, а он работает в WSL».

<!-- sources: src/rune_ui/labs.rs:869-875 (subtitle names Modes, which VibeHex never shows: src/rune_ui/sidebar.rs Pane::Modes); src/desktop/opencode_service.rs:1-10, 70-130 (find, find_cancellable, running, with_wsl, prefer_wsl); src/desktop/opencode_service/wsl.rs:1-40; src/dictation/processor.rs:762, 1042; src/meeting/generator.rs:190, 231; src/rune_ui/platform_settings.rs:260-380; src/capture_host/routing/opencode.rs:1-70; src/rune_ui/assistant/connect.rs:100-105; src/rune_ui/output/ai_cleanup.rs:195; src/labs.rs:469-471, 1348; src/control/settings.rs:446-450 -->
