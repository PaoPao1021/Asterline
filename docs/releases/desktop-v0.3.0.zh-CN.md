# Asterline Desktop v0.3.0

Desktop 0.3.0 完成与 TUI 的功能对齐计划：终端 UI 的每一项工作台能力现在同时具备 GUI 入口与相同的斜杠命令入口，由同一个共享解析器驱动。桥接协议升级到 **版本 2**（仅影响 WebView IPC；SQLite 存储、会话快照与 `team.json` 格式保持不变）。

## TUI 与桌面共用一套契约

- 输入解析、命令目录、补全与定向 Skill 校验移入共享 Rust 模块（`asterline::contract`）。桌面 WebView 不再自己解析——每次提交都经由宿主，行为与 TUI 完全一致。
- 全部目录命令在桌面输入框可用：`/ask`、`/all`、`/team`、`/runs`、`/logs`、`/diff`、`/focus`、`/find`、`/mode`、`/new`/`/clear`、`/resume`、`/retry`、`/attach`、`/import`、`/export`、`/approve`、`/reject`、`/continue`、`/note`、`/block`、`/verify`、全部 `/step` 子命令、`/help`，以及 `/exit`（先优雅停机再关闭窗口）。
- `@member /skill` 在到达非交互运行器前，会先对照该成员后端已发现的 Skill 校验；未知命令保留草稿并给出结构化报错。

## 结构化状态与恢复

- `DesktopSnapshotV2` 新增会话级模式覆盖、逐成员提示队列、中继暂停状态、建议验证命令与当前会话 ID；队列更新与拉回是结构化事件，不再降级为通知文本。
- `ModeRunV2.state` 携带结构化的 phase/iteration/round/idea/vote 字段。
- Plan 模式的 `builder` 与 `auto_execute` 编辑后不再丢失；桌面尚未建模的设置字段在编辑后原样保留。
- 时间线对齐 TUI 上限（5 000 条、单项 256 KiB），超限时显示明确的截断状态而不是静默丢历史。
- Agent 消息渲染安全 Markdown/GFM（原始 HTML 转义、链接协议白名单）。

## 工作台

- 输入框：Enter 发送、Shift/Alt+Enter 换行、按目标保留草稿、方向键历史 + Ctrl+R 反向搜索、TUI 的 256 KiB 输入上限与 1 000 条提示历史、共享补全、Ctrl+K 命令面板（`/help` 以只读方式打开同一面板）。
- 运行中再次发送进入成员队列并显示队列栏；最后一条排队消息可拉回输入框编辑。
- 图片附件：粘贴、拖放或文件选择（PNG/JPEG/GIF/WebP/TIFF，最多四张，TIFF 转 PNG）。宿主把它们持久化到受管粘贴目录，WebView 只接触不透明 token。
- 新增模式面板：五种模式、逐字段来源徽标（default / team.json / 本会话）、应用到当前会话、保存为团队默认、重置覆盖、直接启动模式运行。
- Runs 面板：完整状态、结构化模式状态、步骤新增/状态/重命名/删除/分配负责人、note/block/continue/verify，以及事件历史。
- 会话区：完整可搜索历史（不再只显示前八条）、原生会话导入（Claude/Codex/Grok，带预览）、一键导出 Claude 格式。
- 团队设置：后端 CLI 检测、按后端的模型目录 + 手动输入、`session_id` 的原生会话搜索、Plan 模式 builder/auto-execute 字段，审批策略编辑器保持不变。
- 全部原生 `<select>` 替换为主题化下拉；键盘焦点全局可见，axe 扫描无 serious/critical 违规。

## 启动与运维

- 项目选择器新增“高级启动”：团队名册文件、打开时询问、自定义数据库路径、恢复开关、更新检查开关、离线假代理，以及需明确风险确认的关闭审批门调试模式。debug/fake 仅对本次启动生效。
- 更新检查改用 rustls，Linux 包构建不再依赖 OpenSSL。
- `ASTERLINE_DESKTOP_FAKE=1` 在 debug 构建下直接使用离线假代理，便于冒烟测试；release 构建忽略该变量且不持久化。

## 平台说明

- Windows、macOS（Apple 芯片 + Intel）、Linux 包由发布工作流构建；图标、配置路径、外部终端 attach、剪贴板图片读取（X11/Wayland）保留各自的平台适配。
- 完整的“能力 → GUI 入口 → 命令入口 → 测试”矩阵见 `docs/desktop-parity.zh-CN.md`。
