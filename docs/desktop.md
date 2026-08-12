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

The first release intentionally does not embed a terminal, provide repository
search, expose a separate log explorer, or add telemetry.

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
workspace's `.asterline` directory. Desktop V1 sends no telemetry.

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

- Windows: per-user Inno Setup installer (including the signed Evergreen
  WebView2 bootstrapper) and portable ZIP.
- macOS: Developer ID signed and notarized DMG and portable app archive for
  each architecture. The release fails when signing credentials are absent.
- Linux: AppImage built natively on each architecture.

The app's manual update check only considers `desktop-v*` releases and never
installs an update automatically.

Use the Windows installer on systems that may not already have WebView2. The
portable ZIP does not install operating-system prerequisites.
