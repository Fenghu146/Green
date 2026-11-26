# Green 设计文档 v0.3

版本 v0.3 · 2026-11-26 · 状态：待批准

前提：一个人维护、完全开源、GPL-3.0、自用（可能分享给朋友）、学生。**时间不再是硬约束**（用户原话：追求质量、完成度、舒适度），范围冻结仍然有效。v0.2 / v0.1 存档在 `design-v0.2.md` / `design-v0.1.md`。
本轮两处实测：ElyMark 剪枝构建（§1.3）、22 MB PDF 的成因复现（§5.1）。

## 0. 相对 v0.2 改了什么

| v0.2 | v0.3 | 为什么 |
| --- | --- | --- |
| PDF「待定」 | **删净**（不读/不编辑/不预览），只保留导出 | 用户拍板 |
| 插件 = 编译期 crate | **外部进程 + JSON** | 真实动机是热更新：加格式不重编译、不发版 |
| 无导出章节 | 独立 §5，含 22 MB 复现与体积审计 | 删 `ely-pdf-engine` 会连带掐掉导出；旧链路已证实必炸 |
| Agent 章节（CLI/JSON/expect-hash/跟随滚动） | **整章删除**；只留外部改动检测与单实例转发 | Markdown 本身 Agent 友好，不需要再包命令面 |
| 15 天日程 | **阶段制 S1–S6**（无 Agent 阶段） | 时间放宽，且该阶段已取消 |
| `gpui-pre` 0.3.12 | **0.3.14** | 仓库实际 pin |
| 基座「待验证」 | **已实测** | 本轮主任务 |

官方不写：多引擎、协同、云、移动端、PDF 阅读、Agent 命令面。

## 1. 形态与基座

Markdown 一等公民；核心不认识 Markdown；插件是外部进程；单进程 GPUI；公式与导出同走 Typst。

```
┌──────────────────────────────────────────────────┐
│ green.exe（单进程，GPUI）                         │
│  shell : 窗口 · 工具条 · 状态栏 · 设置             │
│  core  : Block 模型 · 块渲染 · 编辑 · 原子保存     │
│  math  : 公式 → SVG（typst 库，按哈希缓存）        │
│  views : Registry（扩展名 → Provider）            │
│  export: Block → .typ → typst → PDF + 体积审计     │
└────────┬───────────────────────┬─────────────────┘
         │ 进程内                 │ stdin/stdout JSON
   plugins/markdown（内置）   外部插件进程（任意语言）
```

不变量：① 内容只存在磁盘文件里，无数据库、无自有格式、无 sidecar；② 所有落盘经 core（临时文件 + 原子替换）；③ `green-core` 不出现 `markdown` 字样；④ 插件崩溃/超时/清单坏都不影响启动；⑤ 无常驻服务、无安装器。

### 1.2 基座事实（clone 后核对，commit 3ba2adc）

- `apps/ely-mark` 是完整 app：`src/` 36 个条目、57 个 `.rs`、13,148 行；`main.rs` 38 行。
- 依赖 `ely-office` 与 `ely-pdf-engine`（后者仅 2 处：`output/files.rs:92`、`output/tests.rs:339`）。
- `crates/ely-office/Cargo.toml:18` 硬依赖 `ely-lok`；prelude 不导出 LOK 类型。
- 耦合在 crate 内部：**15 个文件、64 处 `ely_lok::`**，合计 3,836 行（全 crate 8,916 行）——open/save/find/links/panel/resident 都写在 `Engine = Lok` 上。
- `.cargo/config.toml` 已给 Windows MSVC 设 `-C target-feature=+crt-static`。

### 1.3 剪枝实测

| 项 | 结果 |
| --- | --- |
| 动作 | 删 7 个 app；去掉 `ely-pdf-engine`；`output/files.rs` 的 PDF 分支改为返回 Unsupported |
| 冷构建（debug） | **3 分 31 秒**（211.5 s，含从 git 拉组件库） |
| 改 `ely-office/src/lib.rs` 一行后的增量 | **26.4 s** |
| `ely-mark.exe` | debug **73.0 MB**；release **40.6 MB** |
| 能否打开 .md | **能**：`ELY_SMOKE=1` 退出码 0，日志 `opened 作业.md as tab 0` / `window shown`，到首帧 2.2 s |
| VCRT | release exe 无 `VCRUNTIME140`/`MSVCP140`/`ucrtbase` 字符串 → 静态链接成立 |
| 拆 `ely-office`（摘 LOK） | **不做**：15 文件 / 64 处 / 3,836 行，是「几百行」不是「几十行」 |

结论：**走 A（ElyOffice 剪枝）**。LOK 只作编译期依赖，不打包、不启动，运行时零成本。

## 2. 许可

**GPL-3.0-or-later**。gpui 经 `vendor/gpui-pre` 打补丁（GPL 仓库），这是事实形态；用户已接受。硬约束只剩：**不引入 AGPL**、**不引入无力履行义务的 MPL**。

| 组件 | 版本 | 许可 | 义务 |
| --- | --- | --- | --- |
| gpui（`gpui-pre`，vendor 打过补丁） | 0.3.14 | 包标 Apache-2.0，按 GPL-3.0-or-later 处理 | 整包 GPL，公开源码 |
| ElyOffice 自有代码 | 3ba2adc 起 fork | MIT | 保留版权与许可；可并入 GPL |
| Ely-GPUI-Components | fc71e96 | MIT / Apache-2.0 | 保留声明 |
| Typst / `typst-svg` / `lopdf` | 0.15.1 / 0.15.1 / 0.45 | Apache-2.0 / Apache-2.0 / MIT | 保留 LICENSE、NOTICE |
| `serde`/`serde_json`/`windows` | 1.0.229 / 1.0.154 / 0.62.2 | MIT OR Apache-2.0 | 保留声明 |
| 其余间接依赖 | 锁 `Cargo.lock` | MIT/Apache-2.0/Zlib/BSD/ISC | `cargo-about` 生成 `THIRD-PARTY-NOTICES.txt` |

排除：AGPL（分发即整包 AGPL，网络使用也触发 → MuPDF 出局）；MPL（文件级 copyleft，只在能履行时用）；LGPL 与 GPL 兼容，可用但暂无。
插件是独立进程、只经 JSON 通信，按通行理解通常不构成衍生作品，朋友写的插件可以不是 GPL（工程判断，非法律意见）。

## 3. 插件：外部进程 + JSON

```
%APPDATA%\Green\plugins\<id>\  manifest.json + 任意脚本
{ "id": "csv", "name": "CSV", "version": "0.1.0", "apiVersion": 1,
  "extensions": [".csv"], "command": ["python", "csv_plugin.py"],
  "timeoutMs": 3000, "capabilities": ["blocks@1"], "enabled": true }
```

安装 = 放目录；卸载 = 删目录；启停 = 设置或 `enabled`；热更新 = 换文件，下次解析生效——不重编译、不发版。

契约：stdin 收 `{apiVersion, path, ext, content(base64), contentOmitted}`，stdout 回 `{apiVersion, blocks[], meta}`。`content` 上限 8 MB（超出则 `contentOmitted`，插件按 `path` 自读）。Block 固定九种；不认识的整块忽略。非零退出、超时、JSON 不合法 → 降级 plaintext + 状态栏，不弹窗、不影响启动。

兼容性（第一天就设计）：`apiVersion` 整数、超范围拒绝；`capabilities` 协商；两侧忽略未知字段、新增字段必须可选有默认值；破坏性变更 `apiVersion += 1` 且旧版留 2 个小版本；按 (path, mtime, size, 插件版本) 缓存。

| 动作 | 改 Green 的文件 | 重编译 | 重发 tag |
| --- | --- | --- | --- |
| 加外部进程格式（CSV） | **0** | **否** | **否** |
| 改外部插件逻辑 | 0 | 否 | 否 |
| 改协议（apiVersion+1） | 1 文件 + 兼容层 | 是 | 是 |
| 加内置视图（Markdown） | `plugins/` 一模块 + registry 1 行 | 是 | 是 |

该不该做：CSV（会想打开，能覆盖，~100 行）、日志（会，能，~50）、Org（可能，部分，~150）、JSON/YAML（偶尔，能，~80）、HTML（偶尔，只能高亮，~60）、Typst（想预览，不能，不做）、Mermaid（想看图，能但需 Node，中等）。**结论：值得做，但只做这一层**——价值在于第一行是 0：加格式不重编译、不发版。代价约 1.5 天。
以后再说：市场/索引/签名、权限与沙箱、插件间依赖、native dylib 加载。

## 4. 文件与安全

1. Markdown 就是磁盘纯文本；任何程序都能写。不写 front-matter、不加锚点、不重排格式；打开再保存，未改动字节不变。
2. 没有锁文件；并发靠「读-改-写 + 哈希校验」。
3. **外部改动**（git checkout、同步盘、别的编辑器）：`notify` 8（`outside/` 已实现）+ 300 ms 去抖 + mtime/size/hash 比对。缓冲干净 → 静默重载，按光标所在行重新定位并保留滚动；缓冲脏 → 顶部细条三选一（重新载入 / 保留我的并另存 `name.conflict-*.md` / 行级差异）。**永不自动合并，永不静默覆盖。**
4. **单实例**：`\\.\pipe\green-{sid}`（SD 限定当前用户），帧 = `u32` 长度 + JSON，**只有一种消息** `{op:"open", path}`。第二次启动 → 发消息 → 退出；无实例则自己启动；超时 2 s。不对外暴露任何命令面。
5. **保存契约**：写同目录 `name.tmp-<rand>` → `ReplaceFileW`；遇 `ERROR_SHARING_VIOLATION`/`ERROR_ACCESS_DENIED` 重试 3 次（50/150/400 ms）→ 仍失败则保留临时文件并弹一次明确错误（路径 + 错误码 + 临时文件位置），原文件不动。绝不「删原文件再写」、绝不静默失败。
6. 自动保存：编辑后 800 ms 或每 200 次操作写 `%LOCALAPPDATA%\Green\journal\`（快照 + 追加日志）；撤销存反向操作，不存全文快照。

## 5. 导出（必须保留）

### 5.1 22 MB 到底花在哪（已复现）

用用户那份 `作业.md`（26 个块级公式、2.4 KB）走同一条链路：

| 环节 | 产物 | 页 | 字节 | 字体 | 图片 |
| --- | --- | --- | --- | --- | --- |
| ElyMark 导出 HTML | 作业.html | — | 545,566 | 只写 `sans-serif`/`monospace` | **0 张**，28 个内联 SVG |
| MuPDF 1.28.6 转同一 HTML（PyMuPDF Story，同一引擎） | story.pdf | 2 | **4,605,496** | 6 个，全部 `sub=no` | 28 张（=28 公式），96 dpi，**未压缩** |
| Typst 排同一组 26 个公式 | doc26.pdf | 1 | **56,525** | 4 个，全部 `sub=yes` | 0 |

story.pdf 字节拆解：字体 **4,140,464（89.9%）**（最大单个 3,556,308）、图片 387,219（8.4%）、其余 57,930（1.3%）。

**两个成因同时存在，不是二选一。**
① **主因是字体整包嵌入**（`sub=no`，90%）。本机 `msyh.ttc` 18.8 MB、`simsun.ttc` 17.5 MB、`msyhbd.ttc` 16.1 MB——整包嵌一个中文字体，4 页就是 18 MB 量级，与 22 MB 吻合；我复现用的是 MuPDF 自带回退字体（3.56 MB），所以只有 4.6 MB。
② **次因是公式被光栅化且未压缩**：28 个公式正好 28 张图，96 dpi、raw RGB（`pdfimages` ratio 100%）。DPI 一升是平方增长：300 dpi ≈ 3.8 MB，600 dpi ≈ 15 MB。

> 复现用的是「同一 HTML + 同一 MuPDF 版本」，不是 worker 本体（需 MuPDF 源码构建，本机 `engines/` 为空）。worker 的 DPI 与字体选择让数字在 4.6–22 MB 间移动，机制不变。

### 5.2 后端与判断

Typst（Apache-2.0，zip 21.4 MB / exe 50.1 MB）：矢量、字形子集、Flate 默认，**挡得住**，1.5 天。pandoc（GPL-2.0-or-later，41.6 MB + TeX 数百 MB–数 GB）：挡不住/取决于引擎，2–3 天且难装。自绘 PDF：挡得住但要自己写子集化，3–4 天，放 `later.md`。
**选 Typst**——4.6 MB → 56 KB 就是理由。

四个成因 × 防护 × 自证：

| 成因 | 防护 | 自证 |
| --- | --- | --- |
| 字体整包嵌入 | 只用实际 family/weight；不随包中文字体；Typst 默认子集 | 逐字体报字节与 `sub`；单字体 > 3 MB 告警 |
| 页面被位图化 | 全矢量；不提供截图式导出 | 图片数 vs 页数；每页一张近整页大图即告警 |
| 图片重复/未压缩 | 按内容哈希去重；JPEG 保 DCTDecode，PNG 走 Flate | 按流哈希统计总字节与重复次数；占比 > 70% 告警 |
| 内容流未压缩 | 所有流必须 FlateDecode | 逐 stream 检查 `/Filter` |

审计（`lopdf` 0.45，MIT）：页数、总字节、逐字体字节（`FontFile2/3`、`/Length1`）、图片（`/Filter` 与长度、按哈希去重）、内容流压缩、每页最大图片覆盖率。导出后固定一行：

```
export pdf: pages=12 bytes=845120 fonts=[{"name":"MicrosoftYaHei","bytes":412000}] images=3/210000 (dup:0) compressed=true max_image_coverage=0.0%
```

阈值：单页均值 > 500 KB、单字体 > 3 MB、图片占比 > 70%、任一图片覆盖页面积 > 40% → 日志 + 状态栏。
验收：用户 fixture **≤ 2 MB**、导出 ≤ 10 s、审计齐全、`compressed=true`、`max_image_coverage ≈ 0`、无单字体 > 3 MB；回归超 2 MB 阻断发布。

**这条链路不修，整条换掉。** 修字体子集化要改 MuPDF 的 C 源码（无维护能力）；换 Chromium headless 要引入 100 MB+ 运行时；Typst 1.5 天解决且与编辑器公式同源。HTML 导出保留为独立功能，不参与 PDF。

## 6. 数学公式

**Typst 是唯一后端**：编辑器用 `typst` + `typst-svg` **作为库**（不是 CLI）把公式渲成 SVG 并缓存；导出整篇走 Typst 编译。同一段公式文本、同一套字体、同一个排版器 → 不会「编辑器一个样、导出一个样」。
已知风险：GPUI 文本嵌图片是基座缺口。块级公式（用户的 20+ 条）无此问题；行内需走 `math_text` 那条路，S2 第一天验证，过不了则行内降级为「源码 + 悬浮预览」，块级不受影响。

白名单（名单外一律显示原始 `$...$` 源码并标红，不崩、不空白）：

| 类别 | 宏 |
| --- | --- |
| 文本/框 | `\text{}`（含中文）、`\boxed{}`、`\quad`、`\ `、`\\`、`&`（对齐） |
| 上下标 | `x^{}`、`x_{}`、`x_i^2` |
| 分数/根号 | `\frac{}{}`、`\sqrt{}`、`\sqrt[n]{}` |
| 运算 | `\times`、`\cdot`、`\pm`、`\mp`、`\div`、`\bmod`、`\sim`、`\approx`、`\equiv` |
| 关系 | `=`、`<`、`>`、`\le`、`\ge`、`\ne`、`\to`、`\rightarrow`、`\leftarrow`、`\Rightarrow` |
| 算子 | `\sum`、`\prod`、`\int`（带上下限） |
| 括号 | `(`、`)`、`[`、`]`、`\{`、`\}`、`\left(`…`\right)` |
| 希腊字母 | `\alpha \beta \gamma \delta \epsilon \theta \lambda \mu \pi \sigma \phi \omega` 与 `\Gamma \Delta \Theta \Lambda \Pi \Sigma \Phi \Omega` |
| 其他 | `\infty`、`\dots`、`\cdots` |

名单外（`\begin{matrix}`、`\overbrace`、`\mathbb`、自定义宏等）→ 源码标红。导出时同样放源码文本以保证不丢内容，并在报告里计数。

关键映射：`\text{中文}`→`"中文"`；`\boxed{x}`→`#box(stroke: 0.6pt, inset: 3pt, radius: 1pt)[$x$]`；`\leftarrow`→`arrow.l`；`\times`/`\cdot`→`times`/`dot.op`；`\bmod`→`mod`；`\sim`→`tilde.op`；`\quad`→`quad`；`\le/\ge/\ne`→`lt.eq`/`gt.eq`/`eq.not`；`x^{2}`→`x^2`；**裸标识符 `B052H` 必须写成 `"B052H"` 或 `B 0 5 2 H`**（实测坑：否则 unknown variable）。

覆盖率（实测）：`作业.md` 的 26 个块级公式逐条转换，**26/26 编译通过**（`doc26.pdf`，exit 0）。唯一失败点是上述引号规则，属转换器规则，不是能力缺口。

延迟（实测）：CLI 编译 1 个公式→SVG **529 ms**（warm），1 个→PDF 553 ms，26 个→PDF 570 ms——启动与字体扫描占绝大部分，25 条公式只多 17–50 ms，即**进程内约 1–2 ms/公式**。所以渲染必须用库，CLI 只用于整篇导出（一次进程）。
缓存键：`hash(公式源 + 字号 + 主题色 + dpi 档)`；滚动、重绘、同色主题切换都命中；改一个字符只重编那一条。

备选都不选：`ratex`（crates.io 0.0.1、136 下载、MIT，未发布状态）；`latex2mathml`（0.2.3、2020 后未更新，只产 MathML 而 GPUI 无渲染器）；`katex`/KaTeX/MathJax（需 JS 运行时，与原生冲突，只能作退路）；自绘（排版 + 字体度量 + 伸缩括号，远超 1.5 天）。

## 7. 性能

| 指标 | 目标 |
| --- | --- |
| 冷启动到首屏 | ≤ 2.0 s p95（基座 3.4–3.6 s，剪掉引擎后砍半） |
| 热启动到首屏 | ≤ 600 ms p95 |
| 打开 5 万字可输入 | ≤ 400 ms p95 |
| K2P | p50 ≤ 8 ms、p95 ≤ 16 ms、p99 ≤ 33 ms；>16 ms 占比 < 1% |
| 滚动 | 60 fps，掉帧率 < 1% |
| 内存（5 万字） | ≤ 200 MB |
| UI 线程长任务 | 0 次 > 50 ms |
| 增量构建 | ≤ 60 s（实测 26.4 s） |
| 首屏公式 | 一屏内全部就位 ≤ 150 ms（缓存未命中） |

单段 5 万字（一行）是对抗用例：S2 实测，不达标按序降级——① 关该段行内装饰与软换行；② 超 2 万字符段落折叠为源码块；③ 移出支持范围并写进 README 已知限制。

## 8. 阶段与完成定义

- **S1 基座与外壳**：剪枝、构建、能开 .md（已达成）；单窗口无侧栏、工具条（插入/加粗/斜体/下划线/删除线/高亮/颜色/字号/对齐/列表）、正文居中、浅深主题。**退出**：增量 ≤ 60 s；两张截图；能开用户这份 `作业.md`。
- **S2 Markdown 与公式**：逐段渲染、编辑闭环（光标/选区/IME/撤销/查找替换）；块级 + 行内公式、SVG 缓存、降级标红。**退出**：5 万字 P1 达标（K2P p95 ≤ 16 ms、掉帧 < 1%）；单段 P2 有结论；26 个公式全对、0 崩溃；行内方案已定。
- **S3 文件与安全**：外部改动、单实例、原子保存 + 占用重试、自动保存/崩溃恢复、撤销。**退出**：杀进程不丢稿；外部改文件不静默覆盖（录屏）；占用保存有明确错误且保留临时文件；第二次启动交给已开窗口。
- **S4 插件**：supervisor、契约、清单、启停、本地安装。**退出**：Python CSV 插件工作；超时被杀并降级；坏清单不影响启动；加格式时 Green 零改动。
- **S5 导出**：Block→`.typ`、Typst 调用、`pdf_audit`、阈值。**退出**：用户 fixture ≤ 2 MB；审计齐全；`max_image_coverage ≈ 0`；`pdffonts` 全 `sub=yes`。
- **S6 打包与硬化**：zip 免安装、便携数据目录、首次运行、VCRT 验证、日志、许可页、10 份真实文档。**退出**：干净 Win11 解压可用；`LICENSE` + `THIRD-PARTY-NOTICES` 在包内。

**什么时候算完**（六条全过即发 1.0，之后只修 bug、不加功能，新需求进 `later.md`）：
① S1–S6 退出条件全过；② 10 份真实文档（含用户这份）打开/编辑/保存无错；③ 导出 ≤ 2 MB、审计无告警、公式矢量；④ 5 万字 K2P p95 ≤ 16 ms、掉帧 < 1%，单段用例有书面结论；⑤ 干净机器解压即用、无 VCRT 依赖；⑥ 许可文件齐全、源码公开可拉。

## 9. 风险

| # | 风险 | 应对 |
| --- | --- | --- |
| R1 | 组件库改动拖慢增量构建 | 已实测 26.4 s；超 60 s 再瘦身 |
| R2 | 单段 5 万字不达标 | S2 按 §7 顺序降级，当天定论 |
| R3 | Typst 库集成（World/FontBook）比预期难 | S2 先做最小 World；退路 CLI + 预渲染缓存（首屏慢但可用） |
| R4 | 行内公式撞上 GPUI 不能嵌图 | 行内降级源码 + 悬浮预览；块级不受影响 |
| R5 | 插件进程安全面 | 无沙箱，等同用户自己运行脚本；只从信任来源装 |
| R6 | 时间宽松导致范围膨胀 | D5 范围冻结；新需求进 `later.md` |

## 10. 决策

- **D1 基座：ElyOffice 剪枝（已实测，推荐 A）**。能编能开、增量 26.4 s；LOK 作编译期依赖保留，不打包 worker。代价：多编一个 crate、3,836 行 LOK 代码留着不用。
- **D2 PDF 阅读/编辑：不做（已定）**。只导出。
- **D3 许可：GPL-3.0-or-later（推荐）**。事实形态；义务是公开源码 + 附许可全文。
- **D4 插件：外部进程 + JSON（推荐）**。加格式不重编译、不发版；语言无关；朋友插件不必 GPL。代价约 1.5 天、无沙箱、只能产 Block。
- **D5 范围冻结（已批准）**。只做 S1–S6；含保存契约（占用重试 + 明确报错，§4 第 5 条）、导出验收线（≤ 2 MB，§5.2）、公式白名单（名单外源码标红，§6）。
