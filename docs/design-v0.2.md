# Green 设计文档 v0.3

版本 v0.3 · 2026-11-26 · 状态：待批准

前提：一个人维护、15 天内能用、完全开源、GPL-3.0、自用（可能分享给朋友）、学生、零法律风险承担能力。
v0.2 存档在 `design-v0.2.md`，v0.1 在 `design-v0.1.md`。本版只写当前结论。

---

## 0. 相对 v0.2 改了什么

| v0.2 | v0.3 | 为什么 |
| --- | --- | --- |
| PDF「待定」 | **删净**：不读、不编辑、不预览（导出 PDF 另算，见 §5） | 用户拍板 |
| 插件 = 编译期 crate | **插件 = 外部进程 + JSON** | 真实动机是热更新：加格式不想重新编译、重新发版；编译期插件恰好砍掉了这个动机 |
| 无导出章节 | 独立 §5：Typst 后端 + 体积审计 + 2 MB 验收线 | 删 `ely-pdf-engine` 会连带掐掉现有 PDF 导出；且 26 MB 的教训必须有机制挡住 |
| D5 待批 | **已批准** | 用户批了 |
| `gpui-pre` 0.3.12 | **0.3.14** | 仓库实际 pin；0.3.12 是我上一版写错 |
| 基座「待验证」 | **已实测**（§1.3） | 本轮主任务 |
| v0.1 的多引擎 / LibreOfficeKit / MuPDF / WASM 市场 / 36–47 周 | 保持砍掉 | 不再重复论证 |

---

## 1. 形态与基座

### 1.1 一句话

Markdown 一等公民；核心不认识 Markdown；插件是外部进程；单进程 GPUI；导出走 Typst。

```
┌──────────────────────────────────────────────────────────┐
│ green.exe（单进程，GPUI）                                 │
│  shell : 窗口 · 工具条 · 状态栏 · 设置 · 命令面板          │
│  core  : Block 模型 · 块渲染 · 编辑会话 · 原子保存 · 导出  │
│  views : Registry（扩展名 → Provider）                    │
└────────┬──────────────────────────┬──────────────────────┘
         │ 进程内                    │ stdin/stdout JSON
   plugins/markdown（内置）     外部插件进程（Python/Node/Rust…）
     .md → Block 树               .csv/.log/… → Block 树
```

五条不变量：

1. 内容只存在磁盘上的文件里：没有数据库、没有自有格式、没有 sidecar。
2. 所有落盘经过 core：临时文件 + 原子替换（§4.5）。
3. `green-core` 不出现 `markdown` 字样；Markdown 只是注册进来的内置 Provider。
4. 插件是独立进程，通过 JSON 契约交互；崩溃、超时、清单坏都不影响启动。
5. 没有常驻服务、没有后台进程、没有安装器（导出时起的 `typst` 与插件进程都是按需的）。

### 1.2 基座事实（clone 后核对，commit 3ba2adc）

- `apps/ely-mark` 是完整 app，不是库：`src/` 36 个条目、57 个 `.rs`、13,148 行；`main.rs` 38 行，只有 `remote::command_line()` 与 `ely_ui::run(..., desk::open, ...)`。
- 依赖 `ely-office`（外壳）与 `ely-pdf-engine`（仅 2 处：`output/files.rs:92` 的 HTML→PDF，`output/tests.rs:339`）。
- `crates/ely-office/Cargo.toml:18` 硬依赖 `ely-lok`（features = ["view"]）；但其 prelude 只导出 `Chrome/Closing/Commands/Dialogs/Finding/Following/Host/Linking/Opening/Remote/Saving/Stopping/Tabs/Undoing/Zooming`，不含 LOK 视图类型。
- 真正的耦合在 crate 内部：**15 个文件、64 处 `ely_lok::`**，这些文件合计 3,836 行（全 crate 8,916 行）——open/save/find/links/panel/resident 都写在 `Engine = Lok` 上。
- `.cargo/config.toml` 已给 Windows MSVC 设 `-C target-feature=+crt-static`（VCRT 静态链接，D12 只需验证）。
- `notify` 8 已在依赖里；`outside/` 已做外部改动检测；`remote.rs` 已有 `command_line()` 与 `ely_ui::Ask`。

### 1.3 剪枝实测（本轮主任务）

| 项 | 结果 |
| --- | --- |
| 剪枝动作 | 删 7 个 app（doc/table/show/pdf/mail/tree/agent）、从 ely-mark 去掉 `ely-pdf-engine` 依赖、把 `output/files.rs` 的 PDF 分支改为返回 Unsupported |
| 冷构建 | **3 分 31 秒（211.5 s，debug；含从 git 拉组件库）** |
| 改 `ely-office` 一行后的增量构建 | **26.4 s（改 `ely-office/src/lib.rs` 加一行注释 → 重编 ely-office + ely-mark + 链接）** |
| `ely-mark.exe` 体积（debug） | **73.0 MB（debug，含调试信息）** |
| 能否打开 .md | **能。`ELY_SMOKE=1 ely-mark.exe fixtures\smoke.md` 退出码 0，日志 `opened smoke.md as tab 0` / `shown as visual` / `window shown`；进程到首帧 2.2 s** |
| 拆 `ely-office`（把 LOK 摘掉）的实际改动 | 不做。15 文件 / 64 处引用 / 涉及 3,836 行，属「几百行」而非「几十行」；15 天里不划算 |

结论：**走 A（ElyOffice 剪枝），不转 B**。LOK 只作为编译期依赖留着，不构建 worker、不打包、不启动——运行时零成本。

---

## 2. 许可

结论：**GPL-3.0-or-later**。基座 gpui 经 `vendor/gpui-pre` 打了补丁（GPL 仓库），所以这是事实形态；你已接受。

两条硬约束保留：**不引入 AGPL**、**不引入无力履行义务的 MPL**。

### 2.1 白名单（进发布包或参与构建的）

| 组件 | 版本 | 许可 | 义务 |
| --- | --- | --- | --- |
| gpui（`gpui-pre`，vendor 打过补丁） | 0.3.14 | 发布包标 Apache-2.0；因打补丁，按 GPL-3.0-or-later | 整包 GPL，公开源码 |
| ElyOffice 自有代码 | commit 3ba2adc 起 fork | MIT | 保留版权与许可；MIT 可并入 GPL |
| Ely-GPUI-Components | rev fc71e96 | MIT / Apache-2.0 | 保留声明 |
| Typst（导出后端） | 0.15.1 | Apache-2.0 | 保留 LICENSE；随包时可附其 NOTICE |
| `lopdf`（体积审计） | 0.45 | MIT | 保留声明 |
| `serde` / `serde_json` / `windows` | 1.0.229 / 1.0.154 / 0.62.2 | MIT OR Apache-2.0 | 保留声明 |
| 其余间接依赖 | 锁 `Cargo.lock` | MIT / Apache-2.0 / Zlib / BSD / ISC | `cargo-about` 生成 `THIRD-PARTY-NOTICES.txt` |

### 2.2 排除项

| 许可 | 后果 | 处理 |
| --- | --- | --- |
| AGPL-3.0 | 分发即整包 AGPL，网络使用也触发 | **禁止**（历史选择里的 MuPDF 因此出局） |
| MPL-2.0 | 文件级 copyleft，改过的文件要公开 | 只在能履行时用；基座里已无 LibreOffice |
| LGPL | 动态链接需可替换 + 提供修改 | 与 GPL 兼容，可用；本项目暂无 |
| GPL-3.0 | 整包开源 | 本项目主许可 |

### 2.3 插件与许可（写清楚，别让朋友担心）

插件是**独立进程**，与 Green 只通过 stdin/stdout 的 JSON 交换数据。按 FSF 对「独立进程、通用协议、非紧密耦合」的通行理解，这通常不构成衍生作品，朋友写的插件可以不是 GPL。这是工程判断，不是法律意见；真要分发时在插件 README 里写明「本插件与 Green 仅经 JSON 通信」。

---

## 3. 插件：外部进程 + JSON

### 3.1 形态

```
%APPDATA%\Green\plugins\<id>\
  manifest.json
  csv_plugin.py        ← 任意语言，任意文件
```

```json
{ "id": "csv", "name": "CSV 表格", "version": "0.1.0",
  "apiVersion": 1, "extensions": [".csv", ".tsv"],
  "command": ["python", "csv_plugin.py"], "timeoutMs": 3000,
  "capabilities": ["blocks@1"], "enabled": true }
```

- 安装 = 放一个目录；卸载 = 删目录；启用/禁用 = 改 `enabled` 或设置里点。
- 热更新 = 换掉目录里的文件，下次解析生效；不需要重新编译 Green、不需要发 tag。
- 语言无关：Python / Node / Rust / Go 都行，只要读 stdin 写 stdout。

### 3.2 契约

请求（Green → 插件，stdin，一个 JSON 对象）：

```json
{ "apiVersion": 1, "path": "C:/x/a.csv", "ext": ".csv",
  "content": "<base64>", "contentOmitted": false }
```

响应（插件 → Green，stdout，一个 JSON 对象）：

```json
{ "apiVersion": 1,
  "blocks": [ { "type": "table", "rows": [["a","b"],["1","2"]] } ],
  "meta": { "title": "a.csv" } }
```

- `content` 上限 8 MB（base64 后）；超过则 `contentOmitted: true`，插件自己按 `path` 读。
- Block 类型固定九种（段落/标题/列表/代码/引用/表格/图片/分隔线/提示），不认识的类型整块忽略。
- 失败：非零退出、超时、JSON 不合法 → 该文件降级为 plaintext 视图，状态栏一条提示，**不弹窗、不影响启动**。

### 3.3 兼容性（第一天就设计，不是以后补）

| 机制 | 做法 |
| --- | --- |
| 协议版本 | 清单 `apiVersion`（整数）。宿主支持 1..=N；不在范围→拒绝该插件并提示 |
| 能力协商 | 插件声明 `capabilities`（如 `blocks@1`）；宿主只调用双方都声明过的能力 |
| 前向兼容 | 两侧解析都忽略未知字段（不要 `deny_unknown_fields`）；新增字段必须可选且有默认值 |
| 向后兼容 | 破坏性变更 → `apiVersion += 1`，旧版本至少保留 2 个小版本 |
| 降级 | 清单坏/命令找不到/超时 → plaintext + 状态栏；插件被禁用不影响其他插件 |
| 缓存 | 按 (path, mtime, size, 插件版本) 缓存解析结果；改动即失效 |

### 3.4 加一个新格式，到底要动什么

| 动作 | 改 Green 的哪些文件 | 重新编译 Green | 重新发 tag |
| --- | --- | --- | --- |
| 加一个外部进程格式（CSV） | **0** | **否** | **否** |
| 改一个外部插件的解析逻辑 | 0 | 否 | 否 |
| 改插件协议本身（apiVersion+1） | 协议模块 1 个文件 + 兼容层 | 是 | 是 |
| 加一个内置视图（Markdown 这种） | `plugins/` 一个模块 + registry 1 行 | 是 | 是 |

这张表就是保留插件系统的理由：**第一行是 0**。

### 3.5 该不该有插件系统（现实清单）

| 格式 | 会真想打开吗 | 外部进程能覆盖吗 | 插件成本 |
| --- | --- | --- | --- |
| CSV / TSV | 会（导出的数据、日志表格） | 能（→ Table 块） | ~100 行 |
| `.log` | 会（构建、服务日志） | 能（高亮 + 行号） | ~50 行 |
| Org | 可能会（课程笔记迁移） | 部分（语法子集） | ~150 行 |
| JSON / YAML | 偶尔 | 能（折叠树 → 列表/代码块） | ~80 行 |
| HTML | 偶尔看源码 | 不能预览（没有浏览器引擎），只能高亮 | ~60 行 |
| Typst | 想预览 | 不能（要编译成图，而 v1 不显示 PDF） | 高，不建议 |
| Mermaid | 想看图 | 能（mermaid-cli 出 SVG） | 中，需 Node |

**结论：值得做，但只做这一层。** 理由不是「以后可能加很多格式」，而是：Markdown 之外的「偶尔打开」需求真实存在（CSV、日志最可能），而外部进程模型把每个格式的成本压到 50–150 行、且不用重编译发版。代价是 1.5–2 天的保险费（supervisor + 契约 + 一个示例）。如果连这 1.5 天都不想付，那就只留「扩展名 → 内置视图」的清单层——但那样加新格式仍要重编译，等于没有插件系统。

### 3.6 成本与排期

外部进程比编译期插件多约 1.5 天（进程管理、超时、错误、示例）。这 1.5 天从「设置/主题」与缓冲日里挤：设置压成半天、主题沿用基座。**15 天不变，但没有整天缓冲，只剩半天。** 再加一个意外就要砍「插件启停 UI」（退化为改 manifest 里的 `enabled`）。

### 3.7 以后再说

插件市场/索引/签名、权限与沙箱、插件间依赖、运行时加载 native dylib（要直接操作窗口时才考虑）——都不做；触发条件分别是「插件 ≥ 5 个」「开始装不可信来源」「出现第一个真实依赖需求」「外进程不够用」。

---

## 4. Agent 友好

### 4.1 不变量

1. Markdown 就是磁盘上的纯文本；Agent 用任何工具写，Green 都能读。
2. 不写 front-matter、不加锚点、不重排格式；打开再保存，未改动字节不变。
3. 没有锁文件；并发靠「读-改-写 + 哈希校验」。

### 4.2 外部改动

- 检测：`notify` 8（基座已有）+ 300 ms 去抖；比对 mtime/size/hash。
- 无未保存修改 → 静默重载，按光标所在行文本重新定位，保留滚动。
- 有未保存修改 → 顶部细条三选一：重新载入 / 保留我的（外部版本另存 `name.conflict-*.md`）/ 看差异（行级高亮）。
- 永不自动合并，永不静默覆盖。

### 4.3 CLI 与三条 IPC 路径

```
green <cmd>
   │
   ├─ 应用没在跑 ──────────► 直接读/写文件（原子替换），退出
   │                          （只有 open 会启动应用）
   │
   └─ 应用在跑 ─► 命名管道 \\.\pipe\green-{sid}（SD 限定当前用户）
                     │
                     ├─ 文件没打开 ─► open 则让应用打开；其余命令直接改文件
                     │
                     └─ 文件已打开 ─► 应用内执行（同一撤销栈、同一脏状态），由应用保存
```

- 管道帧：`u32` 长度 + JSON；握手带 `protocol` 版本；单请求超时 2 s。
- 超时或管道不可用 → 退化为直接改文件，并在 stderr 说明（Agent 可据此重试）。

| 命令 | 作用 | 关键参数 |
| --- | --- | --- |
| `green open <path>` | 打开/聚焦 | `--line N` |
| `green read <path>` | 输出文本 | `--lines A:B`、`--json` |
| `green insert <path>` | 插入 | `--at end` / `--line N`、`--text` 或 `--stdin` |
| `green replace <path>` | 替换 | `--find`、`--with`、`--all`、`--expect-hash` |
| `green outline <path>` | 标题树 | `--json` |
| `green status <path>` | 打开/脏/哈希 | `--json` |

退出码：0 成功、2 用法错、3 哈希冲突、4 文件被占用。

### 4.4 Agent 写完之后

```
Agent 写文件 → notify → 比对哈希 → 干净则静默重载（恢复光标/滚动）
                              → 脏则三选一（不覆盖、不合并）
→ 光标在文末且窗口未聚焦时跟随滚动（可关）
```

### 4.5 保存契约（含占用重试）

1. 写 `name.tmp-<rand>`（同目录，保证同卷）。
2. `ReplaceFileW` 失败于 `ERROR_SHARING_VIOLATION` / `ERROR_ACCESS_DENIED` 时，重试 3 次（间隔 50 / 150 / 400 ms）。
3. 仍失败 → **保留临时文件**，弹一次明确错误（目标路径 + 错误码 + 临时文件位置），原文件不动。
4. 绝不「删掉原文件再写」、绝不静默失败、绝不吞异常。

---

## 5. 导出（必须保留）

### 5.1 连带影响

删 `ely-pdf-engine` 会掐掉 `output/files.rs` 的 PDF 分支（"生成 HTML → 交给引擎转 PDF"）；同文件的 HTML / PNG 导出不受影响。所以「删引擎」与「保留导出」必须一起解决：导出换成新后端。

### 5.2 后端评估

| 后端 | 体积失控挡得住吗 | 质量 | 许可 | 随包/依赖 | 15 天成本 |
| --- | --- | --- | --- | --- | --- |
| **Typst**（`typst.exe` 外进程） | **挡得住**：矢量文本、字形子集、Flate 默认 | 高（专业排版） | Apache-2.0 | 官方 Windows 包 **21.4 MB**（zip，v0.15.1） | 1–1.5 天 |
| pandoc（+PDF 引擎） | **挡不住/取决于引擎**：TeX 正常但装不下；wkhtmltopdf 常整包嵌字体 | 中 | GPL-2.0-or-later | pandoc **41.6 MB** + TeX 500 MB–2 GB 或 wkhtmltopdf | 2–3 天，且装不起来 |
| 自绘 PDF | 挡得住，但子集化要自己写 | 可控 | 自有 | 0 | 3–4 天（子集化 2–3 天） |

**选 Typst**：Green 已有 Block 树，直接 Block → `.typ` → `typst compile`，比 pandoc 少一层转换；它默认输出矢量与子集字体，体积成因里的第 1、2、4 条在构造上就不会发生。自绘放 `later.md`，除非前两条都不成立。

### 5.3 四个体积成因 × 防护 × 自证

| 成因 | 防护 | 自证（导出后自动打印） |
| --- | --- | --- |
| 字体整包嵌入 | 只嵌实际出现的字形（Typst 上游默认；**需用审计器复验，不靠信任**）；不随包中文字体，用系统字体；不重复嵌同一字体实例 | 逐字体报字节 + 字形数；单字体 > 3 MB 告警 |
| 页面被位图化 | 全矢量输出；**不提供「截图式」导出路径** | 统计图片数与页数：每页一张近整页大图即告警 |
| 图片重复 / 未压缩 | 按内容哈希去重后引用；JPEG 保持 DCTDecode，PNG 走 Flate；不重编码成无损位图 | 按流哈希统计图片总字节与重复次数；图片占比 > 70% 告警 |
| 内容流未压缩 | 所有流必须带 FlateDecode | 逐个 stream 检查 `/Filter`；缺失即告警 |

### 5.4 体积审计（自证机制，不靠眼睛）

用 `lopdf`（基座 workspace 已有，0.45，MIT）写 `pdf_audit`：页数、总字节、字体（`FontFile2/3` 长度、字形相关字段）、图片（`/Subtype /Image` 的 `/Filter` 与长度、按流哈希去重计数）、内容流压缩。

导出后固定一行结构化日志：

```
export pdf: pages=12 bytes=845_120 fonts=[{"name":"MicrosoftYaHei", "bytes":412_000, "glyphs":1830}] images=3/210_000 (dedup:0) compressed=true
```

阈值（超了写日志 + 状态栏提示）：单页均值 > 500 KB、单字体 > 3 MB、图片占比 > 70%。

### 5.5 验收

- fixture：`md-50k-cn.md`（5 万字，含标题/列表/代码/表格/一张图）。
- **体积上限 ≤ 2 MB**，页数不限；导出 ≤ 10 s。
- 审计日志必须出现，且 `compressed=true`、无单字体 > 3 MB。
- 这条写进 D-export 的当天验收；任何一次回归超过 2 MB 即阻断发布。

### 5.6 26 MB 的诊断（拿不到文件就不编）

若拿到那个文件，按顺序跑：`pdfinfo`（页数/生产者）→ `pdffonts`（几个字体、是否 subset、各多大）→ `pdfimages -list`（图片数/编码/总字节）→ `qpdf --json`（流是否压缩、对象复用）。判读：字体行 `emb=yes sub=no` 且体积大 → 成因 1；图片多且 `type=image` 占大头 → 成因 2/3；流 `uncompressed` → 成因 4。拿不到就用 §5.3 的清单，不猜具体数字。

### 5.7 映射与边界

- 九种 Block → Typst markup；样式走一个模板（`#set text(...)`），字体用系统字体名（缺字体则回退并提示）。
- 不做：精细分页控制、页眉页脚模板编辑、目录（可后加）、PDF/A。
- Typst 以子进程调用：`typst compile --font-path <系统字体目录?> in.typ out.pdf`；失败时保留 `.typ` 与日志，报错不静默。

---

## 6. 性能

### 6.1 指标

| 指标 | 目标 |
| --- | --- |
| 冷启动到首屏 | ≤ 2.0 s p95（基座实测 3.4–3.6 s，剪掉引擎后目标砍半） |
| 热启动到首屏 | ≤ 600 ms p95 |
| 打开 5 万字可输入 | ≤ 400 ms p95 |
| K2P | p50 ≤ 8 ms，p95 ≤ 16 ms，p99 ≤ 33 ms；>16 ms 占比 < 1% |
| 滚动 | 60 fps，掉帧率 < 1% |
| 内存（5 万字） | ≤ 200 MB |
| UI 线程长任务 | 0 次 > 50 ms |
| 增量构建（改 core 一行） | ≤ 60 s（超了就瘦身或转 B） |

### 6.2 D1–D2 的闸门

| 天 | 探针 | 判据 |
| --- | --- | --- |
| D1 | 剪枝冷构建、改一行增量、exe 体积、打开 .md | 能编能开；增量 > 60 s 即瘦身 |
| D2 | P1：5 万字约 350 段 | 打开 ≤ 400 ms；K2P p95 ≤ 16 ms；掉帧 < 1% |
| D2 | P2：5 万字**单段**（一行） | K2P p95 ≤ 16 ms，无 > 100 ms 卡顿 |
| D2 | P3：5 MB / 约 10 万字 | 能开不崩，记录降级行为 |

P2 不过就当天改方案，按顺序：① 关该段行内装饰与软换行（长行模式）→ 重测；② 超 2 万字符的段落折叠为源码块 → 重测；③ 仍不过则把「单段 5 万字可编辑」移出支持范围，第 2 天写进 README 已知限制，并把它从发布门槛删掉——明确记录，不偷偷降级。

---

## 7. 15 天

| 天 | 做什么 | 当天验收 |
| --- | --- | --- |
| D1 | 基座闸门：剪枝 + 构建 + 打开 .md | 冷构建分钟数、增量秒数、exe 体积、能否打开；A/B 结论 |
| D2 | 性能闸门：P1/P2/P3 | 数字表；P2 不过则当天执行 §6.2 分支 |
| D3 | 外壳收窄：单窗口、无侧栏、工具条、正文居中 | 浅/深两张截图 |
| D4 | Markdown 变内置 Provider，core 零引用 | `grep -ri markdown green-core/` 为空；禁用后落到 plaintext |
| D5 | 编辑闭环 + 保存契约（占用重试/报错） | 杀进程不丢稿；占用文件保存有明确错误与临时文件 |
| D6 | 文件入口 + 命名管道三路径 | 三种情况各演示一次（没跑/没开/已开） |
| D7 | Agent CLI（复用 `remote.rs::command_line`） | 脚本改文件，窗口随即可见；`--expect-hash` 冲突返回 3 |
| D8 | 热重载 + 冲突三选一（复用 `outside/`） | 两个进程同改不静默覆盖（录屏） |
| D9 | 插件 supervisor + 契约 + 清单 + 启停 | 一个 Python 插件返回 Block 树；超时被杀死并降级 |
| D10 | 真加 CSV 插件，不重编译不发版 | 改动文件数/行数；Green 零改动 |
| D11 | **导出**：Block→typ、Typst 调用、pdf_audit、阈值 | 5 万字 fixture ≤ 2 MB；审计日志一行齐全 |
| D12 | 打包 zip + VCRT 验证 + 首次运行 | 干净机器解压即用；`dumpbin`/依赖检查确认无 VCRT dll 依赖 |
| D13 | 稳定性：10 个真实文档 + 崩溃恢复 + 日志 | 检查清单全绿 |
| D14 | 释放（半天）：设置与许可页、主题沿用基座 | 许可页可打开；`THIRD-PARTY-NOTICES` 在包内 |
| D15 | 发布：README、LICENSE、示例插件、录屏、tag | 一个 zip + 一页说明 |

> 没有整天缓冲：插件多花 1.5 天、导出多花 1.5 天，都从设置/主题与缓冲里挤。再出意外就砍「插件启停 UI」（退化为改 manifest）。

---

## 8. 风险

| # | 风险 | 应对 |
| --- | --- | --- |
| R1 | 剪枝后编不过或增量 > 60 s | D1 当天判；转 B（gpui 自建，+3–4 天）或瘦身（删 ely-lok 依赖，需动 15 文件） |
| R2 | 单段 5 万字不达标 | §6.2 分支当天执行 |
| R3 | Typst 导出在中文字体上踩坑（缺字体/回退/子集异常） | D11 用真实中文字体测；审计器逐字体报数；缺字体显式提示 |
| R4 | Typst 子进程启动慢（~几十 ms）或路径找不到 | 首次调用探测并缓存路径；失败时给出安装指引，不静默 |
| R5 | 15 天做不完 | 砍插件启停 UI → 再砍 D8 的差异视图 |
| R6 | 插件外部进程带来安全面 | 无沙箱，等同用户自己运行脚本；文档写明只从信任来源装 |

---

## 9. 决策

### D1 基座：ElyOffice 剪枝（推荐，已实测）

- **结论**：走 A。剪枝后能编、能开（见 §1.3）；LOK 作为编译期依赖保留，不打包 worker。
- **代价**：多编一个 Rust crate（无运行时成本）；`ely-office` 里 3,836 行 LOK 相关代码留着不用。
- **不批准的含义**：转 B，15 天里自己搭窗口/设置/打包，+3–4 天。

### D2 PDF 阅读/编辑：不做（已定）

- 只产出 PDF（导出），不读、不编辑、不预览。无替代方案。

### D3 许可：GPL-3.0-or-later（推荐）

- 事实形态（gpui 打过补丁）；义务是公开源码 + 附许可全文，你本来就满足。
- 若想留后路：不打包 gpui 补丁 → Apache-2.0，但放弃基座对 IME/渲染的修复。

### D4 插件：外部进程 + JSON（推荐）

- 满足「加格式不重编译、不发版」；语言无关；朋友写的插件不必 GPL。
- 代价：1.5 天保险费；无沙箱；只能产出 Block（不能自定义 chrome）。

### D5 范围冻结（已批准）

- 15 天只做 §1/§3/§4/§5/§7 写的东西；新想法进 `later.md`。
- 含保存契约：占用时有限重试 + 明确报错，不静默丢内容（§4.5）。
- 含导出验收线：5 万字 ≤ 2 MB（§5.5）。

---

## 附：本版刻意不写的

不写多引擎、不写协同、不写云、不写移动端、不写 PDF 阅读。它们不在 15 天里，写了只会变成承诺。
