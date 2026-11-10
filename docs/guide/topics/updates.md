---
topic: updates
title: Updates, About, and feedback
keys: [updates.channel]
os: every OS
---

# Updates, About, and feedback

VibeHex updates itself, so you do not download new versions by hand. This page
covers how an update arrives, how to see your version, and how to reach the
developer.

## How an update arrives

VibeHex looks for a new version ten seconds after it starts and then every
hour. The news comes from the fork's releases on GitHub. To check right now,
press "Check for updates" at the bottom of the sidebar.

When there is a newer version, VibeHex downloads it in the background and
checks that the file is intact and properly signed. A window then says "Hex
<version> is ready", lists what changed, and offers **Restart to update** or
**Not now**. Not now closes it; the sidebar keeps a Restart to update button.
If you do nothing, a Mac installs the update when you quit.

## When VibeHex cannot update itself

The bottom of the sidebar says why:

- "Download update": VibeHex sits where it cannot replace itself (on a Mac, a
  folder you cannot write to or an app opened straight from a download; on
  Linux, a plain downloaded file). The button opens the download page.
- "Updated by your package manager": your package manager, on Linux, does it,
  and the sidebar shows its command.
- "Managed by your organization": an administrator turned updates off.
- A build made from source or locally does not check.

## Update channel

In General › Advanced › Application › Update channel you can choose Stable or
Dev, for installs that update themselves. Dev is for newer builds and checks
more often. Choosing another channel asks you to confirm with a "Switch to"
button in the sidebar. If unsure, stay on Stable.

## Version and About

The sidebar's footer shows "Version" and its number. About, the last item after
Labs, shows the same, says VibeHex is a fork of Hex, and has Open on GitHub for
the source code.

## Send feedback

Labs has a Send feedback button, and About has "Email the developer". Both open
a new message to the developer with the version in its subject, so your mail
says which build you use. About also has the developer's Telegram, @DenisKey.

**Asked as.** "How do I update VibeHex?"; "Which version do I have?"; «Как
обновить приложение?»; «Куда написать разработчику?»; «Как проверить
обновления?».

<!-- sources: src/updater.rs:1-130,440-480; src/updater/macos.rs:1-12; src/rune_ui/sidebar.rs:485-640; src/rune_ui/sidebar_update.rs; src/rune_ui/update_sheet.rs:130-180; src/rune_ui/general.rs:1150-1185; docs/vibehex-releases.md:12-30; docs/features/about.md; src/rune_ui/about.rs; src/rune_ui/labs.rs:773-796; src/labs.rs:25-45 -->
