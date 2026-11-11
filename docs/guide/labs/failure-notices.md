---
feature: failure-notices
key: labs.failure_notices
title: Failure notices
group: Dictation
os: every OS
vibehex: false
---

# Failure notices

**What it does.** A failed dictation flashes the HUD and plays the error tone, but doesn't say why. With this on, the reason shows as one line beside the HUD for about six seconds.

**Turn it on when.** Dictations sometimes fail and you want to know why.

**Leave it off when.** The error tone is enough for you.

**Needs.** Nothing else. It works without Live text.

**Good to know.** A new dictation hides the notice at once.

**Where.** Labs › Failure notices (`labs.failure_notices`).

**Asked as.** "Why did my dictation fail?"; «Почему диктовка не сработала».

<!-- sources: docs/features/labs.md:49; docs/features/dictation.md:1595-1605, 1639-1652; src/rune_ui/labs.rs:400-405, 1017; src/rune_ui/live_text.rs:205-300; src/labs.rs (on_this_os, audience) -->
