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

## 许可

Green 自有代码以 **GPL-3.0-or-later** 发布，全文见 [`LICENSE`](LICENSES/GPL-3.0.txt)。

- 基座 [ElyOffice](https://github.com/ELY-Open-Source/ElyOffice)（commit `3ba2adc`）为 MIT，见 [`LICENSES/ElyOffice-MIT.txt`](LICENSES/ElyOffice-MIT.txt)。
- 界面组件 [Ely-GPUI-Components](https://github.com/ELY-Open-Source/Ely-GPUI-Components)（`fc71e96`）为 MIT，见 [`LICENSES/Ely-GPUI-Components-MIT.txt`](LICENSES/Ely-GPUI-Components-MIT.txt)。
- 随包字体 Noto Sans CJK SC 为 SIL OFL 1.1，见 [`LICENSES/OFL-1.1.txt`](LICENSES/OFL-1.1.txt)；不单独售卖字体，字体文件不改名不改字形。
- 其余依赖的许可见 [`THIRD-PARTY-NOTICES.txt`](THIRD-PARTY-NOTICES.txt)（由 `cargo-about` 生成，S6 复核）。
