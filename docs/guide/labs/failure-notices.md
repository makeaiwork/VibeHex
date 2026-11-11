---
feature: failure-notices
key: labs.failure_notices
title: Failure notices
group: Dictation
os: every OS
vibehex: false
---

# Failure notices

**What it does.** A failed dictation flashes the HUD and plays the error tone, but doesn't say why. With this on, the reason shows as one line beside the HUD for about six seconds. A closed laptop lid or a missing microphone gets a plain "what to do" message; other failures show their own wording.

**Turn it on when.** Dictations sometimes fail and you want to know why.

**Leave it off when.** The error tone is enough for you.

**Needs.** Nothing else. It works on every OS and without Live text.

**Good to know.** The line can't be clicked and shows in the same place as Live text, replacing any live words until it leaves. A new dictation hides it at once, and a background job's failure that arrives while a newer dictation records isn't shown. Turning the lab off hides it too. Failures with no message show nothing.

**Where.** Labs › Failure notices (`labs.failure_notices`).

**Asked as.** "Why did my dictation fail?"; "Show the error reason next to the HUD"; «Почему диктовка не сработала»; «Что за ошибка при диктовке».

<!-- sources: docs/features/labs.md:49; docs/features/dictation.md:1595-1605, 1639-1652; src/rune_ui/labs.rs:400-405, 1017 (dismissed when off); src/rune_ui/live_text.rs:33 (six seconds), 89-98, 199-210 (shares Live text plate), 270-300 (show_failure), 303-311 (new capture dismisses); src/rune_hud.rs:179-189 (only a failure with a message; skipped while a newer capture records); src/desktop/activity.rs:100-111 (lid closed, no microphone); src/labs.rs:509 (on_this_os every OS); src/labs/group.rs:55 (Dictation); src/control/settings.rs:351 -->
