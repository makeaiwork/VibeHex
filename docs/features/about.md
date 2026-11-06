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

Checks: `rune_ui::about` (the mailto link and its encoding); scene `about`
and its light and dark goldens. Not yet verified by pressing the links in a
running app.
