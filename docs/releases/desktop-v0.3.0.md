# Asterline Desktop v0.3.0

Desktop 0.3.0 completes the TUI parity program: every workbench capability of
the terminal UI now has both a GUI entry and the same slash-command entry,
driven by one shared parser. The bridge moves to **version 2** (WebView IPC
only; SQLite stores, session snapshots, and `team.json` keep their formats).

## One contract for TUI and Desktop

- Composer parsing, the command catalog, completion, and targeted-skill
  validation moved into a shared Rust module (`asterline::contract`). The
  desktop WebView no longer parses anything itself — every submission goes
  through the host and behaves exactly like the TUI.
- All catalog commands work in the desktop composer: `/ask`, `/all`, `/team`,
  `/runs`, `/logs`, `/diff`, `/focus`, `/find`, `/mode`, `/new`/`/clear`,
  `/resume`, `/retry`, `/attach`, `/import`, `/export`, `/approve`, `/reject`,
  `/continue`, `/note`, `/block`, `/verify`, every `/step` subcommand, `/help`,
  and `/exit` (graceful runtime shutdown, then window close).
- `@member /skill` is validated against the target backend's discovered skills
  before anything reaches a noninteractive runner; unknown commands keep the
  draft and report a structured error.

## Structured state & recovery

- `DesktopSnapshotV2` adds conversation-scoped mode overrides, per-member
  prompt queues, relay pause state, the suggested verify command, and the
  active conversation. Queue updates and pull-backs are structured events
  instead of notices.
- `ModeRunV2.state` carries structured phase/iteration/round/idea/vote fields.
- Plan-mode `builder` and `auto_execute` settings no longer disappear when
  edited, and any settings field the desktop does not model yet survives edits
  unchanged.
- The timeline mirrors TUI caps (5 000 items, 256 KiB per item) and shows an
  explicit truncation state instead of silently dropping history.
- Agent messages render safe Markdown/GFM (raw HTML is escaped, link protocols
  are whitelisted).

## Workbench

- Composer: Enter sends, Shift/Alt+Enter inserts a newline, per-target drafts,
  arrow-key history with Ctrl+R reverse search, the TUI's 256 KiB budget and
  1 000-entry prompt history, shared completions, and a Ctrl+K command palette
  (`/help` opens the same palette read-only).
- Running-again sends join the member's queue with a visible queue bar; the
  last queued prompt can be pulled back into the composer.
- Image attachments: paste, drag-drop, or file picker (PNG/JPEG/GIF/WebP/TIFF,
  max four, TIFF converts to PNG). The host persists them to the managed paste
  directory and the WebView only ever sees opaque tokens.
- New mode panel: five modes, per-field source chips (default / team.json /
  this chat), apply to this chat, save as team default, reset overrides, and
  direct mode-run launch.
- Runs panel: full status, structured mode state, step add/status/rename/
  remove/assign, note/block/continue/verify, and event history.
- Sessions: full searchable history (no longer capped at eight), native
  session import (Claude/Codex/Grok) with previews, and one-click export to
  Claude format.
- Team settings: backend CLI detection, per-backend model catalogs with manual
  entry, native session search for `session_id`, plan-mode builder/auto-execute
  fields, and the unchanged approval policy editor.
- Themed dropdowns replace every native `<select>`; keyboard focus is visible
  application-wide, and axe scans report no serious or critical violations.

## Launch & operations

- Advanced launch in the project picker: team roster file, ask-on-open,
  custom database path, restore toggle, update-check opt-in, offline fake
  agents, and approval-gate-off debug mode behind an explicit risk
  confirmation. Debug/fake apply to that launch only.
- The update check now uses rustls, removing the OpenSSL dependency from the
  Linux package build.
- `ASTERLINE_DESKTOP_FAKE=1` runs the offline fake agents in debug builds for
  smoke testing; it is ignored by release builds and never persisted.

## Platform notes

- Windows, macOS (Apple silicon + Intel), and Linux packages are built by the
  release workflow; icons, config paths, external-terminal attach, and
  clipboard image capture (X11/Wayland) keep their per-platform adapters.
- See `docs/desktop-parity.md` for the full capability → GUI → command → test
  matrix.
