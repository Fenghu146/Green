# 应用内离屏截图（`--capture`）

版本 v0.2 · 2026-11-26 · 状态：**已实现（Windows 实测）**，排在主线之后的验证工具，不进发行包。

## 0. 它解决什么

验证界面时有两条路，只有这一条不打扰人：

| 做法 | 焦点 | 用户看得见吗 | 截图 |
| --- | --- | --- | --- |
| 前台启动真窗口 | **抢** | 看见 | 屏幕截屏 |
| `ELY_SMOKE=hold` + `scripts/launch-quiet.ps1` | 不抢 | **窗口会出现在用户当前的虚拟桌面上**（实测：新窗口跟随活动虚拟桌面） | `PrintWindow` |
| **`green --capture`** | 不碰 | **看不见**：窗口从不显示（`visible=False`） | Green 自己离屏渲染后读回像素 |

`--capture` 不依赖桌面、不依赖虚拟桌面、不需要有人坐在电脑前，因此在 CI 里也能跑；同一份文档同一主题，两次运行**逐字节相同**（实测 SHA-256 一致），这才谈得上视觉回归。

## 1. 命令行

```text
green --capture out.png [--theme light|dark] [--width N] [--height N] [--wait-ms N] file.md
```

| 参数 | 默认 | 说明 |
| --- | --- | --- |
| `--capture <path>` | 无 | 出现即进入截图模式；父目录不存在即报错，不自动创建 |
| `--theme light\|dark` | `light` | 没有桌面可问系统，`system` 直接拒绝 |
| `--width` / `--height` | 1400 / 900 | 逻辑像素；PNG 尺寸 = 逻辑 × 平台缩放（本机 170% → 2374×1378） |
| `--wait-ms` | 2500 | 开窗后等文档排完再取图的时间 |
| `<file.md>` | 无 | 恰好一个位置参数 |

退出码：0 写好 PNG；2 参数错（含"这个构建没有 capture"）；3 这台机器没有可用渲染后端；4 窗口没画出来；5 写盘失败或截到空白帧（空白帧改存 `<out>.blank.png`）。

数据目录用 `%TEMP%\green-capture-<pid>`，跑完删除：不读用户的设置、不碰用户正在跑的实例、不写 `session.json` 到用户目录。

## 2. 怎么实现的（Windows）

1. **入口**：`apps/green/src/main.rs` 在 `remote::command_line()` 之前认 `--capture`，交给 `apps/green/src/capture.rs`（解析参数、设环境变量），再由 `crates/ely-ui/src/capture.rs` 跑。
2. **窗口**：走与正常启动同一串安装（字体、组件库、设置、主题、i18n、键位、`shell::want_scene()`），同一个 root（`desk::open`），只把 `WindowOptions` 换成 `show: false, focus: false`，尺寸取 `--width/--height`。文件经 `opening::install_launch_files` 交给 root，和正常启动的 `launch_files` 是同一条路。
3. **帧从哪来**：平台的帧循环**不会**给没显示过的窗口画帧（第一版只截到启动画面与背景色）。所以截图的帧自己画：`window.draw(cx)` 连续 6 次、每次隔 250 ms，最后一次的 scene 用 `Window::render_to_image()` 交给 DirectX 渲染器离屏读回。
4. **启动画面要撤**：`shell` 的启动画面在 `startup::started()` 之后才离开。正常启动由平台的"下一帧回调"打这个标记；截图路径在开窗后直接调用 `ely_platform::startup::started()` 并 `refresh()`，画面才会换成文档。
5. **离屏读回靠基座**：`Window::render_to_image()` 在 gpui 的 `test-support` 特性下，Windows 侧接到 `WindowsWindow` 的 `render_to_image`（D3D 渲染器渲染到离屏目标再读回，**不 Present**）。因此 `ely-ui` 有 `capture` 特性（`gpui/test-support` + `gpui_windows/test-support`），`green` 有 `capture = ["ely-ui/capture"]`；发行包不开，体积与代码面不变。

```text
cargo run -p green --features capture -- --capture out.png --theme dark doc.md
```

## 3. 字体

文本系统挂在 platform 上，不挂在窗口上：Windows 用 `WindowsPlatform::new(false)` 的 DirectWrite；随包两字体照旧交给 `cx.text_system().add_fonts(...)`（GPUI）与 `fonts::book()`（Typst），找不到即失败退出，不回落系统字体。Linux 没带 `x11`/`wayland` 特性时 platform 给的是 `NoopTextSystem`（`vendor/gpui-pre-linux` 的 `platform.rs`），量出的字宽为 0，画出来是纯背景色——真要在 Linux 上跑，得显式给 `CosmicTextSystem`，并在开头量一个字宽自检。

## 4. 实测（2026-11-26，Windows 11 build 26200，debug 构建）

| 项 | 结果 |
| --- | --- |
| 一帧的时间 | 5–7 s（启动 1 s + `--wait-ms` 2.5 s + 6 帧 1.5 s） |
| 内容 | 标题、正文、代码块、列表、**行内与块级公式**（Typst 已排好）都在 |
| 同一命令两次 | PNG 逐字节相同（SHA-256 一致） |
| 前台窗口 | 启动前 / 运行中 / 结束后同一个 `GetForegroundWindow()` |
| 屏幕上 | 用户视角截屏里没有它的窗口；`EnumWindows` 看得到 `class=Zed::Window visible=False`，全程 `visible` 一次都没有 |
| 亮/暗主题 | 都正确（背景 `#FCFAF7` / `#252525`） |

> 注意：debug 构建是控制台程序（`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`），从一个没有控制台的父进程启动会**新开一个控制台窗口**并可能抢焦点。那是控制台，不是 Green 的窗口；release 构建没有这个问题，脚本化调用也可以用 `CreateNoWindow`。

## 5. 还没做

- **整页**（`--full-page`）：现在只出首屏。整页要先量内容高度、再开一扇够高的窗（gpui 的 `list` 只建可见行），Windows 的 D3D11 纹理上限 16384 px，超了要竖直分块。参数先不接受，传了即 `exit 2`。
- **源码模式**（`--mode source|visual`）：现在固定可视。
- **等到"安静"再截**：gpui 不对外给"这帧之后没有新请求"的信号，现在是固定 `--wait-ms` + 6 帧；文档里的图片、Mermaid 慢到超过这个时间就会截到未完成的一帧。
- **CI 接线**：GitHub Actions 的 Windows runner 有 WARP（软件 D3D），理论可跑；未在本仓库的 CI 上验证。

## 6. 和 `scripts/launch-quiet.ps1` 的分工

- `--capture`：默认工具。不显示窗口，可重复，适合 CI 与视觉回归。
- `launch-quiet.ps1`：兜底。要截的是**真窗口**（比如看窗口装饰、拖拽、真实输入焦点）时用它：`ELY_SMOKE=hold` 不激活 + `SW_SHOWNOACTIVATE` + 推到底层 + `PrintWindow`。它会在用户当前的虚拟桌面上留下一个窗口，因此只在没有 `--capture` 能表达的场景里用。

## 7. 备注：Mac 上的"不打扰"

- **不激活**：不调 `NSApplication.activate(ignoringOtherApps:)`；`NSApp.setActivationPolicy(.accessory)` 让进程不抢焦点、不占 Dock；建窗用 `orderFrontRegardless()` 而不是 `makeKeyAndOrderFront(_:)`。
- **真离屏**：`NSView.bitmapImageRepForCachingDisplay(in:)` + `cacheDisplay(in:to:)` 能把任意视图层级画进 `NSBitmapImageRep`，不需要窗口；但 Green 在 macOS 是把场景画进 `CAMetalLayer`，所以该走 gpui 的 `MetalHeadlessRenderer`（`gpui_platform::current_headless_renderer()` 的 macOS 分支），与 §2 的 Windows 路径同构。
- 结论：Mac 上"完全不可见 + 内容正确"本来就不难，因为 Metal 离屏渲染不需要窗口；未在真机上验证。

## 8. 为什么不是虚拟桌面

实测（同机、同日）：**后台进程新建的窗口落在启动那一刻活动的虚拟桌面上**（用户在第 2 个桌面时启动，窗口就出现在第 2 个桌面），并在之后停留在那里。而公开的 `IVirtualDesktopManager` 只能**读**（`IsWindowOnCurrentVirtualDesktop`、`GetWindowDesktopId` 跨进程可用），**不能把别的进程的窗口搬到指定桌面**（`MoveWindowToDesktop` 对 green.exe 的窗口返回 `E_ACCESSDENIED`，对自己的窗口可以）。所以"工作留桌面一、用户去桌面二"这个安排只在窗口创建时用户还在桌面一时成立，不能靠事后搬窗口补救；未公开接口的 IID 随版本变，不写死。这也是 `--capture` 成为唯一干净答案的原因。
