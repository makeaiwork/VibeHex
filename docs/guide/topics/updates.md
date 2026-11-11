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
hour. The news comes from the fork's releases on GitHub
(github.com/makeaiwork/VibeHex). To check right now, press "Check for updates" at
the bottom of the sidebar; it then says "VibeHex is up to date" or "Couldn't
check for updates".

When there is a newer version, VibeHex downloads it in the background and
checks that the file is intact and properly signed. A window then says "Hex
<version> is ready". It lists what changed in that release and in every one
you skipped since yours, and offers **Restart to update** or **Not now**. Not
now closes it; the sidebar keeps a Restart to update button, with a "What's
new" link that opens the release notes page on GitHub. If you do nothing, a
Mac installs the update when you quit.

## When VibeHex cannot update itself

The bottom of the sidebar says why:

- "Download update": VibeHex sits where it cannot replace itself, so the button
  opens a download page instead. On a Mac that is a folder you cannot write
  to or an app opened straight from a download; on Linux, a plain downloaded
  file; on Windows, a copy the installer did not install, one placed outside your
  user folder, or one whose install record is from another installer. On
  Windows a line under the button says which reason applies.
- "Updated by your package manager": your package manager, on Linux, does it,
  and the sidebar shows its command once a newer version exists.
- "Managed by your organization": an administrator turned updates off.
- A build made from source or locally does not check: the sidebar says it is a
  local build with updates off, or "Updated from your source checkout".

## Update channel

In General › Advanced › Application › Update channel you can choose Stable or
Dev, whenever VibeHex checks for updates. Dev is for newer builds and checks
more often (every ten minutes, and when you open the window). Choosing
another channel asks you to confirm with a "Switch to" button in the sidebar,
such as "Switch to Stable" followed by the version. If unsure, stay on Stable.

## Release notes

Each release's notes are on its page at github.com/makeaiwork/VibeHex/releases.
"What's new" under Restart to update opens the page for the staged release.

You can also ask Assistant "What's new?": it knows every VibeHex release,
tells what changed since the version you had before, and offers what suits
your settings, each as a card you decide on. For two weeks after an update,
a new Assistant conversation starts with "Updated to <version>" to ask it.

## Version and About

The sidebar's footer shows "Version" and its number. About, the last item after
Labs, shows the same, says VibeHex is a fork of Hex, and has Open on GitHub for
the source code.

## Send feedback

Labs has a Send feedback button, and About has "Email the developer". Both open
a new message to the developer with the version in its subject, so your mail
says which build you use. About also has the developer's Telegram, @DenisKey.

**Asked as.** "How do I update VibeHex?"; "Which version do I have?"; «Как
обновить приложение?»; «Что нового в последнем релизе?»; «Куда написать разработчику?»; «Как проверить
обновления?».

<!-- sources: src/updater.rs:1-60,62-135,240-300,640-725; src/updater/macos.rs:1-12; src/updater/windows.rs:50-92; src/updater/notes.rs:1-30,106-113; src/rune_ui/sidebar.rs:485-640; src/rune_ui/sidebar_update.rs:1-160; src/rune_ui/update_sheet.rs:1-30,120-200; src/rune_ui/general.rs:1111-1145; src/rune_ui/about.rs:40-125; docs/vibehex-releases.md:12-30; src/assistant/releases/mod.rs; src/rune_ui/assistant/whats_new.rs -->
