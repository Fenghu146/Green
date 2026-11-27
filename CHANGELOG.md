# 更新记录

## 1.0.0

第一个对外发布的版本，Windows / macOS / Linux 三平台。

**应用**

- 离线 Markdown 笔记编辑器：内容只存在磁盘上的 `.md` 文件里，没有数据库、没有自有格式、没有 sidecar。
- 逐段渲染与源码视图；标题折叠与大纲树只改视图，从不写进文件。
- 行内与块级公式用 Typst 进程内渲染，编辑器与 PDF 导出共用同一批随包字体。
- 多标签、会话恢复、原子保存、单实例转发。
- 外部进程插件（stdin/stdout JSON）；加一种格式不需要改产品代码。
- 导出 PDF 走 Typst 库，**不依赖外部 `typst.exe`**。

**发布物**

- Windows：NSIS 安装包（每用户安装，不要求管理员）与便携 zip。
- macOS：`.dmg`（拖进「应用程序」）与便携 zip，arm64 与 x86_64 各一份，ad-hoc 签名。
- Linux：`tar.gz`，附 `.desktop` 与图标。
- 每个包自带字体、`LICENSE`（GPL-3.0 全文）与 `THIRD-PARTY-NOTICES.txt`；发布页另附 `SHA256SUMS.txt`。

**已知限制**

- `green --capture` 的帧读回仅 Windows 实现；macOS / Linux 返回 `render_to_image not implemented`。
- macOS Intel 包只验证了构建，没有在 Intel 真机上运行过。
- 插件按 `id` 字典序决定同名扩展的归属，清单里没有显式优先级字段。
- `\begin{aligned}` 等 LaTeX 环境不在转换器覆盖内（只认 `cases`），导出时那一条公式降级为源码。
- 未签名：Windows 会弹 SmartScreen，macOS 会提示「无法验证开发者」（见发布说明的处理步骤）。
- 不做 `.md` 文件关联。
