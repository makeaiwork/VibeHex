---
topic: cli-and-agents
title: The hex command, AI agents and the local API
keys: [labs.ai_cleanup_server, meetings.enabled, vocabulary]
os: every OS
---

# The hex command, AI agents and the local API

VibeHex comes with a terminal command, `hex`, for people comfortable with a terminal and for AI agents such as Claude Code or Codex. On a Mac, if `hex` is not found, use `/Applications/Hex.app/Contents/MacOS/hex`.

## Control the running app

Most commands talk to the VibeHex that is running. If it is not, they stop with exit code 3 and never launch it (`hex start` does that). Add `--json` for machine-readable output.

```sh
hex status
hex settings set sound_effects off
hex labs on ai-cleanup-server
hex vocabulary add Kubernetes --sounds-like kuber,cuber
hex meetings show latest
hex mcp
```

- `hex status`: whether it is listening, the current model, permissions.
- `hex settings list`, `hex settings get <key or prefix>`, `hex settings set <key> <value>`: every saved setting, changed as the window would change it. Booleans take `on` or `off`, and `none` clears an optional one. `hex settings apply <file>` sets several at once.
- `hex labs` lists Labs features; `hex labs on <name>` and `hex labs off <name>` switch one. The names are in each Labs article.
- `hex vocabulary list`, `add`, `remove`: the shared Vocabulary.
- `hex history` reads kept dictations (filters `--since 3d`, `--app`, `--grep`, `--search`), and `hex history clear --yes` deletes them all.
- `hex meetings list`, `show`, `chapters`, `search` read recorded meetings and work even with the app closed. `hex meetings record` and `stop` need the app running.
- `hex dictation start`, `stop`, `toggle` do what the shortcut does.

## Let an AI agent read your meetings

`hex mcp` is a read-only MCP server over standard input and output. Add it to an agent's MCP settings as a local command `hex` with the argument `mcp`. The agent can then list meetings, search transcripts, read a transcript or part of one, and read stored notes and chapters. It works whether or not Meetings is switched on, changes nothing, makes no network requests, and runs no model. Transcripts leave your computer only through the agent you connect. `--meetings-dir <folder>` reads a copied meetings folder, for example one synced from another computer.

## The local API

While running, VibeHex listens on `127.0.0.1` only (your own computer), protected by a secret token kept in a private file, `local-api.json`, in its data folder. The `hex` command uses it, as can your own scripts. Keep the token private.

**Asked as.** "Can I control VibeHex from the terminal?"; "How do I connect Claude Code to my meetings?"; "Is there an API?"; «Как управлять VibeHex из терминала»; «Как дать агенту доступ к записям встреч»; «Команда hex не найдена».

<!-- sources: docs/cli.md:1-60, 205-230; src/control/cli.rs:20-230; src/control/transcript_cli.rs:51-140; src/labs.rs:1172-1260, 1278-1303; docs/features/control-cli.md:60-90; docs/features/meetings.md:1344-1395; docs/transcript-cli.md:1-30; src/meeting/mcp.rs:1-30; src/local_api.rs:577-660 -->
