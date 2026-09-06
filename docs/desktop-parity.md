# Desktop parity checklist

> Status: **0.3.0** — every TUI workbench capability below has a GUI entry, a
> command entry, and an automated test. The alignment baseline is upstream
> Asterline `v1.0.1` (`e94e357`).

Asterline Desktop shares one feature contract with the TUI: the composer
parser, the command catalog, completion, and skill invocation rules live in
the UI-agnostic Rust module `src/contract` and are consumed by both surfaces.
The desktop never re-implements parsing in TypeScript; the WebView asks the
host (`parse_composer_text`) for the same `ParsedInput` the TUI consumes.

## 1. Composer contract

| TUI capability | Desktop GUI entry | Desktop command entry | Automated acceptance |
| --- | --- | --- | --- |
| `/ask <member> <text>` | — (target selector + mention) | shared parser → `user_message` | `contract::tests::ask_command_targets_member`, `composer::tests::slash_commands_map_to_desktop_commands` |
| `/all <text>` | — | shared parser → `user_message` (all) | `contract::tests::all_command_broadcasts` |
| `@member <text>` / `@all <text>` | mention typing with completion popup | shared parser → `user_message` | `composer::tests::mention_becomes_a_structured_user_message`, `App.test.tsx` send flow |
| `/team` `/runs` `/logs` `/diff` `/mode` `/focus` | utility launcher, Runs/Mode/Team panels, logs drawer | `ComposerAction::surface` mapping | `composer::tests::surfaces_map_to_gui_panels`, e2e `opens the runs panel…` |
| `/new` `/clear` | Sidebar "New chat" | `new_session` | `tui::commands::tests::tui_and_contract_accept_identical_commands`, `contract.test.ts` alias test |
| `/resume` | Sidebar conversation list (searchable) | `request_resume` | `App.test.tsx` resume request |
| `/retry` | — | `retry` | `composer::tests::slash_commands_map_to_desktop_commands` |
| `/attach <member>` | Inspector member card, attach buttons | `open_native_session` host API | `App.test.tsx` attach guard, `attach::tests` |
| `/import <session_id>` / `@member /import` | Sidebar "Import native session" modal | `import_session` | `session_adapter::tests::queue_import_and_export_commands_map_through`, e2e import modal |
| `/export [claude]` | Sidebar "Export to Claude format" | `export_session` | same |
| `/approve` `/reject` | Approval queue buttons | `approve_first` action → `approve` | `App.test.tsx` approval flow |
| `/continue [run] [note]` | Runs panel "Continue" | `continue_run` | `RunsPanel` vitest coverage via mock, e2e runs panel |
| `/note [run] <text>` | Runs panel note field | `note_run` | `contract.test.ts` note/block flow |
| `/block [run] <reason>` | Runs panel block field | `block_run` | same |
| `/verify [run] [command]` | Runs panel verify (suggested verify prefilled) | `verify_run` | `session_adapter` mapping, e2e |
| `/step add\|todo\|doing\|done\|block\|rename\|edit\|remove\|delete\|drop\|assign\|owner\|unassign\|clear-owner` | Runs panel step editor (status/owner selects, add, remove) | same `run_*` commands | `contract::tests::plan_and_focus_commands` (arity), `session_adapter::tests::zero_step_is_rejected_at_bridge_boundary` |
| `/mode <mode>` | Mode switcher buttons | `set_mode` | e2e `opens the command palette … and switches mode` |
| `/find <query>` | Find tab of the utility drawer | `ComposerAction::find` | `App.test.tsx` find flow |
| `/help` | Command palette (read-only mode) | help action | e2e palette test |
| `/exit` | App close / Exit | `exit_desktop` (graceful runtime shutdown, then window close) | `session_adapter` shutdown mapping; host `exit_desktop` |
| `/skills`, `/abort`, `/model`, `/plan`, `/lead` … (removed commands) | — | fall back to help / invalid with draft kept | `contract::tests::removed_*` tests, `contract.test.ts` |
| Targeted `@member /skill` | Composer completion offers only that backend's discovered skills; undiscovered → structured error, draft kept | resolved in host against `skills::discover` | `composer::tests::targeted_skill_must_be_discovered_for_that_backend` |
| Command catalog & completion | Command palette (Ctrl+K), completion popup | `command_catalog`, `complete_composer` IPC | `composer::tests::completion_covers_commands_members_and_skills`, `catalog_lists_every_shared_command` |
| Composer history / drafts | ArrowUp/Down history, per-target drafts, Ctrl+R reverse search, 256 KiB budget | — (local UI state mirroring TUI caps) | `MAX_COMPOSER_BYTES` mirror, e2e queue test types via composer |
| Image attachments (paste/drag/choose, max 4, TIFF→PNG, token-only) | Composer image buttons + chips; placeholders shown on user messages | `stage_clipboard_image`, `stage_image_path`, `stage_image_bytes`, `remove_staged_attachment`, `discard_staged_attachments`; `user_message.attachments` tokens | `attachments::tests`, `session_adapter::tests::attachments_become_managed_marker_lines`, `more_than_four_images_are_rejected` |
| Queued prompts (send while busy) | Composer queue bar + "pull back" | `edit_queued_prompt`; structured `queue_updated`/`queued_prompt_returned` events | `bridge::tests::queue_updates_are_structured_not_notices`, `queued_prompt_return_is_structured_and_clears_the_queue`, e2e queue test |
| Relay pause/resume + paused-route continue/drop | Topbar relay toggle; paused-route cards | `set_relay_paused`, `resolve_paused_route` | `bridge::tests::paused_route_*`, `App` toggle |
| Timeline caps (5 000 items, 256 KiB/item, explicit truncation state) | Truncation pill on timeline and items | host caps + reducer mirror | `bridge::tests::timeline_caps_mirror_the_tui_and_report_truncation`, `oversized_streaming_text_is_clamped_with_an_explicit_flag` |
| Safe Markdown/GFM rendering (raw HTML disabled) | Agent messages rendered via `renderMarkdown` | — | `markdown.test.ts` (escape-first, link protocols) |
| Snapshot recovery point | Every event reduces into `DesktopSnapshotV2`; resume/new/reset restore atomically | `get_desktop_snapshot` | `bridge::tests::streaming_message_is_reduced_into_recoverable_snapshot`, `conversation_restore_tracks_active_conversation` |

## 2. Modes

| TUI capability | Desktop GUI entry | Automated acceptance |
| --- | --- | --- |
| Five modes with binding summary | Mode panel (utility launcher "Modes", `/mode`) | e2e mode panel flow |
| Per-field sources: `default` / `team.json` / `this chat` | Source chips in the mode panel | `ModePanel` render + `mergeModes` unit tests |
| Conversation overrides (`SetModeOverrides`) | Editable knobs in the panel | `session_adapter::tests::mode_overrides_map_onto_the_runtime_command` |
| Save overrides into team.json (`SaveModeDefaults`) | "Save as team default" | mock command path; runtime `team_runtime` shares the TUI implementation |
| Reset overrides | "Reset overrides" | `bridge::clear_mode_overrides` + host command |
| Start a mode run (`RunMode`) | Task box in the panel | mock `run_mode` handler, `session_adapter` validation |
| Plan `builder` / `auto_execute` fields | Settings → Modes → Plan; mode panel knobs | `bridge::tests::plan_mode_builder_and_auto_execute_round_trip` |
| Unknown settings fields preserved | — (host-side guarantee) | DTO `#[serde(flatten)] extra` round-trip tests |

## 3. Team & sessions

| TUI capability | Desktop GUI entry | Automated acceptance |
| --- | --- | --- |
| CLI install detection | Settings → General "Backend CLIs on PATH" | `catalog::tests::availability_reports_four_backends` |
| Real model catalog per backend + manual entry + refresh | Settings → Members model `datalist` + Refresh | mock `list_models`; host `list_models` uses shared `discover_models` |
| Native session search/selection for `session_id` | Settings → Members session `datalist` | mock `listNativeSessions`; `native_sessions` module tests (claude/codex/grok listing) |
| Default target, full member fields, approval policy | Settings (unchanged panes + additions) | `SettingsModal.test.tsx` |
| Full conversation history (search, not first 8) | Sidebar search box | Sidebar rendering, e2e snapshot |
| Session import | Sidebar → Import modal (backend + member + search) | e2e import modal smoke |
| Session export (Claude format) | Sidebar export button | `contract.test.ts` export notice path |
| Attach busy-state protection | Host refuses attach while runtime busy | `open_native_session` guard, `attach::tests::active_members_cannot_be_attached` |

## 4. Launch & operations

| TUI capability | Desktop GUI entry | Automated acceptance |
| --- | --- | --- |
| `--team <path>` / `--pick-team` | Project picker "Advanced launch" | `session_adapter::tests::launch_options_map_onto_shared_session_options` |
| `--db <path>` | Advanced launch "Database path" | same |
| `--no-restore` | Advanced launch "Restore last conversation" checkbox | same |
| `--debug` (approval gates off, session-scoped) | Advanced launch risk confirmation (two-step checkbox) | `launch_options_default_to_safe_values`, picker disables Open until acknowledged |
| `--fake` (session-scoped) | Advanced launch "Use offline fake agents" | same |
| `--no-auto-update` | Desktop is manual-by-default; auto-check opt-in | `to_session_options` test |
| `--banner` | N/A (terminal decoration, intentionally not copied) | — |
| Update check via trusted release page | Topbar update button + official release URL only | `update_check::tests::only_official_release_urls_can_be_opened` |
| Graceful shutdown on exit | `/exit` and window close both stop the runtime before exiting | host `shutdown_inner` / `CloseRequested` handler |
| Single workspace lock; WebView never touches SQLite/`team.json`/CLIs | unchanged architecture | host is the only IPC surface; `is_lock_error` → Locked phase |
| No telemetry; diagnostics are local | Logs drawer export | `App.test.tsx` diagnostics export |

## 5. Quality gates

- `cargo test` (root crate, includes `contract`, `native_sessions`, TUI regression suites) — 966 tests.
- `cargo clippy` clean (root + `desktop/src-tauri`).
- `cargo test` in `desktop/src-tauri` — 59 tests (bridge V2, session adapter, composer parsing, attachments, launch options).
- `pnpm lint` (strict TypeScript), `pnpm test` (36 Vitest tests), `pnpm build`.
- `pnpm test:e2e` — 6 Playwright scenarios over the mock runtime (primary flow, panels/utilities, settings persistence, queue pull-back, palette, runs panel).
- Three-platform smoke (clipboard images, external attach, paths, lock conflict, diagnostics, installer launch) follows `docs/real-smoke.md` per release.
