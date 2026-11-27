# later.md

范围外的想法。不进 S1–S6，执行期不跑偏；要做时先改 `docs/design.md` 的范围。

## 界面

- **界面风格后续调整，会有我们自己的设计语言**（用户原话）。S1 的骨架照搬基座视觉；用户评价「和 ElyMark 差不多，完成度不错」。未来重做的是设计语言，不是布局——执行期不要动视觉。
- 菜单栏保留（用户定案，2026-11-26）；左侧栏「文件」页签保留。顶栏维持现状，不做两行。
- 插入菜单不照搬 ima 的「录音纪要 / 附件 / 笔记」：那是笔记产品功能（录音转写、附件库、笔记互链），不是 Markdown 编辑器的事。

## 大纲

- 列表项是否进大纲。现在只认标题 H1–H6；若以后有笔记拿有序列表当结构，再加一个开关。

## 语法

- `==高亮==` 是非标准 Markdown 扩展；Typora 等编辑器会显示原始 `==`。不再扩更多非标准行内语法，除非先写进 `docs/design.md` 的扩展语法清单。

## 插件

- 列表项、alert 正文、表格单元格内嵌 Block。v1 的 Block 契约把它们定成字符串；要做就是 `apiVersion += 1`，并按约定保留 v1 两个小版本。
- 给 manifest 加显式 `priority` 字段，取代现在按 `id` 字典序选扩展名属主的规则。现在不做：要改宿主产品代码（`green-plugins`），而这一轮验证的是"零产品代码加格式"；增强版用 `0` 前缀 id 是权宜之计，已记在 `examples/plugins/README.md`。

## 打包与便携

- 便携模式（`<exe>/data` 存在即用）现在是 `apps/green/src/portable.rs` 的产品策略，`crates/` 层不知道“便携”这个概念。若将来多个 app 共用这套策略，再把它上移进 `ely-platform`。

## 平台

- ~~Linux / macOS 目标~~：2026-11-27 已扩进 v1（`docs/design.md` §1.5，run 45939598799）。剩下的两条缺口记在这里：**macOS 运行验证**（只验过构建），以及 **Linux 的 `--capture` 帧读回**（gpui 的 Linux 后端没有，要做得先 vendor `gpui-pre-wgpu` 并在 surface renderer 上补离屏读回）。
- `ely-platform/src/diagnostics.rs:47` 仍把 `AppId::Pdf` 映射到 `"ely-pdf-worker"`，而 `ely-pdf-engine` / `ely-pdf-worker` 都已移出 workspace。它现在只是日志里的一个字符串，不影响行为；删 Pdf app 时一并去掉。
