# Green

Green 是一个离线的 Markdown 笔记应用：文件留在你自己的目录里，没有账号、没有云、没有自有格式。界面用 Rust + GPUI，公式与导出用 Typst。

设计以 [`docs/design.md`](docs/design.md)（v0.4）为准；`docs/design-v0.1.md`–`v0.3.md` 是历史存档，`docs/base/` 是基座 ElyOffice 的原始文档。

## 进度

**S1 基座与外壳**：剪枝后的 ElyOffice 基座可构建；多标签；工具条；正文居中（48rem）；浅/深两套主题；随包 Noto Sans CJK SC（GPUI 与 Typst FontBook 读同一批文件）。

S2–S6 未开始：Markdown 逐段渲染与折叠/大纲、行内公式、文件安全、插件、导出、打包。

## 构建与运行

需要 Rust（`rust-toolchain.toml` 钉版本）与 Windows 的 C++ 生成工具（MSVC）。

```sh
cargo build -p green
target/debug/green.exe path/to/note.md
```

- 构建产物在仓库自己的 `target/`（见 `.cargo/config.toml`），不与别的项目共用。
- `ELY_SMOKE=1` 画一帧即退出；`ELY_LOG` 设日志级别；`ELY_APPEARANCE=light|dark|system` 覆盖主题。
- 开发构建从 `assets/fonts/` 读字体；打包后从可执行文件旁的 `fonts/` 读。
- 应用数据在 `%LOCALAPPDATA%\Green\green\`：日志 `ely.log`、设置 `settings.json`、会话 `session.json`。

## 扩展语法

标准 Markdown 之外，Green 只认一条：

| 写法 | 效果 |
| --- | --- |
| `==文字==` | 高亮。文件里原样保留 `==文字==`，不改写；表达式的 Typst 侧对应 `#highlight[...]` |

`==` 不是 CommonMark/GFM 语法：用 Typora、Obsidian 以外的编辑器打开会看到原始 `==`。这是有意为之——Green 不往文件里写自有标记，折叠、大纲都只存视图状态。

## 资产与许可

Green 自有代码以 **GPL-3.0-or-later** 发布，全文见 [`LICENSES/GPL-3.0.txt`](LICENSES/GPL-3.0.txt)。

- **图标**：`assets/icon.svg`（母版）与 `assets/icon-small.svg`（≤32 px 变体），用户提供的原创素材，随本仓库以 GPL-3.0-or-later 发布；`scripts/icon.py` 生成 `assets/icons/green.ico` 与 `green.png`。图标产物已提交，构建不需要脚本。
- **字体**：随包 Noto Sans CJK SC Regular/Bold，SIL OFL 1.1，见 [`LICENSES/OFL-1.1.txt`](LICENSES/OFL-1.1.txt)；不单独售卖字体，字体文件不改名不改字形。`docs/LICENSE-AUDIT.md` 记着当前缺口：Noto Sans Math 与 Noto Sans Mono 尚未随包，字体族的 OFL 版权行也还没按族拆开。
- **组件库**：`vendor/ely-gpui-components` 是 [Ely-GPUI-Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components)（`fc71e96`，MIT）的 fork，声明见 [`LICENSES/Ely-GPUI-Components-MIT.txt`](LICENSES/Ely-GPUI-Components-MIT.txt)。
- **基座**：[ElyOffice](https://github.com/ELY-Open-Source/ElyOffice)（commit `3ba2adc`）为 MIT，见 [`LICENSES/ElyOffice-MIT.txt`](LICENSES/ElyOffice-MIT.txt)。
- **示例插件**：`examples/plugins/csv`（Python）、`tsv`（Python）、`log`（Node）是本项目代码，GPL-3.0-or-later；它们作为独立进程运行，不编进 `green.exe`。第三方插件只经 JSON 与 Green 通信，按工程判断通常不构成衍生作品（不是法律意见，详见审计）。
- **第三方依赖**：`green.exe` 链接的 573 个第三方 crate 的许可证见 [`THIRD-PARTY-NOTICES.txt`](THIRD-PARTY-NOTICES.txt)（`cargo about generate packaging/notices.hbs` 生成）；逐个核对、白名单与需要说明的项见 [`docs/LICENSE-AUDIT.md`](docs/LICENSE-AUDIT.md)。
- **便携包**：`script/package-windows.ps1` 出 `dist/Green-<版本>-windows-x86_64.zip`，含 `green.exe`、`fonts/`、图标、`LICENSE`、`LICENSES/`、`THIRD-PARTY-NOTICES.txt`、`README.txt` 与 `data/`；不含安装器、签名和自动更新。
