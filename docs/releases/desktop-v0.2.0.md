# Asterline Desktop v0.2.0

Desktop V1.1 adds read-only utility drawers for runtime logs, Git diff, local
Skills, and search within the currently loaded conversation. Utility requests
are bounded and versioned, so large workspaces do not inflate the runtime
snapshot.

- Logs support level, source, and text filters with a 400-entry limit.
- Diff includes staged, unstaged, and untracked changes with a 2 MiB limit and
  a 10-second Git timeout.
- Skills support backend/text filters and copyable invocations without exposing
  local absolute paths to the WebView.
- Find supports match counts and previous/next navigation in the active chat.

The release keeps the Desktop bridge version at `1` and preserves the existing
workspace lock, session restore, approval, and team settings behavior.
