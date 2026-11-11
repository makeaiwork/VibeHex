---
feature: terminal-paste
key: paste_with_shift
title: Terminal paste
for: [code]
ask: Make dictations paste into terminals that ignore Ctrl+V?
group: Output
os: Windows
vibehex: false
---

# Terminal paste

**What it does.** Some apps, terminals especially, ignore the Ctrl+V that
VibeHex sends to paste a dictation, so nothing appears. With Terminal paste
on, VibeHex presses Ctrl+Shift+V instead.

**Turn it on when.** A dictation does not appear in a terminal or another
app, but pasting by hand with Ctrl+Shift+V works there.

**Leave it off when.** Your apps paste normally. Ctrl+Shift+V is not "paste"
everywhere: some apps use it for "paste without formatting" or something else.

**Needs.** Windows. Nothing else.

**Good to know.** It applies to every app, not just terminals, and to the
next paste without a restart. On Linux the same switch is not in Labs: it is
a row called Terminal paste in the Output section of General.

**Where.** Labs › Terminal paste (`paste_with_shift`).

**Asked as.** "Dictation doesn't paste into the terminal"; "Paste with
Ctrl+Shift+V"; «Текст не вставляется в терминал»; «Диктовка не вставляется,
а Ctrl+Shift+V работает».

<!-- sources: docs/features/labs.md:97, docs/features/labs.md:1397-1399; docs/features/dictation.md:1131-1141; src/labs.rs:294; src/rune_ui/labs.rs:530-536; src/rune_ui/general.rs:1033-1048 (Linux row in Output section of General); src/rune_ui/platform.rs terminal_paste; src/windows/paste.rs -->
