# 应用内离屏截图（`--capture`）规格

版本 v0.1 · 2026-11-26 · 状态：**规格，未实现**。实现排在主线 S1–S6 之后。

## 0. 这份规格解决什么

验证界面时现在靠"启动真窗口 + 抓图"，窗口会出现在用户桌上（见 `scripts/launch-quiet.ps1`：它把窗口放到独立桌面，已经做到看不见、不抢焦点，但仍依赖 DWM、依赖桌面会话，且每次抓图都要跑一个进程）。

`--capture` 的取舍正相反：**Green 自己离屏渲染一帧写 PNG 就退出**，不建可见窗口、不依赖桌面、不依赖"有没有人坐在电脑前"，因此在 CI 里也能跑。这条路径顺带让"同输入同输出"成为可能，才谈得上视觉回归。

术语：本文的"帧"指 gpui 的一次 layout+paint；"完备"指该帧里公式、图片、代码块都已排好，不是占位或源码。

## 1. 命令行

```text
green --capture out.png --theme dark --width 1400 --height 900 file.md
```

| 参数 | 默认 | 说明 |
| --- | --- | --- |
| `--capture <path>` | 无 | 出现即进入截图模式，路径为 PNG 输出；父目录不存在即报错，不自动创建 |
| `--theme <light\|dark>` | `light` | 与 `ELY_APPEARANCE` 同一套取值；`system` 在截图模式拒绝（没有桌面可问系统） |
| `--width <px>` / `--height <px>` | 1400 / 900 | 逻辑像素；PNG 尺寸 = 逻辑 × `--scale` |
| `--scale <n>` | `1` | 设备缩放。默认 1 而不是跟随显示器：离屏时没有显示器，跟随会让同一命令在两台机器上出不同尺寸 |
| `--full-page` | 关 | 见 §5；默认只截首屏 |
| `--max-height <px>` | 20000 | `--full-page` 的高度上限 |
| `--wait-ms <ms>` | 10000 | 等"渲染完备"的上限，见 §6 |
| `<file.md>` | 无 | 位置参数，恰好一个；零个或多个即报错 |

- 入口在 `apps/green/src/main.rs` 里 `remote::command_line()` 之前判定，参数由新模块 `apps/green/src/capture/`（`args.rs` + `run.rs`，各 ≤500 行）手写解析，照 `remote.rs` 现有风格，不引 clap。
- 截图模式**不取单实例锁**（`ely_platform::instance::hold`）、不建托盘、不 listen 转交、不 `cx.activate`、不写 `session.json`/`recent.json`。它读的是磁盘上的文件，不是用户的会话。
- 退出码：

| 码 | 含义 |
| --- | --- |
| 0 | PNG 写好了，且不是空白帧 |
| 2 | 参数错（含 `--capture` 不在这个构建里） |
| 3 | 这台机器没有可用的渲染后端 |
| 4 | `--wait-ms` 内没到完备 |
| 5 | 写盘失败，或截出来是空白帧 |

## 2. 与现有渲染路径怎么共用

今天的启动串（`crates/ely-ui/src/run.rs` 的 `open()`）：

```text
fonts::install → ely_gpui_component::init → settings::install → storage::clear_marked
→ theme::install → i18n::install → shell::want_scene()
→ main_window(WindowOptions, desk::open)   // apps/green/src/desk.rs 是 root
```

截图路径复用同一串安装与**同一个 root**（`desk::open`），只换三样：

1. `WindowOptions { show: false, focus: false, window_bounds: 请求尺寸 }`，其余（标题栏、客户端装饰、`app_id`）照 `main_window` 的构造。窗口存在，但从未显示，也不取焦点。
2. 数据根换成临时目录（`ELY_DATA_ROOT` 同一机制），跑完删掉。这一条来自 `launch-quiet.ps1` 的教训：不隔离就会撞上用户正在跑的实例，还会改到用户的会话与设置。
3. 不装 `updates`（会发网络请求），不 `tray::set_main`，不 `opening::home`。

写法上建议在 `ely-ui` 里把 `open()` 拆出"安装"（今天的第 1–7 步）与"开窗"（`main_window` + 收尾）两段，截图与交互共用"安装"；不在 `apps/green` 里复制一份安装代码（AGENTS.md 规则 1、3）。

模式：首版固定可视（visual）视图；`--mode source|visual` 留给后续，不在本规格的实现范围。

## 3. 字体（无窗口环境）

- **无窗口不等于无字体**。文本系统挂在 platform 上，不挂在窗口上：Windows 用 `WindowsPlatform::new(false)` 里的 DirectWrite；Linux 没带 `x11`/`wayland` 特性时 platform 给的是 `NoopTextSystem`（`vendor/gpui-pre-linux` 的 `platform.rs`），要么开其中一个特性、要么经 `HeadlessAppContext` 显式传 `CosmicTextSystem`；macOS 用 AppKit 的文本系统。
- 随包两份字体照 `ely_ui::fonts`：`bytes()` 交 `cx.text_system().add_fonts(...)` 给 GPUI，`book()` 交 Typst——GPUI 与 Typst 必须是同一批文件，否则公式与正文的度量会不一致。
- 字体目录仍按 `ely_ui::fonts::dir()`：exe 旁 `fonts/` 优先，否则源码树 `assets/fonts`。**找不到即失败退出**，不回落系统字体：回落会让同一命令在不同机器上出不同字宽，截图就不可比较了。
- 启动后自检一条：量一个已知字符串的宽度，`<= 0` 即退出码 3 并说明"文本系统没有量出字宽"。`NoopTextSystem` 的症状就是宽度为 0、PNG 一片背景色，静默成功比失败更糟。

## 4. 与 `ELY_SMOKE` 的区别

`ELY_SMOKE=1` 是"开窗画一帧就退"，用来量启动耗时；它仍然建真窗口。`--capture` 不建可见窗口、不量启动、产物是一张 PNG。两者不合并。

## 5. 页面长度：首屏 or 整页

- 默认（不带 `--full-page`）只出 `--width × --height` 的首屏，等于用户打开时看到的，也等于今天 `launch-quiet.ps1` 抓到的。
- `--full-page` 的机制：先用请求宽度、一次试探布局拿到内容总高 `H`，再用 `H` 开窗渲染一帧。`H > --max-height` 即退出码 4，报"文档太长"。
- **不能"滚着截再拼"**：gpui 的 `list` 只构建可见行；滚动还会改插入点、打字机居中与滚动条状态，两帧拼不到一起。所以整页只能靠一次高窗。
- 整页的代价写在明处：1400 × 20000 的 RGBA 纹理约 112 MB，外加一帧的中间目标；Windows 的 D3D11 纹理上限是 16384 px，超过就按竖直 tile 分块渲染（同一帧、只改渲染目标的 y 偏移，不滚动内容）再拼。默认关闭就是对这笔内存的态度。

## 6. "渲染完备"的判定（N 帧）

实测（本任务，2026-11-26，Windows）：**首帧之后 500 ms 抓图，公式还是 `$...$` 源码；同一份文档 2.5 s 抓到的是排好的字形。** 原因是公式位图在后台由 Typst 排，且文档打开后还有一次字体族扫描（日志 `diagram: 961 font faces`）落在首帧之后。

规则：

1. 开窗后跑帧，直到**连续 3 帧没有新的 repaint 请求**，且没有未完成的背景排版任务（公式、图片、Mermaid）。
2. 满足后再等一帧取图，避免取到"请求了但还没画"的那一帧。
3. `--wait-ms` 内不满足 → 退出码 4，**不写文件**。

不写死帧数、不 sleep。这是可重复的前提，也是今天窗口截图会"时好时坏"的根因。

## 7. 取图与落盘

- Windows：`gpui/test-support` 提供的 `Window::render_to_image()`，最终走 `gpui-pre-windows` 的 `DirectXRenderer::render_to_image`：渲染到离屏目标再读回，**不 Present**，与窗口是否可见无关。
- 落盘 PNG 不带 alpha（背景是主题底色），RGB 8 位；同机同参数两次运行应逐字节相同。
- 写完做一次空白检查（采样网格，颜色种类 ≤ 4 或单色占比 ≥ 99.5% 判空白）：空白即另存 `<out>.blank.png` 供诊断，并以 5 退出。

## 8. 失败与降级

| 现象 | 行为 |
| --- | --- |
| 没有 D3D / Metal / wgpu 设备 | 退出码 3，写明"这台机器没有可用渲染后端"，并把 `scripts/launch-quiet.ps1` 指给用户 |
| 本构建没开 `capture` 特性 | `--capture` 视为未知参数，退出码 2，不尝试退化成窗口截图 |
| 字体缺文件 / 文本系统量不出宽 | 退出码 3 / 4，不写 PNG |
| 文档读不到、非 UTF-8 且无法按 `text.rs` 的规则读 | 退出码 4，错误里带路径，不带正文 |
| 超时 | 退出码 4，不写 PNG（宁可没有，也不要一张像是"已经画完"的图） |

**禁止**：任何情况下静默写出一张黑图或半张图当成功（这是本任务 A 部分明确要求"如实报告"的同一条原则）。

## 9. 三端怎么拿渲染器

| 平台 | 怎么拿 | 备注 |
| --- | --- | --- |
| Windows | 真平台 + `show:false` 的窗口，`render_to_image()` | `gpui_platform::current_headless_renderer()` 在 0.3.14 的 Windows 分支返回 `Ok(None)`，所以**不能**照文档里那句跨平台示例走 `HeadlessAppContext`；`WindowsPlatform::new(true)` 又不能开窗（`cannot open windows while headless`） |
| Linux | `gpui_platform::headless()` + `current_headless_renderer()`（wgpu） | 不需要 X11 / Wayland，CI 直接跑 |
| macOS | `current_headless_renderer()`（Metal 渲染器） | 见 §12 |

三端同一个入口，差异只在"平台怎么给渲染器"与"怎么把文本系统换成真的"。

`render_to_image` 挂在 `gpui` 的 `test-support` 特性下（`Window::render_to_image` 的 `#[cfg]`）。取法两种，实现时二选一：

- **（推荐）** 给 `apps/green` 加 `capture = ["gpui/test-support"]`，截图只在带该特性的构建里存在；发行包不开，体积与代码面不变。
- 或在 `vendor/gpui-pre` / `vendor/gpui-pre-windows` 里把 `#[cfg(any(test, feature = "test-support"))]` 放宽到自己的 `capture` 特性。这是第 4 个 fork 点（见 `docs/design.md` §0.3 已有的三个），代价大于收益，除非将来截图要进发行包。

## 10. 验收（将来实现的退出条件）

1. 在**没有可见桌面**的会话里产出 PNG：Windows 用服务会话或 CI runner，Linux 用 `unshare` 出来的无 X 环境。
2. 同一份文档、同一主题：`--capture` 的 PNG 与 `scripts/launch-quiet.ps1` 的窗口截图内容一致——字、公式、代码块、列表都在。
3. 同一命令连跑两次，PNG 逐字节相同。
4. `--full-page` 的高度覆盖全文（与 `word count` 侧栏一致），不截断。
5. 无 GPU 时不写文件、退出码 3，日志里有原因。
6. CI：`cargo run -p green --features capture -- --capture out.png --theme dark doc.md`，不需要显示器，不需要用户登录。

## 11. 风险与取舍

- **和 A 的分工**：`launch-quiet.ps1` 今天就能用（依赖桌面与 DWM，实测可用）；`--capture` 是根治，但要动 `apps/`、加一个特性、并在 Windows 上先验证"`show:false` 的窗口能不能拿到 device 与首帧"。第一天就该验这一条，验不过就退回窗口方案，别在离屏上硬耗。
- **不要拿它做像素断言**：截图会因为真实 UI 变化而变，这是它的价值（人眼看得见），也是它的风险（断言脆）。要断言就先固定字体、缩放与主题，再只比内容区。
- **整页与内存**：见 §5，默认关。
- **公式时序**：见 §6，规则是"等到安静"，不是"多等一会"。

## 12. 备注：Mac 上的"不打扰"（跨平台时要用的知识）

Mac 比 Windows 容易，两条路都成熟：

- **不激活**：不调 `NSApplication.activate(ignoringOtherApps:)`；`NSApp.setActivationPolicy(.accessory)`（或临时 `.prohibited`）让进程既不抢焦点也不占 Dock；建窗用 `orderFrontRegardless()` 而不是 `makeKeyAndOrderFront(_:)`，窗口出现但不成为 key window。
- **真离屏**：`NSView.bitmapImageRepForCachingDisplay(in:)` + `cacheDisplay(in:to:)` 能把任意视图层级画进 `NSBitmapImageRep`，完全不需要窗口。但 GPUI 在 macOS 是把场景画进 `CAMetalLayer`，不是 NSView 的自绘，所以 Green 该走 gpui 的 `MetalHeadlessRenderer`（`gpui_platform::current_headless_renderer()` 的 macOS 分支），而不是 NSView 缓存。
- 结论：Mac 上"完全不可见 + 内容正确"本来就不难，因为 Metal 离屏渲染不需要窗口；Windows 上今天只能靠"独立桌面 + PrintWindow"绕过（见 `scripts/launch-quiet.ps1` 与同分支提交说明）。以上 macOS 结论未在真机上验证，实现时要重跑一遍。
