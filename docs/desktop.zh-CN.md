# Asterline Desktop V1

Asterline Desktop 是现有 `asterline` / `ast` 终端程序的图形化伙伴。Desktop
与 TUI 复用同一套运行时、团队配置、会话存储和工作区锁；同一个工作区不能同时
打开 Desktop 和 TUI。

## V1 范围

- 打开项目或从最近项目中选择。
- 向成员或整个团队发送消息，并实时展示运行时事件。
- 查看成员状态、切换模式、取消任务、处理审批。
- 查看和继续 Runs、编辑步骤清单、恢复本地历史。
- 编辑完整团队配置，保存时不会丢失高级或暂未显示的字段。
- 在外部终端打开成员原生 CLI。Codex、Claude 在终端退出后可导入历史；Grok、
  Agy 在 V1 中只负责打开终端，不承诺自动同步。
- 中英文切换、亮色和暗色主题。
- 手动检查 Desktop 新版本。

Desktop V1.1 还提供四个只读工具抽屉：

- `/logs` 查看运行时诊断日志，可按级别、来源和关键字筛选；单次最多返回 400 条，结果超过上限时会明确标记截断。
- `/diff` 查看当前工作区相对 Git HEAD 的暂存、未暂存和未跟踪文件变化；不会修改工作区，输出上限为 2 MiB，Git 命令最多运行 10 秒。
- `/skills` 查看当前工作区可发现的 Skills，可按后端和关键字筛选并复制调用语法；最多返回 512 项，绝不把本地绝对路径发送到 WebView。
- `/find` 搜索当前已经加载的会话时间线，不查询其他历史会话，支持匹配计数和前后导航。

工具数据按需通过版本化桥接请求，不写入 `DesktopSnapshotV1`。切换工作区、恢复会话或关闭运行时时，旧工具结果会被清除。

V1 暂不内嵌终端，也不采集遥测。

## 架构与安全边界

WebView 只负责显示。它向 Rust 桥接层发送带版本号的命令；桥接层校验后映射为
`UiCommand`。`RuntimeEvent` 也必须转换成带版本号的 wire DTO，才能发送回
WebView。WebView 不能直接写 SQLite、修改 `team.json` 或启动 provider CLI。

```text
React 工作台
    | 带版本号的 Desktop 命令/事件 DTO
Tauri Rust 桥接
    | RuntimeHandle / UiCommand / RuntimeEvent
Asterline runtime
    | 原子团队配置 + SQLite 会话状态
<workspace>/.asterline/
```

`team.json` 仍是可编辑的启动配置，SQLite 仍保存会话快照和追加式事件历史。
Desktop V1 的项目状态格式为 `1`，兼容窗口为当前及上一个 Desktop 次版本。

最近项目和界面偏好保存在应用本地；项目提示、回答、工具输出、审批与 provider
会话标识继续保存在工作区的 `.asterline` 中。Desktop V1 不发送遥测。应用会保存有上限的
本地诊断日志和异常退出标记，并允许用户从日志抽屉导出诊断文本。

## 从源码运行

先安装 Rust stable、Node.js 24、pnpm，以及当前平台对应的 Tauri 构建依赖。

```powershell
cd desktop
pnpm install --frozen-lockfile
pnpm dev
```

启动原生应用：

```powershell
cd desktop
pnpm tauri dev
```

质量检查：

```powershell
cd desktop
pnpm lint
pnpm test
pnpm test:e2e
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
```

没有 Tauri 桥接时，浏览器开发版会自动使用确定性的 mock 数据；它只用于开发
界面，不会读取或修改真实工作区。

## 安装包与发布

Desktop 使用独立的 `0.x` 版本线。`desktop-v<version>` 标签触发 Desktop 发布；
CLI 继续使用原有的 `v<version>` 标签。

发布覆盖 Windows x64、macOS Intel / Apple silicon、Linux x64 / ARM64：

- Windows：经过 Authenticode 签名的当前用户 Inno Setup 安装包和便携 EXE（安装包内置
  微软签名的 Evergreen WebView2 引导程序）。
- macOS：每种架构的 DMG 和便携 app 压缩包，必须使用 Developer ID 签名并完成
  公证；缺少签名凭据时发布会直接失败。
- Linux：在对应原生架构上构建 AppImage。

应用内只提供手动更新检查，只识别 `desktop-v*` 发布。发现更新后会展示当前与可用版本，
并且只允许在系统浏览器中打开 Asterline 官方 GitHub Release 页面；不会自动安装更新。

如果 Windows 系统可能尚未安装 WebView2，请使用安装包。便携 ZIP 不会安装系统
依赖。
