# Asterline GUI v0.3.0

`gui-v0.3.0` is the first source-preview release of the redesigned Asterline Desktop workbench in [PaoPao1021/Asterline](https://github.com/PaoPao1021/Asterline). It is based on upstream Asterline v1.0.4 and keeps the CLI/TUI runtime, storage, commands, modes, and safety boundaries intact.

> This release contains source code and interface documentation. It does not include signed Windows, macOS, or Linux installers. Use the [upstream releases](https://github.com/song0705/Asterline/releases/latest) for official packaged builds.

## Highlights

- Desktop functions are aligned with the TUI through the shared versioned bridge and command contract.
- Continuous three-column workbench with responsive sidebar and inspector recovery.
- Neutral light and dark themes with restrained green semantics and subtle frost limited to the navigation sidebar.
- Project history, member routing, approvals, Runs, five collaboration modes, themed dropdowns, settings, logs, Diff, Skills, and command palette remain available.
- Compact controls use readable 14px labels and consistently scaled Lucide icons with a single 2px visual stroke.
- The composer now has one rounded focus boundary instead of a second square textarea outline.

## Screenshots

![Asterline Desktop dark workbench](https://raw.githubusercontent.com/PaoPao1021/Asterline/gui-v0.3.0/docs/assets/desktop-workbench-dark.png)

| Light workbench | Team settings |
| --- | --- |
| ![Light workbench](https://raw.githubusercontent.com/PaoPao1021/Asterline/gui-v0.3.0/docs/assets/desktop-workbench-light.png) | ![Team settings](https://raw.githubusercontent.com/PaoPao1021/Asterline/gui-v0.3.0/docs/assets/desktop-settings-dark.png) |

![Structured Runs](https://raw.githubusercontent.com/PaoPao1021/Asterline/gui-v0.3.0/docs/assets/desktop-runs-dark.png)

## Verification

- Production frontend build passed.
- 56 unit tests passed.
- 10 Chromium end-to-end tests passed, including accessibility scans, panel recovery, dropdown keyboard behavior, icon rendering, and the composer focus-boundary regression.

## Documentation

- [Desktop guide](../desktop.zh-CN.md)
- [TUI/Desktop parity matrix](../desktop-parity.zh-CN.md)
- [UI refactor notes](../desktop-ui-refactor.md)
- [Upstream Asterline v1.0.4 notes](v1.0.4.md)
