# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`AGENTS.md` is the authoritative guide (module map, invariants, release rules). Claude Code loads it as
project instructions; if it is not in context, read it before non-trivial work. This file is the
quick orientation and does not override it.

## Workflow

- Branch from `dev`, open a PR to `dev`, merge it yourself once it compiles locally (don't wait on CI).
  There is no `main`; never commit to `production`. `.github/` changes need Dax.
- Release freeze: publish no release on any OS and don't move `production` (see AGENTS.md › Current Work).
- New or user-requested features land in Labs, off by default: a `labs::Feature`, a Labs row, and a
  `hex settings` key, gated where the work starts, not only in the UI ([docs/features/labs.md](docs/features/labs.md)).
- Before touching a user-facing feature, read [docs/features/README.md](docs/features/README.md) and the
  relevant feature file, and update that map in the same change when behaviour changes.
- Before touching any pane, capsule, or picker, read [docs/design-system.md](docs/design-system.md): Rune
  only, `rune-kit` + `rune_ui/controls.rs` / `rune_ui/lists.rs`, OpenCode theme tokens, no colour literals.
- At the start of a work round, bump the Rune pin (`Cargo.toml`, `Cargo.lock`) to Rune's `dev` head and
  update `cargoLock.outputHashes."rune-0.0.1"` in `nix/package.nix`.

## Commands

```sh
./scripts/setup.sh && ./scripts/setup-parakeet.sh   # first-time setup
scripts/check                    # fmt, code-shape, clippy -D warnings, every test (nextest)
scripts/check meeting::query     # fmt + code-shape, then only tests matching the filter
cargo run -- listen              # run the listener
cargo run -- status --json       # ask the running Hex (docs/cli.md)
cargo run -- app -- --list-scenes
cargo run -- app -- --snap /tmp/x.png --scene mac-settings --theme light   # headless UI render
git diff --check
```

- Use `scripts/check [filter]`, not raw `cargo test`; it needs `cargo-nextest` and kills hung tests by
  name. Read its output whole (failures are summarized at the end). Exit codes: 10 fmt, 11 clippy,
  12 tests, 13 code-shape.
- `crates/capture` and `crates/capture-core` are excluded from the workspace (`crates/router` is a member);
  on macOS `scripts/check` names them explicitly, selecting the root package by `cargo pkgid` because a
  dependency is also called `hex`.
- Golden images (`tests/golden/`) only run on Linux. Any change to what the window draws regenerates them
  in the same PR: `HEX_UPDATE_GOLDENS=1 cargo test --locked --bin hex rune_ui` on Linux, or from a Mac
  dispatch `check-linux.yml` on the branch and take its `golden-actual` artifact. Look at every changed image.
- Iterate on UI with `--snap` scenes (fixtures from `SCENES` in `src/rune_ui.rs`) in light and dark; a scene
  is not end-to-end proof, and capsules/HUDs have no scene.
- TypeScript SDK (`sdk/typescript`, `@kitlangton/hex`): `cd sdk/typescript && bun run check && bun run test && bun run build`;
  user-facing SDK changes need a Changeset in `sdk/.changeset/`.

## Code shape (enforced by `scripts/code-shape.py`)

- Source files under 800 non-test lines; 1,500 hard cap (legacy exceptions in
  `scripts/file-size-exceptions.txt` only go down).
- `x.rs` beside two or more `x_*.rs` (or three `x_*.rs` alone) moves into `x/`; folders over 40 source files
  warn, over 60 fail. Refresh ratchets with `python3 scripts/code-shape.py --update-exceptions`.
- Keep modules deep; add a seam only when two real adapters exist or a test needs substitution.

## Architecture in one pass

Hex is one long-lived Rust process per user (edition 2024, single `hex` binary, CLI via `clap`):
a menu-bar app on macOS (`macos_rune`), a tray app on Windows (`windows/rune`), a single-instance
process on Linux (`linux/app`). Closing the window never stops the runtime.

- **Audio → transcript:** `audio` / `dictation/audio` own the authoritative microphone timeline;
  `suppression` (macOS taps), `windows/input`, and `linux/input` feed one shortcut state machine model
  (chords, hold, toggle, double-tap lock). `transcription/engine` is the single thread that owns the
  loaded model and runs every inference in order (live streams, whole clips, background meeting/API work
  that yields to dictation). `parakeet` / `transcription/local_transcriber` / `apple_speech` are the backends.
- **Text → app:** `dictation/modes` and `dictation/processor` apply corrections, optional OpenCode rewrites
  (only via an already running service, `desktop/opencode_service`), and user transformations
  (`personal_commands`: `~/.config/hex/hex.config.ts` run in embedded QuickJS). `paste` inserts via the
  clipboard and restores it; `history` keeps bounded retained output.
- **UI:** `rune_ui` is the one window on every OS; `platform::Capabilities` (`src/rune_ui/platform.rs`) decides which panes and rows
  exist, and every OS difference must be named in `platform::DIFFERENCES` (enforced by `parity_tests`).
  OS glue lives only in `rune_ui/{macos,linux,windows}_host.rs`. `rune_hud` draws the dictation HUD on the GPU.
- **Control surface:** the running app serves an authenticated loopback API (`local_api`, discovery file
  `local-api.json`); `hex` CLI control commands go through `POST /control` (`control`, contract in
  `docs/cli.md`). Every persisted setting needs a `hex settings` key (test enforces it).
- **Features:** `meeting/*` (two-track capture, rolling + final transcripts, notes, MCP via `hex mcp`),
  `capture_host` (Annotate, Scrawl's engine from `crates/capture`), `updater/*` (release-service client),
  `labs` (feature registry), `app_settings` (the one `settings.json`; settings errors never block launch).

## Diagnostics

Data root: `~/Library/Application Support/hex` (macOS), `%APPDATA%\hex` (Windows), `~/.local/share/hex`
(Linux). Logs: `logs/live.ndjson` (state/transcript/outcome events) and `logs/process.log`.
