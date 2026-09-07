# 桌面端功能对齐清单

> 状态：**开发中（v1.0.4 对齐）** — 对齐基准为上游 Asterline `v1.0.4` 之后的最新主干（`93746b9`）。

Asterline Desktop 与 TUI 共享同一功能契约：命令解析、命令目录、补全与定向 Skill 调用规范位于 UI 无关的 Rust 模块 `src/contract`，两端共同消费。桌面端不再用 TypeScript 重复解析；WebView 通过宿主 IPC（`parse_composer_text`）获得与 TUI 相同的解析结果。

## 1. 输入框契约

| TUI 能力 | 桌面 GUI 入口 | 桌面命令入口 | 自动化验收 |
| --- | --- | --- | --- |
| `/ask <member> <text>` | —（目标选择器 + @提及） | 共享解析 → `user_message` | `contract::tests::ask_command_targets_member` 等 |
| `/all <text>` | — | 共享解析 → 广播消息 | `contract::tests::all_command_broadcasts` |
| `@member` / `@all` | 输入时补全弹层 | 共享解析 → `user_message` | `composer::tests::mention_becomes_a_structured_user_message` |
| `/team` `/runs` `/logs` `/diff` `/mode` `/focus` | 工具行、Runs/模式面板、团队设置、日志抽屉 | `ComposerAction::surface` 映射 | `composer::tests::surfaces_map_to_gui_panels`、e2e |
| `/new` `/clear` | 侧栏“新对话” | `new_session` | 契约一致性测试（TUI/桌面同判） |
| `/resume` | 侧栏会话列表（可搜索） | `request_resume` | `App.test.tsx` |
| `/retry` | — | `retry` | `composer::tests` |
| `/attach <member>` | Inspector 成员卡 attach | `open_native_session` | attach 忙碌保护测试 |
| `/import` / `@member /import` | 侧栏“导入原生会话” | `import_session` | `session_adapter::tests` |
| `/export [claude]` | 侧栏导出按钮 | `export_session` | 同上 |
| `/approve` `/reject` | 审批卡按钮 | `approve_first` → `approve` | `App.test.tsx` 审批流 |
| `/continue` `/note` `/block` | Runs 面板对应操作；`/verify` 与引擎验证已按 v1.0.2 移除 | `continue_run` 等 | `contract.test.ts`、e2e |
| `/step` 全部子命令 | Runs 面板步骤编辑（状态/负责人/新增/删除） | `run_*` 命令 | 参数缺失/非法 run ID 测试 |
| `/mode <mode>` | 模式切换按钮 | `set_mode` | e2e 面板切换模式 |
| `/find` | 工具抽屉 Find 页 | `ComposerAction::find` | `App.test.tsx` |
| `/help` | 命令面板（只读帮助模式） | help 动作 | e2e 面板测试 |
| `/exit` | 窗口关闭 / 退出 | `exit_desktop`（先优雅停机再关窗） | 宿主 shutdown 测试 |
| 已移除命令（`/skills` 等） | — | 回退 help/invalid 且保留草稿 | `contract::tests::removed_*` |
| 定向 `@member /skill` | 补全仅展示该后端已发现 Skill；未发现则结构化报错并保留草稿 | 宿主用 `skills::discover` 校验 | `composer::tests::targeted_skill_must_be_discovered_for_that_backend` |
| 命令目录与补全 | Ctrl+K 命令面板、输入补全 | `command_catalog` / `complete_composer` IPC | `composer::tests` |
| 历史/草稿/Ctrl+R/256 KiB 上限 | 方向键历史、按目标草稿、历史搜索、输入截断 | 本地状态，镜像 TUI 上限 | e2e + `MAX_COMPOSER_BYTES` |
| 图片附件（粘贴/拖放/选择，最多 4 张，TIFF→PNG，仅 token） | 附件按钮与芯片；用户消息展示占位符 | `stage_*`、`remove_staged_attachment`、`discard_staged_attachments`；`user_message.attachments` | `attachments::tests`、超限拒绝测试 |
| 队列（忙时发送） | 输入框队列栏 + “拉回编辑” | `edit_queued_prompt`；结构化 `queue_updated`/`queued_prompt_returned` | `bridge::tests`、e2e 队列用例 |
| 中继暂停/恢复、暂停路由继续/丢弃 | 顶栏中继开关；暂停路由卡片 | `set_relay_paused`、`resolve_paused_route` | `bridge::tests::paused_route_*` |
| 时间线上限（5000 条 / 256 KiB，明确截断状态） | 截断提示与单项截断徽标 | 宿主上限 + 前端镜像 | `bridge::tests::timeline_caps_mirror_the_tui_*` |
| 安全 Markdown/GFM（禁用原始 HTML） | Agent 消息经 `renderMarkdown` 渲染 | — | `markdown.test.ts` |
| 快照恢复点 | 所有事件先归约为 `DesktopSnapshotV2` | `get_desktop_snapshot` | `bridge::tests` 流式归约测试 |

## 2. 协作模式

| TUI 能力 | 桌面 GUI 入口 | 自动化验收 |
| --- | --- | --- |
| 五种模式与绑定摘要 | 模式面板 | e2e 模式面板流程 |
| 字段来源：`default` / `team.json` / `this chat` | 面板来源徽标 | `mergeModes` 单测 |
| 会话级覆盖 | 面板可编辑旋钮 | `mode_overrides_map_onto_the_runtime_command` |
| 保存为团队默认 | “保存为团队默认” | mock 命令路径（运行时与 TUI 共用实现） |
| 重置覆盖 | “重置覆盖” | `bridge::clear_mode_overrides` |
| 直接启动模式运行 | 面板任务框 | mock `run_mode`、适配器校验 |
| Plan `builder` / `auto_execute` | 设置→模式→Plan；面板旋钮 | `current_mode_fields_round_trip` |
| Review `reviewer_hint` | 设置→模式→Review；面板旋钮 | `current_mode_fields_round_trip`、`ModePanel.test.tsx` |
| Team `allow_add_members` | 设置→模式→Team；面板旋钮 | 同上 |
| `/new` 保留模式覆盖；Run 显示会话内编号 | 模式状态保留，Runs 使用 `run-{number}` | 根运行时回归 + 前端渲染 |
| 未知设置字段原样保存 | —（宿主侧保证） | DTO `#[serde(flatten)] extra` 往返测试 |

## 3. 团队与会话

| TUI 能力 | 桌面 GUI 入口 | 自动化验收 |
| --- | --- | --- |
| CLI 安装检测 | 设置→General 的 CLI 状态芯片 | `catalog::tests` |
| 真实模型目录 + 手动输入 + 刷新 | 设置→成员的模型 datalist + 刷新 | mock `list_models`；宿主复用共享 `discover_models` |
| 原生会话搜索选择 | 设置→成员的会话 datalist | `native_sessions` 模块测试（Claude/Codex/Grok） |
| 默认目标、完整成员字段、后端原生权限预设 | 设置→成员；Codex/Claude/Grok/Agy 使用各自 CLI 名称 | `SettingsModal.test.tsx`、根权限映射测试 |
| Codex 原生人工审批（默认关闭） | 设置→审批；高级启动可仅本次开启 | `SettingsModal.test.tsx`、启动项映射测试 |
| 完整历史搜索（不再只显示前 8 条） | 侧栏搜索框 | e2e 页面快照 |
| 原生会话导入 | 侧栏导入弹窗（后端 + 成员 + 搜索） | e2e 冒烟 |
| Claude 格式导出 | 侧栏导出按钮 | `contract.test.ts` |
| attach 忙碌保护 | 宿主在运行中拒绝 attach | `open_native_session` 守卫测试 |

## 4. 启动与运维

| TUI 能力 | 桌面 GUI 入口 | 自动化验收 |
| --- | --- | --- |
| `--team` / `--pick-team` | 项目选择器“高级启动” | `launch_options_map_onto_shared_session_options` |
| `--db` | 高级启动数据库路径 | 同上 |
| `--no-restore` | 高级启动恢复开关 | 同上 |
| `--debug`（仅开发诊断） | 高级启动开关；不再隐式改变审批 | 启动项映射测试 |
| `--manual-approvals` | 高级启动“人工审批 Codex 工具请求” | 启动项映射测试 |
| `--fake`（仅本次启动） | 高级启动离线假代理 | 同上 |
| `--no-auto-update` | 桌面默认手动检查；可自愿开启 | `to_session_options` 测试 |
| `--banner` | 不适用（终端装饰，有意不复制） | — |
| 更新只走官方 Release 页 | 顶栏更新按钮 | `update_check` 域名校验测试 |
| 退出前优雅停机 | `/exit` 与关窗均先停运行时 | 宿主 shutdown 测试 |
| 单工作区锁；WebView 不直接访问 SQLite/`team.json`/CLI | 架构不变 | 宿主是唯一 IPC 面；锁冲突 → Locked 阶段 |
| 无遥测；诊断本地导出 | 日志抽屉导出 | `App.test.tsx` 诊断导出 |

## 5. 质量门

- 根 crate `cargo test`（含 `contract`、`native_sessions` 与 TUI 回归套件）— 995 项。
- 根 crate 与 `desktop/src-tauri` 的 `cargo clippy` 干净。
- `desktop/src-tauri` `cargo test` — 60 项（桥接 V2、适配器、解析、附件、启动项）。
- `pnpm lint`（严格 TS）、`pnpm test`（46 个 Vitest）、`pnpm build`。
- `pnpm test:e2e` — 9 个 Playwright 场景（主流程、无障碍、面板/工具、设置持久化、队列拉回、命令面板、Runs 面板）。
- 三平台冒烟（剪贴板图片、外部终端 attach、路径、锁冲突、诊断、安装包启动）按 `docs/real-smoke.md` 随发布执行。
