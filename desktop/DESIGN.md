---
name: Asterline Desktop
description: Continuous mature developer workbench with neutral themes and restrained green.
colors:
  accent: "#116b45"
  accent-dark: "#7ee2b8"
  action-bg: "#1f7a3a"
  action-bg-dark: "#238636"
  action-hover: "#196731"
  action-hover-dark: "#2c9140"
  on-action: "#ffffff"
  surface: "#ffffff"
  surface-dark: "#0d1117"
  panel: "#f6f8fa"
  panel-dark: "#161b22"
  surface-hover: "#e8ecf0"
  surface-hover-dark: "#2a323c"
  text: "#1f2328"
  text-dark: "#e6edf3"
  muted: "#59636e"
  muted-dark: "#a2acb8"
  border: "#d1d9e0"
  border-dark: "#343e4a"
typography:
  body:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "PingFang SC", "Noto Sans CJK SC", system-ui, sans-serif'
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "normal"
  message:
    fontFamily: '"Segoe UI Variable Text", "Segoe UI", "Microsoft YaHei UI", "PingFang SC", "Noto Sans CJK SC", system-ui, sans-serif'
    fontSize: "15px"
    lineHeight: 1.7
    letterSpacing: "normal"
  headline:
    fontSize: "20px"
    fontWeight: 600
    letterSpacing: "normal"
  label:
    fontSize: "14px"
    fontWeight: 600
    lineHeight: "20px"
  code:
    fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, "Liberation Mono", monospace'
    fontSize: "13px"
    lineHeight: 1.6
rounded:
  chip: "4px"
  control: "6px"
  field: "7px"
  card: "8px"
  composer: "10px"
  modal: "12px"
spacing:
  control-gap: "6px"
  group-gap: "8px"
  message-gap: "12px"
  settings-gap: "16px"
  timeline-gap: "24px"
components:
  button-primary:
    backgroundColor: "{colors.action-bg}"
    textColor: "{colors.on-action}"
    rounded: "{rounded.control}"
    padding: "0 14px"
    typography: "{typography.label}"
  button-primary-hover:
    backgroundColor: "{colors.action-hover}"
  button-primary-dark:
    backgroundColor: "{colors.action-bg-dark}"
    textColor: "{colors.on-action}"
  button-primary-dark-hover:
    backgroundColor: "{colors.action-hover-dark}"
  settings-input:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.field}"
    padding: "0 9px"
    height: "38px"
  conversation-item:
    rounded: "{rounded.control}"
    padding: "9px 8px"
  state-pill:
    rounded: "{rounded.chip}"
    padding: "3px 6px"
  run-card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.card}"
---

# Design System: Asterline Desktop

## Overview

**Creative North Star: "成熟的连续工作台"**

中性明暗主题、克制的绿色强调与清晰的系统 CJK 排版，为长时间阅读和操作提供稳定背景。左侧导航保留轻微磨砂，聊天、检查器和设置使用实色阅读区；视觉层级依靠分区、字重和细边线。

本文件于 2026-09-08 从已实现的 CSS 提取；依据为 `src/styles/tokens.css`、`components.css`、`responsive.css`，产品约束见 `PRODUCT.md`，本轮范围见 `../docs/desktop-ui-refactor.md`。YAML 中无后缀颜色为亮色，`-dark` 为暗色覆盖值；组件预览通过 CSS 变量跟随主题。

**Key Characteristics:**

- 连续三栏工作区，阅读内容优先。
- 中性明暗表面，绿色区分强调文字与实心操作。
- 仅左侧轻微磨砂，其他阅读区保持实色。
- 系统 CJK 字体，紧凑控件与宽松消息行距。

## Colors

### Primary

绿色强调文字使用 accent；实心操作使用独立的 action-bg 与 on-action，悬停切换 action-hover。暗色主题分别覆盖强调文字和按钮填充，不能互换两种角色。运行、选择和成功状态复用绿色。

### Neutral

surface 是聊天与实色卡片底色，panel 用于侧区和次级表面，surface-hover 区分可操作行。text、muted 与 border 分别承担正文、元信息与分区；暗色主题保留同一语义关系。

橙色用于等待与审批，红色用于失败与危险，蓝色与紫色还用于路由、推理及成员后端身份。完整语义颜色及浅色状态底在 tokens.css；这些是已有状态编码，不是额外品牌强调色。

**The Action Contrast Rule.** 绿色文字与实心按钮填充分别使用 accent 和 action-bg；按钮文字使用 on-action。

## Typography

正文和控件使用 YAML 中的系统无衬线栈；中文优先由 Microsoft YaHei UI 等 CJK 回退承接。代码使用独立等宽栈。没有装饰展示字体，也没有统一比例缩放字体体系。

消息使用 message（15px / 1.7）；普通界面使用 body。空状态标题使用 headline，设置与弹窗标题通常为 18px；紧凑按钮及下拉选项统一为 14px / 20px，模式按钮使用 500 字重，主操作使用 600。字段说明标签仍为 13px，元信息为 12px。字体尺寸设置尚未实现。

2026-09-09 控件清晰度修复：共享 Lucide 图标的数值尺寸至少 16px，并向上取偶数；默认视觉描边为 2px。只使用 Lucide 的 absoluteStrokeWidth 补偿，不叠加 CSS non-scaling-stroke。此规则保留小图标内部细节，不承诺消除正常抗锯齿或替代 Windows DPI 实机验收。

## Layout

桌面连续网格为 `256px minmax(0, 1fr) 292px`，无栏间间隙与外层留白。主阅读列最大宽度 860px，输入区最大宽度 804px；顶部工具栏高 64px。消息垂直间隔使用 timeline-gap，设置双列间隔使用 settings-gap。

窗口宽度不超过 1200px 时，左栏改为 240px，检查器成为宽 292px 的右侧浮层；不超过 860px 时左栏成为宽 256px 的浮层。640px 以下，工具栏换行、设置改为纵向结构、消息和输入区收窄内边距。粗指针下常用图标按钮至少 44px。折叠栏可恢复，关闭侧栏由应用设置 inert。

当前宽度固定；拖动宽度、宽度记忆、字体尺寸及信息密度设置属于后续工作。

## Elevation & Depth

主区与成员列表以实色、分隔线和色阶建立层级。输入框使用轻阴影；弹窗、抽屉与窄窗侧栏使用浮层阴影。完整阴影值保存在 sidecar 扩展，直接提取自 CSS。

**The Sidebar Material Rule.** 只有左侧导航使用背景磨砂；不得对阅读文字施加滤镜、父级半透明或缩放效果。

侧栏材质为应用内 backdrop-filter（blur 18px、saturate 110%），亮色背景 alpha 为 .82、暗色为 .84。主内容实色；不支持 backdrop-filter、降低透明度偏好或强制颜色模式时回退实色。它不是 Windows 原生 Acrylic，也不透出桌面壁纸。Windows WebView 在系统 DPI 100% / 125% / 150% / 200% 下仍待实机验收。

## Shapes

控件轻圆角、卡片适度圆角、弹窗更圆，使用 YAML 中对应角色。主工作区、侧栏和成员行保持直角连续边界；成员行以底部分隔线连接。状态点和头像中的圆形只承担身份或状态意义。

## Components

- **Buttons:** 主操作为绿色填充，最小高度 36px；次要按钮为实色底与较强边框，悬停使用中性色阶。禁用按钮 opacity 为 .5，光标为 not-allowed。键盘焦点使用 2px accent 轮廓、1px 偏移和 3px 淡绿色光环。
- **Inputs / Fields:** 设置输入高 38px、字号 14px，细边框与实色背景。聚焦边框变为 accent，并保留键盘焦点轮廓；禁用字段使用 panel-muted。项目路径错误字段已有红色边框，不推导为所有输入的通用错误实现。
- **Navigation:** 会话行最小高 58px，悬停使用 surface-hover；选中保留同底色，增加左侧 2px 内阴影标记。检查器标签用底部绿色线标记当前页。
- **Chips:** 状态标签使用 12px 半粗体与小圆角；运行绿色、等待和审批橙色、失败红色。纯状态标签不添加虚构的悬停或点击行为。
- **Cards / Containers:** Runs 与工具卡使用实色底、细边框、card 圆角，静止时无阴影。Runs 标题是可聚焦按钮，悬停改变底色。成员条目使用平面分隔线。
- **Composer:** 实色面板、composer 圆角与轻阴影；focus-within 使用单一的圆角绿色外框，textarea 不再叠加独立焦点框。textarea 为 15px / 1.6；发送按钮高 32px，停止状态使用红色语义。

常规图标控件过渡为 .15s ease，方向箭头为 .16s / .18s；侧栏和弹窗不做装饰进入动画。降低动态效果偏好将动画与过渡缩至 .01ms。

## Do's and Don'ts

### Do:

- Do 使用真实主题语义变量，让明暗模式共享组件结构。
- Do 保持消息可读、键盘焦点可见和折叠栏可恢复。
- Do 为侧栏磨砂保留实色降级。

### Don't:

- Don't 在主阅读区扩展玻璃材质或使用文字缩放效果。
- Don't 添加装饰字体、动画背景或虚构运行状态。
- Don't 把计划中的宽度、字体或密度设置描述为已实现。
