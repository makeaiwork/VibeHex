---
feature: live-text
key: labs.live_text
title: Live text
group: History and recording
os: every OS
vibehex: false
---

# Live text

**What it does.** While you hold the shortcut and talk, VibeHex shows the words it has heard so far as one line just beside the recording HUD. You see at once whether it understood you, instead of waiting for the final text.

**Turn it on when.** You like feedback while speaking, or dictate long text and want to catch a misheard word early.

**Leave it off when.** The extra line distracts you. The shown words are provisional and can change; the pasted text is the final result.

**Needs.** A speech model that can stream, running on the computer's graphics chip. With other models, or Apple Speech, no line appears.

**Good to know.** The line can't be clicked and disappears when the recording ends. Off, VibeHex doesn't do the extra work at all. It applies at the next dictation.

**Where.** Labs › Live text (`labs.live_text`).

**Asked as.** "Show the words while I'm speaking"; "Live transcription next to the HUD"; «Показывать текст прямо во время диктовки»; «Как включить живой текст».

<!-- sources: docs/features/labs.md:44, 1270-1284; docs/features/dictation.md:687-712; src/rune_ui/labs.rs:222-227 -->
