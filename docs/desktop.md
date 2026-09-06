# Asterline Desktop V1

Asterline Desktop is the graphical companion to the existing `asterline` and
`ast` terminal applications. The desktop app and the TUI use the same runtime,
team configuration, conversation store, and workspace lock. They must not be
opened on the same workspace at the same time.

## V1 scope

- Open a project or choose from recent projects.
- Send messages to a member or the whole team and stream runtime events.
- Inspect member state, change modes, cancel work, and answer approvals.
- Browse and continue Runs, start verification, and restore local history.
- Edit the complete team configuration without discarding advanced fields.
- Open a member's native CLI in an external terminal. Codex and Claude history
  can be imported after the terminal exits; Grok and Agy remain terminal-only
  in V1.
- Switch between English and Simplified Chinese, light and dark themes.
- Check manually for Desktop releases.

Desktop V1.1 also adds four read-only utility drawers:

- `/logs` browses runtime diagnostics with level, source, and text filters. Each request returns at most 400 entries and reports truncation explicitly.
- `/diff` shows staged, unstaged, and untracked changes relative to Git HEAD. It never mutates the workspace, caps output at 2 MiB, and stops Git after 10 seconds.
- `/skills` lists skills discovered for the workspace, with backend and text filters plus copyable invocations. Results are capped at 512 and never expose absolute local paths to the WebView.
- `/find` searches the currently loaded conversation timeline only, with match counts and previous/next navigation.

Utility data is requested on demand through versioned bridge commands rather than being added to `DesktopSnapshotV1`. Switching workspaces, restoring a conversation, or shutting down clears stale utility results.

The first release intentionally does not embed a terminal or add telemetry.

## 0.3.0 — full TUI parity

Desktop 0.3.0 completes the alignment with the terminal UI. Composer parsing,
the command catalog, completion, and targeted-skill validation live in a shared
Rust contract (`asterline::contract`); the desktop composer accepts every TUI
slash command, alias, and `@member /skill` form with identical behavior,
including `/exit` (graceful shutdown, then window close).

Highlights on top of the V1.1 drawers:

- **Bridge v2.** `DesktopSnapshotV2` adds conversation-scoped mode overrides,
  per-member prompt queues, relay pause state, the suggested verify command,
  and the active conversation. Queue updates and pull-backs are structured
  events. The WebView IPC version is now `2`; SQLite stores, session
  snapshots, and `team.json` keep their on-disk formats.
- **Queues and images.** Sending while a member runs joins that member's queue
  (visible queue bar, last-queued pull-back). Images attach via paste,
  drag-drop, or file picker (PNG/JPEG/GIF/WebP/TIFF, max four, TIFF converts
  to PNG); the host stages them in the managed paste directory and the
  WebView only sees opaque tokens.
- **Mode panel.** Per-field binding sources (default / team.json / this chat),
  apply to this chat, save as team default, reset overrides, direct mode-run
  launch, and lossless plan-mode `builder`/`auto_execute` editing.
- **Runs panel.** Structured mode-run state, full step editing (status, owner,
  add, rename, remove), note/block/continue/verify, and event history.
- **Sessions.** Full searchable history, native session import for
  Claude/Codex/Grok with previews, one-click export to Claude format.
- **Advanced launch.** Team roster file, ask-on-open, custom database path,
  restore toggle, update-check opt-in, offline fake agents, and a debug mode
  that disables approval gates behind an explicit risk confirmation. Debug and
  fake apply to that launch only and are never persisted.
- **Themed widgets and a11y.** Every native `<select>` is replaced by a themed,
  keyboard-complete dropdown; focus visibility is guaranteed application-wide
  and the app is checked with automated axe scans.

The update check now uses rustls, so the Linux package build no longer needs
OpenSSL. The full capability matrix (TUI feature → GUI entry → command entry →
automated test) lives in `docs/desktop-parity.md`.

## Architecture and trust boundary

The webview is a presentation layer. It sends a versioned command envelope to
the Rust bridge, which validates it and maps it to `UiCommand`. Runtime events
are converted to versioned wire DTOs before being emitted back to the webview.
The webview never writes SQLite, edits `team.json`, or launches a provider CLI
directly.

```text
React workbench
    | versioned Desktop command/event DTOs
Tauri Rust bridge
    | RuntimeHandle / UiCommand / RuntimeEvent
Asterline runtime
    | atomic team config + SQLite conversation state
<workspace>/.asterline/
```

`team.json` remains the editable startup configuration. SQLite remains the
conversation snapshot and append-only event history. The project state format
for Desktop V1 is `1`; current and previous minor Desktop releases are the
supported compatibility window.

Recent-project metadata and UI preferences are app-local. Project prompts,
responses, tool output, approvals, and provider session identifiers stay in the
workspace's `.asterline` directory. Desktop V1 sends no telemetry. It keeps a
bounded local diagnostic log, records an unclean-exit marker, and lets the user
export a diagnostic text report from the Logs drawer.

## Run from source

Install Rust stable, Node.js 24, and pnpm as described in the project setup.
Platform-specific Tauri prerequisites are also required.

```powershell
cd desktop
pnpm install --frozen-lockfile
pnpm dev
```

For the native application:

```powershell
cd desktop
pnpm tauri dev
```

Quality checks:

```powershell
cd desktop
pnpm lint
pnpm test
pnpm test:e2e
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

The browser development build automatically uses deterministic mock data when
the Tauri bridge is unavailable. This mode is only for UI development and does
not read or mutate a real workspace.

## Packaging and releases

Desktop has an independent `0.x` version line. A tag named
`desktop-v<version>` starts the Desktop release workflow; CLI tags keep the
existing `v<version>` form.

Release artifacts are produced for Windows x64, macOS Intel and Apple silicon,
and Linux x64 and ARM64:

- Windows: Authenticode-signed per-user Inno Setup installer and portable
  executable (including the signed Evergreen WebView2 bootstrapper).
- macOS: Developer ID signed and notarized DMG and portable app archive for
  each architecture. The release fails when signing credentials are absent.
- Linux: AppImage built natively on each architecture.

The app's manual update check only considers `desktop-v*` releases. When an
update exists, it shows both versions and can open only the official Asterline
GitHub Release page in the system browser. It never installs an update
automatically.

Use the Windows installer on systems that may not already have WebView2. The
portable ZIP does not install operating-system prerequisites.
