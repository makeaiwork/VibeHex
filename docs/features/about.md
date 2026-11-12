# About and the VibeHex name (every OS)

VibeHex is this fork of Hex. The sidebar opens with its logo and name, and an
About item after Labs says what the fork is and how to reach its developer.

```ts
Sidebar                                         // rune_ui::brand, rune_ui::sidebar
├── Header   Hex's logo (packaging/linux/icons, 128 px, decoded once) and
│            "VibeHex", above the navigation; not a button
└── About    the last item, after Labs (Pane::About, nav-about)
About                                           // rune_ui::about
├── Card     the logo at 56 pt, VibeHex, "Version <version>" as the footer
│            reads it
├── Fork     "VibeHex is a fork of Hex, the open-source dictation app…", then
│            Source code github.com/makeaiwork/hex with Open on GitHub
└── Footer   "Hi! I'm Denis, and I make VibeHex…", Email the developer (a new
             message to zelenpro@gmail.com, subject "VibeHex feedback
             (version <version>)"), Telegram — @DenisKey (t.me/DenisKey)
```

Each link opens in its app through `opencode_catalog::open_url`; a failure
shows as the window's error. Fixtures and tests open nothing.

The name comes from one constant, `app_paths::PRODUCT_NAME` (`brand::NAME` in
the window), wherever the app names itself in a title or a menu:

```ts
VibeHex                                         // app_paths::PRODUCT_NAME
├── Window        its title, on every OS (rune_ui::spec)
├── macOS         app_paths::app_name: the app menu's title, Hide VibeHex and
│                 Quit VibeHex; the status item's name and its menu's Quit
│                 VibeHex. A test build (HEX_TEST_BUILD) stays "Hex Test", as
│                 its bundle is named
├── Windows, Linux tray (desktop/tray_menu.rs): its title, the tooltips
│                 (VibeHex Dictation, …: recording, …), Show VibeHex,
│                 Enable / Disable VibeHex, Quit VibeHex
└── Sidebar       Quit VibeHex (Windows, Linux)
```

Translated strings carry the name in their own text, as rows of VibeHex's
tables (`desktop/i18n/vibehex.rs`, `vibehex/russian.rs`); Hex's rows stay as
they are. Everything else still says Hex: the `hex` command and its replies,
errors and notices, onboarding and permission text, Labs descriptions, the
Linux launcher entry (`Name=Hex`, its Quit Hex action) and the Windows
installer.

Checks: `labs::feedback_mailto_names_the_version_in_the_subject` (the mailto link, shared with Labs › Send feedback); scene `about`
and its light and dark goldens; `the_menu_reads_meeting_then_actions_then_the_app`
(the status item's rows, macOS) and `tray_menu_ids_round_trip_and_show_the_state`
(the tray's tooltip, Windows and Linux). Not yet verified by pressing the links in a
running app.
