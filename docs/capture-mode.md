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
| `--still-ms` | 2000 | 画面要连续不变这么久才算画完。慢机器或大文档的排版间隙超过它就会截到未渲染的源码，那时把它调大 |
| `--wait-ms` | 15000 | 最多等画面稳定这么久；超过即失败，不拿最后一帧充数 |
| `<file.md>` | 无 | 恰好一个位置参数 |

退出码：0 写好 PNG；2 参数错（含"这个构建没有 capture"）；3 这台机器没有可用渲染后端；4 窗口没画出来；5 写盘失败或截到空白帧（空白帧改存 `<out>.blank.png`）。

数据目录用 `%TEMP%\green-capture-<pid>`，跑完删除：不读用户的设置、不碰用户正在跑的实例、不写 `session.json` 到用户目录。

## 2. 怎么实现的（Windows）

1. **入口**：`apps/green/src/main.rs` 在 `remote::command_line()` 之前认 `--capture`，交给 `apps/green/src/capture.rs`（解析参数、设环境变量），再由 `crates/ely-ui/src/capture.rs` 跑。
2. **窗口**：走与正常启动同一串安装（字体、组件库、设置、主题、i18n、键位、`shell::want_scene()`），同一个 root（`desk::open`），只把 `WindowOptions` 换成 `show: false, focus: false`，尺寸取 `--width/--height`。文件经 `opening::install_launch_files` 交给 root，和正常启动的 `launch_files` 是同一条路。
3. **帧从哪来**：平台的帧循环**不会**给没显示过的窗口画帧（第一版只截到启动画面与背景色）。所以截图的帧自己画：`window.draw(cx)` 之后用 `Window::render_to_image()` 交给 DirectX 渲染器离屏读回，隔 100 ms 再画下一帧。
4. **启动画面要撤**：`shell` 的启动画面在 `startup::started()` 之后才离开。正常启动由平台的"下一帧回调"打这个标记；截图路径在开窗后直接调用 `ely_platform::startup::started()` 并 `refresh()`，画面才会换成文档。
5. **收敛判定**（替掉固定等待）：每帧记下像素，**画面连续 2 秒不变**即取这一帧落盘——前提是文档已画出来（`Desk::shown`：首个标签的视图报过显示）。超过 `--wait-ms`（默认 15 s）仍不稳定即 `exit 4`，**不写文件**，绝不拿最后一帧充数。
   为什么不是"两帧相同"：一开始就是这条规则，它挡不住一种假收敛——文档出来后、Typst 的公式落地前，帧确实一模一样两三次，于是截到的是**公式的 LaTeX 源码**（实测到过一次，见 §4）。这段等待在帧状态里完全看不见（gpui 不对外暴露"后台还有排版在跑"），所以用一段不短于实测间隙的静默时间来代替。

   **这是判据的边界，不是 bug**：某台机器或某份文档的排版间隙真的超过 `--still-ms`，就会截到源码（一张合法、看着也像回事的 PNG）。遇到时把 `--still-ms` 调大，不必改代码重编。日志会写明实际等了多少毫秒、画了多少帧（`the picture stood still for 2000 ms over 12 frames; the run took 2630 ms`），能看出是"刚好够"还是"差一点"。
6. **指针要离开窗口，而且要让它知道**：文档里的每个块都是带 `.hover()` 的 div，指针压在哪个块上，那个块就画一层 `colors.hover` 高亮。Windows 的 `mouse_position()` 是 `GetCursorPos` + `ScreenToClient`，没显示的窗口同样算数；更麻烦的是 gpui 的指针位置**只在 `bounds_changed` 时向平台重读**，其它时候用缓存。于是光把窗口移走不够：第一帧读到的还是"窗口还在指针下面"时的坐标，之后一直沿用——这正是"同一命令 8 次里 2 次多出一块灰底"的根因（每次运行鼠标位置不同，压中的块也不同）。
   修法：把窗口移到 `(-32000, -32000)`（`SetWindowPos`，不显示、不改尺寸），然后**在开头几帧再 `bounds_changed` 一次**，让 gpui 重新读到"指针在窗口外"（实测读出 `16259,16218`，窗口只有 `1187×689`）。移窗到生效有延迟，第一帧读到旧值，所以这一步要跨帧做。
7. **离屏读回靠基座**：`Window::render_to_image()` 在 gpui 的 `test-support` 特性下，Windows 侧接到 `WindowsWindow` 的 `render_to_image`（D3D 渲染器渲染到离屏目标再读回，**不 Present**）。因此 `ely-ui` 有 `capture` 特性（`gpui/test-support` + `gpui_windows/test-support`），`green` 有 `capture = ["ely-ui/capture"]`；发行包不开，体积与代码面不变。

```text
cargo run -p green --features capture -- --capture out.png --theme dark doc.md
```

## 3. 字体

文本系统挂在 platform 上，不挂在窗口上：Windows 用 `WindowsPlatform::new(false)` 的 DirectWrite；随包两字体照旧交给 `cx.text_system().add_fonts(...)`（GPUI）与 `fonts::book()`（Typst），找不到即失败退出，不回落系统字体。Linux 没带 `x11`/`wayland` 特性时 platform 给的是 `NoopTextSystem`（`vendor/gpui-pre-linux` 的 `platform.rs`），量出的字宽为 0，画出来是纯背景色——真要在 Linux 上跑，得显式给 `CosmicTextSystem`，并在开头量一个字宽自检。

## 4. 实测（2026-11-26，Windows 11 build 26200，debug 构建）

| 项 | 结果 |
| --- | --- |
| 一次的时间 | `tests/fixtures/capture.md`（2 条行内、2 条块级公式）空闲时 4.4–4.8 s、11–13 帧；高负载下一次可到 8.7 s |
| 高负载 8 次 | 并行 `cargo check` 把 CPU 压到 84–87% 时连跑 8 次，8/8 同一哈希 `905A8929…`（11–12 帧，4.4–8.7 s） |
| 内容 | 标题、正文、代码块、列表、**行内与块级公式**（Typst 已排好）都在 |
| 同一命令多次 | PNG 逐字节相同（SHA-256 `905A8929…`，连跑 5 次与换用收敛判定前后都不变） |
| 超时 | `--wait-ms 400` → `exit 4`、"no frame settled within 400 ms (2 frames drawn)"，**没有写文件** |
| 前台窗口 | 启动前 / 运行中 / 结束后同一个 `GetForegroundWindow()` |
| 屏幕上 | 用户视角截屏里没有它的窗口；`EnumWindows` 看得到 `class=Zed::Window visible=False`，全程 `visible` 一次都没有 |
| 亮/暗主题 | 都正确（背景 `#FCFAF7` / `#252525`） |
| 鼠标位置 | 修好后无影响：日志里指针始终在窗口外（`16259,16218` 对 `1187×689`），8 次连跑同一哈希。修前实测 8 次出 3 种哈希，差异都是某一块多了一层 `colors.hover` 灰底 |

> 注意：debug 构建是控制台程序（`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`），从一个没有控制台的父进程启动会**新开一个控制台窗口**并可能抢焦点。那是控制台，不是 Green 的窗口；release 构建没有这个问题，脚本化调用也可以用 `CreateNoWindow`。

## 5. 还没做

- **整页**（`--full-page`）：现在只出首屏，参数传了即 `exit 2`。整页要等主线之后做，记在 `later.md`。
- **源码模式**（`--mode source|visual`）：现在固定可视。
- **图片 / Mermaid 的完备**：收敛判定只保证"画面不再变"，不保证"该出现的都出现了"。文档里的图片和 Mermaid 若在 600 ms 之后才开始变，会先被当成稳定；600 ms 这个下限就是用来压低这类误判的，不是证明。
- **CI 接线：验证过，不接。** 手动触发 + main 的作业跑通了（GitHub 的 `windows-latest`，11 分 41 秒，`--capture` 退出 0，指针读数 `32504,32384` 说明移窗修复在 runner 上同样生效），但 **runner 的视口是 752×511，本机是 1187×689**，PNG 的哈希因此不同（`37E4A478…` 对 `905A8929…`）：同一份逻辑尺寸的请求在两台机器上给出不同的逻辑视口，排版就不同。所以这个作业已经删掉——会被环境改变的基线比没有基线更糟。要接 CI，前提是先让视口与缩放固定（规格里的 `--scale`，默认 1 而不是跟随显示器），再加上 WARP 与本机 GPU 的字形栅格化差异另行核对。

## 5.1 平台范围

v1 只投 Windows。macOS 与 Linux 的这条路径**没有编译过、也没有跑过**：`gpui/test-support` 的 `render_to_image` 在三端都声明了，但 macOS 的 Metal 与 Linux 的 wgpu 离屏渲染器都还没在这里验证；Linux 还缺真文本系统（见 §3）。不为此开机器验证——等真要出非 Windows 版再补，别把一个暂时不需要的结论当成已验证。

## 6. 和 `scripts/launch-quiet.ps1` 的分工

- `--capture`：默认工具。不显示窗口，可重复，适合 CI 与视觉回归。
- `launch-quiet.ps1`：兜底。要截的是**真窗口**（比如看窗口装饰、拖拽、真实输入焦点）时用它：`ELY_SMOKE=hold` 不激活 + `SW_SHOWNOACTIVATE` + 推到底层 + `PrintWindow`。它会在用户当前的虚拟桌面上留下一个窗口，因此只在没有 `--capture` 能表达的场景里用。

## 7. 备注：Mac 上的"不打扰"

- **不激活**：不调 `NSApplication.activate(ignoringOtherApps:)`；`NSApp.setActivationPolicy(.accessory)` 让进程不抢焦点、不占 Dock；建窗用 `orderFrontRegardless()` 而不是 `makeKeyAndOrderFront(_:)`。
- **真离屏**：`NSView.bitmapImageRepForCachingDisplay(in:)` + `cacheDisplay(in:to:)` 能把任意视图层级画进 `NSBitmapImageRep`，不需要窗口；但 Green 在 macOS 是把场景画进 `CAMetalLayer`，所以该走 gpui 的 `MetalHeadlessRenderer`（`gpui_platform::current_headless_renderer()` 的 macOS 分支），与 §2 的 Windows 路径同构。
- 结论：Mac 上"完全不可见 + 内容正确"本来就不难，因为 Metal 离屏渲染不需要窗口；未在真机上验证。

## 8. Windows 事实（实测，勿重跑）

2026-11-26，Windows 11 build 26200，本机。这四条已经跑过，别再花时间验证：

1. **新建窗口落在启动那一刻活动的虚拟桌面上，之后不动。** 同一台机器三次启动时用户都在第 2 个桌面（index 1），窗口就落在 index 1（`IsWindowOnCurrentVirtualDesktop=1`）；更早一次当前在 index 0，窗口同样落在 index 0。所以"用户在桌面二干活、Green 在桌面一跑"只在**窗口创建时用户还在桌面一**的情况下成立：创建后再切走，窗口留在原处；创建时用户已在桌面二，窗口就出现在他那边（`ELY_SMOKE=hold` 仍不抢焦点）。
2. **两帧相同不等于画完。** 文档出现后、公式落地前的帧可以完全一样（实测截到过 LaTeX 源码），因为 gpui 不对外给"后台还有排版"的信号。所以判据是"连续 2 秒不变"，不是"两帧相同"。
3. **指针位置会改变画面。** 文档块带 `.hover()`，指针压着哪个块，那个块就多一层 `colors.hover` 半透明底；`WindowsWindow::mouse_position()` 用 `GetCursorPos` + `ScreenToClient`，没显示的窗口也算数，而且 gpui 只在 `bounds_changed` 时重读它。截图必须先移窗、再让 gpui 重读指针（见 §2 第 6 条），否则同一命令会写出不同的 PNG。
4. **`MoveWindowToDesktop` 不能搬别的进程的窗口。** 对 green.exe 的窗口返回 `E_ACCESSDENIED`（对本进程自己的窗口可以，返回 `S_OK` 且 `IsWindowOnCurrentVirtualDesktop` 随之为 0）。读的一侧跨进程可用：`IsWindowOnCurrentVirtualDesktop`、`GetWindowDesktopId`；桌面清单从 `HKCU\...\Explorer\VirtualDesktops\VirtualDesktopIDs`（显示顺序，每 16 字节一个 GUID）与 `CurrentVirtualDesktop` 读，不需要 COM。

所以"事后把窗口搬回桌面一"这条路不成立；未公开接口的 IID 随版本变，不硬码。这也是 `--capture` 成为唯一干净答案的原因。
