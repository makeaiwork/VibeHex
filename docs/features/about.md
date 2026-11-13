# About and the VibeHex name (every OS)

VibeHex is this fork of Hex. The sidebar opens with its logo and name, and an
About item after Labs holds the version and the update check, says what the
fork is, asks for a star on GitHub, and says how to reach its developer.

```ts
Sidebar                                         // rune_ui::brand, rune_ui::sidebar
├── Header   Hex's logo (packaging/linux/icons, 128 px, decoded once) and
│            "VibeHex", above the navigation; not a button
├── About    the last item, after Labs (Pane::About, nav-about)
└── Bottom   an update to take (Restart to update, Download update, Switch
             to …) with its dot, only while there is one, then Assistant.
             No version and no check here. Assistant's face sits as far
             above the bottom edge as the logo below the top (41 pt on
             macOS, `sidebar::LOGO_INSET`), folded or not; Linux ends with
             Quit VibeHex, whose row mirrors the brand row
About                                           // rune_ui::about
├── Card     the logo at 56 pt, VibeHex, "Version <version>" (a local or
│            test build adds its commit and a Local / Test badge), then
│            (about/update.rs):
│            ├── Check for updates (a small button) and, after a check the
│            │   person asked for, "Checking for updates…" (the button
│            │   disabled), "VibeHex is up to date" or "Couldn't check for
│            │   updates" beside it; the hourly check alone adds no words
│            ├── An update to take -> the sidebar's button with its dot
│            ├── No check to run -> one quiet line instead: Managed by your
│            │   organization, Downloading update, Local build, updates off,
│            │   Updated from your source checkout, Updated by your package
│            │   manager, or the package manager's command
│            └── Release notes (Labs › Release notes link, a release build)
├── Fork     "VibeHex is a fork of Hex, the open-source dictation app…",
│            "If VibeHex is useful to you, a star on GitHub helps others
│            find it.", then Source code github.com/makeaiwork/VibeHex with
│            Star on GitHub (a star icon; it opens the repository, where
│            starring happens)
└── Footer   "Hi! I'm Denis, and I make VibeHex…", Email the developer (a new
             message to zelenpro@gmail.com, subject "VibeHex feedback
             (version <version>)"), Telegram — @DenisKey (t.me/DenisKey)
```

Each link opens in its app through `opencode_catalog::open_url`; a failure
shows as the window's error. Fixtures and tests open nothing. Check for
Updates in the macOS menu bar or app menu opens the window on About
(`Destination::About`), which then says how the check went (#1120).

The app is called VibeHex wherever the user reads its name. One constant
holds it, `app_paths::PRODUCT_NAME` (`brand::NAME` in the window), for the
places that show the bare name; a sentence carries it in its own text:

```ts
VibeHex                                         // app_paths::PRODUCT_NAME
├── Window        its title, on every OS (rune_ui::spec); the onboarding tour's
│                 mock rows and certificate
├── macOS         app_paths::app_name: the app menu's title, Hide VibeHex and
│                 Quit VibeHex; the status item's name and its menu's Quit
│                 VibeHex. A test build (HEX_TEST_BUILD) stays "Hex Test", as
│                 its bundle is named
├── Windows, Linux tray (desktop/tray_menu.rs): its title, the tooltips
│                 (VibeHex Dictation, …: recording, …), Show VibeHex,
│                 Enable / Disable VibeHex, Quit VibeHex
├── Sidebar       Quit VibeHex (Windows, Linux)
└── Sentences     every translated window string, in all nine languages:
                  desktop/i18n/vibehex/naming.rs holds Hex's strings said
                  with VibeHex's name, keyed by the new English text; Hex's
                  own rows stay in its tables, unused, for the next merge.
                  Untranslated text says it too: failures that reach the HUD
                  or the window, settings.json errors, Activity's lines, the
                  floating windows' accessibility names, meeting-import
                  errors (meeting_import.swift), the CPU-too-old dialog, the
                  GNOME custom shortcuts' names, the reasons the OS shows for
                  keeping awake, and macOS's permission prompts
                  (app/Info.plist, and the camera text in scripts/build-app.sh)
```

"Hex" stays where it is another thing's name or Hex itself:

- the `hex` command, its help and replies, `hex settings` and `hex labs`
  descriptions, and the local API's replies;
- Hex Microphone (the audio device and its driver), the "Hex Meetings" folder
  and the MCP server's name, "Hex Screenshot …" and "Hex Recording …" files;
- the Linux launcher and autostart entries (`Name=Hex`, its Quit Hex action),
  the Windows installer, the Start Menu shortcut and the startup registry
  value, the keychain service ("Hex AI cleanup"), bundle ids, settings keys,
  the discovery file, log lines;
- the credit in About ("VibeHex is a fork of Hex…"), the three Windows
  messages that send to Hex's installer at anoma.ly/hex (VibeHex publishes no
  Windows installer to name instead), and the model picker's benchmark source
  ("Hex's run of the transcribe.cpp WER harness": Hex measured it).

A new string from Hex that says "Hex" in the window fails
`no_translated_string_calls_the_app_hex` or
`no_source_writes_out_a_string_that_calls_the_app_hex`
(`desktop/i18n/vibehex/naming.rs`) until it says VibeHex and has its rows;
the kept names and strings are that file's `KEPT_NAMES` and `KEPT_STRINGS`.

Checks: `labs::feedback_mailto_names_the_version_in_the_subject` (the mailto link, shared with Labs › Send feedback); scenes `about`
and `about-up-to-date` and their light and dark goldens;
`an_asked_update_check_says_how_it_went`,
`release_notes_link_shows_only_with_its_lab`,
`the_sidebar_bottom_mirrors_the_logo`,
`an_update_is_a_secondary_button_with_a_dot_and_its_own_label`; `the_menu_reads_meeting_then_actions_then_the_app`
(the status item's rows, macOS) and `tray_menu_ids_round_trip_and_show_the_state`
(the tray's tooltip, Windows and Linux). Not yet verified by pressing the links in a
running app.
