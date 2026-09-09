# Desktop UI 重构：连续工作台

## 范围

2026-09-08 第一阶段：替换全局玻璃、森林色背景与装饰动效，建立中性明暗主题、清晰排版、连续三栏结构。用户追加：左侧栏保留类似 Codex 的轻微磨砂半透明效果。

本轮不变更 TUI/运行时协议，不重新声明最新上游功能已完全对齐。也不启用 Windows 原生 Acrylic 或透出桌面壁纸。

## Direction contract

- THESIS: A calm, continuous developer workbench that keeps team conversations readable.
- OWN-WORLD: Asterline's existing project/session navigation, member identities, approvals and run controls; green denotes action and state.
- STORY: Choose a project and session, read and send work centrally, inspect members and runs alongside it.
- FIRST VIEWPORT: Navigation / conversation / inspector, compact utilities, anchored composer.
- FORM: Neutral light and dark surfaces; only navigation has subtle backdrop frost. System sans with CJK fallbacks; code uses monospace.
- FINISH: No loss of existing controls; text at least 12px, message body 15px; keyboard focus, collapsed-panel inertness, responsive layouts and solid-material fallback.

## 成熟项目参考

- [VS Code workbench UX](https://code.visualstudio.com/api/ux-guidelines/overview)：连续工作区和分工明确的侧栏。
- [Primer semantic colors](https://www.primer.style/product/primitives/color/)：语义色和前景/背景分离。
- [GitHub VS Code themes](https://github.com/primer/github-vscode-theme)：中性明暗色参考，不逐值复制。
- [Linear UI refresh](https://linear.app/changelog/2026-03-12-ui-refresh)：统一工具条、控件和信息层级。

## 本轮实现

- 拆分主题 tokens、单一组件规则、响应式和材质规则；不再叠加旧 Edition 02 装饰覆盖层。
- 只在侧栏使用 backdrop-filter；侧栏文字没有 filter、父级透明度或文字变换。背景透明度由主题 token 控制。
- 主聊天、成员、设置、运行面板均为实色；支持 reduced-transparency、forced-colors 和不支持 backdrop-filter 的实色降级。
- 消息 15px、标题 18–20px、控件 13–14px、元信息不低于 12px。
- 移除鼠标跟随背景、噪点、旋转成员卡、巨大文字水印、虚构的静态 LOCAL / READY。
- 保持折叠恢复；关闭的侧栏设置 inert，不能被 Tab 聚焦。
- Runs 的 Escape 关闭与关闭按钮行为一致，不影响正在运行的任务。

## 验证和边界

视觉证据保存在 output/playwright/workbench-*.png。浏览器演示数据不是实际工作区状态。

已截图检查明暗主题、1440×900、960×640、390×844、设置、Runs 和审批。构建通过，46 项单元测试通过，最终 9 项端到端测试通过（含暗色主界面、命令面板和 Runs 的 axe 扫描）。

浏览器计算样式：侧栏 rgba(22,27,34,.84)，opacity 为 1、filter 为 none；背景 backdrop-filter 为 blur(18px) saturate(1.1)。消息 15px / 25.5px；已检查主界面无小于 12px 的可见文字。390px 窗口的 document scrollWidth 为 390px。

模拟 prefers-reduced-transparency: reduce 后，侧栏变为不透明 rgb(22,27,34)，backdrop-filter 为 none。Runs 在成员忙碌时的 Escape 关闭也通过浏览器操作检查。

Impeccable 自动检测器缺少 engine binary，未能运行；没有将其标记为通过。独立收尾审查已查看 9 张截图、概念参考和相关代码，结论为 ship；在当前证据范围内没有阻塞交付的问题。非阻塞后续项为常规设置说明去重、Windows DPI 实机验收和极端长内容覆盖。

原生 Windows WebView 在 100% / 125% / 150% / 200% 系统缩放下的清晰度仍需实机检查；浏览器截图不能代替 DPI 验收。

## 后续阶段

### 2026-09-09：小控件清晰度修复

用户反馈上一阶段小按钮模糊。实际预览为 DPR 1、100% 缩放，按钮无 filter、opacity 或 transform 模糊；共享 SVG 同时启用 Lucide absoluteStrokeWidth 与子路径 non-scaling-stroke，导致重复补偿。原 13px 图标实际描边约 3.32px，小图标细节拥挤。

- 删除 CSS 的重复线宽补偿，默认视觉描边统一为 2px；数值图标尺寸至少 16px，向上取偶数。
- 紧凑按钮及下拉选项使用 14px / 20px 控件 token；模式按钮高 32px、字重 500。保持侧栏磨砂和其他区域实色。
- 先生成 desktop-control-clarity-concept.png，再修改 SVG/CSS；提示词保存在同目录 .prompt.md。图仅作方向参考。
- 构建通过，56 项单元测试及 9 项端到端测试通过。图标单测覆盖 9 个尺寸及显式线宽；浏览器测试校验路径 vectorEffect 为 none、最终描边为 2px，防止 CSS 回归。
- 在新版内置浏览器检查 1280×720 暗色主界面和 526×698 明暗主题；526px 窗口 scrollWidth 为 526px，下拉选项计算字号为 14px / 20px。预览服务已重启，避免旧页面未加载新版造成误判。
- 此次不改系统 ClearType、GPU 或 WebView 参数；原生 DPI 验收仍待完成，未重新打包 Windows 安装程序。

### 2026-09-09：输入区边框修复

- 用户截图中的横向绿色边框来自全局 `textarea:focus-visible`：它在已有 composer `focus-within` 外框内部再次绘制了方形 outline 和 shadow。
- composer 继续承担唯一的 10px 圆角焦点边界；内部 textarea 清除自己的 outline 和 shadow，键盘焦点仍通过外框清晰可见。
- 修复前先生成 `desktop-composer-focus-border-concept.png`，提示词保存在同目录 `.prompt.md`；参考图不作为应用位图资源。
- 生产构建、56 项单元测试和 10 项端到端测试通过；新增浏览器断言覆盖单一焦点边界。内置浏览器暗色主题实测 textarea 为 `outline: none`、`box-shadow: none`，外框保持 1px border + 1px focus shadow。

### 后续规划

1. 可拖动侧栏宽度、宽度记忆、字体尺寸和信息密度设置，三者独立。
2. 设置、模式和运行流程的信息架构重整，以及底部日志 / Diff 面板。
3. 长时间流式响应、长会话与高频事件的性能测试。
4. 如需透出桌面壁纸，另行评估 Windows 原生材质、窗口配置和系统兼容性。

### 2026-09-09：输入区边框修复

- 根因是全局 `textarea:focus-visible` 与 `.composer:focus-within` 同时绘制焦点框；textarea 的直角轮廓穿过组件中部，形成双层边框。
- 输入区改为由 composer 外壳统一绘制 10px 圆角焦点边界，内部 textarea 不再单独绘制 outline 和 box-shadow；键盘焦点仍清晰可见。
- 已先生成 `docs/assets/desktop-composer-focus-border-concept.png`，作为单一外框结构参考，不作为应用截图或位图控件。

## 生图记录

采用内置 imagegen，模式 generate / ui-mockup。先生成再修改前端。
视觉参考：docs/assets/desktop-workbench-themes-concept.png；完整提示词见同目录 desktop-workbench-themes-concept.prompt.md。
此图仅作视觉方向参考，不作为界面截图，也不作为新增功能清单。
