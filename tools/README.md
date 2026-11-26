# tools

两个独立的小程序，不随产品发布，也不在 `crates/` 里：`cargo test --workspace` 不含它们，
它们自己跑 `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`（见 `.github/workflows/ci.yml`）。

`tools/Cargo.toml` 带一个空的 `[workspace]`，所以是独立包，不需要往根 `Cargo.toml` 的成员里加。
没有任何依赖：许可与公告扫描看不到它。

## 夹具：`fixtures`

```sh
cargo run --manifest-path tools/Cargo.toml --bin fixtures            # 写夹具与清单
cargo run --manifest-path tools/Cargo.toml --bin fixtures -- --check  # 逐字节比对已提交的副本
```

固定种子 `0x677265656E6F7465`，同一份代码在任何机器上写出同样的字节；清单在
`fixtures/perf/GENERATED.md`（字节、字符、行、段、SHA-256）。**改夹具就改生成器再重跑**，
不要手改生成出来的文件。

| 文件 | 内容 | 用途 |
| --- | --- | --- |
| `fixtures/perf/50k-paragraphs.md` | 5 万字、350 段 | §9「打开 5 万字可输入」「折叠/展开（5 万字）」的正常形态 |
| `fixtures/perf/50k-single-paragraph.md` | 5 万字、单段一行、无换行 | §9 的对抗用例；换行与行内装饰的退路从这里量 |
| `fixtures/perf/5mb-large.md` | 5 MB；正文约 10 万字 | 表格与代码块占大头；打开、滚动、内存的极端形态 |
| `fixtures/perf/homework-28-formulas.md` | 28 条公式（19 行内 + 9 块级，另含表格） | §7 导出验收的形态：`作业.md` |
| `fixtures/real/images/*.png` | 两张示意图 | `fixtures/real/` 里的文档引用；自造，无版权内容 |

100,000 字的正文塞不进 5 MB 的字节：中文按 UTF-8 是 3 字节，10 万字约 300 KB。所以
`5mb-large.md` 由三部分组成——正文约十万字、一张两万多行的长表、一段补足到整五 MB 的日志块；
清单里分别给出总字符数与正文字数。补白的最后一行是等长的 `x`，它只负责字节数，不是内容。

## 度量：`bench`

这一轮只做离线分析：产品把原始样本导成 JSONL，`bench` 读它，按 `docs/design.md` §9 判定。

```sh
cargo run --manifest-path tools/Cargo.toml --bin bench -- stats raw.jsonl
cargo run --manifest-path tools/Cargo.toml --bin bench -- check raw.jsonl --json out.json
```

### 输入

一行一个 JSON 对象，UTF-8，字段只认这两个：

```json
{"metric":"k2p","ms":7.4}
{"metric":"k2p","ms":12.1,"at":"2026-11-26T10:00:00.123Z","doc":"50k-paragraphs.md","seq":7}
```

- `metric`：下表的名字，**必填**，拼错即报错（不认识的指标不会被悄悄跳过）。
- `ms`：样本，单位随指标（毫秒，`memory` 是 MB，`fold.shape` 是调用次数），**必填**，非负。
- 其余字段（`at`、`doc`、`seq`…）原样忽略，产品想带什么就带什么。
- 空行与 `#` 开头的行跳过，便于在日志里嵌一段。

### 指标

| `metric` | §9 的要求 | 样本 |
| --- | --- | --- |
| `k2p` | p50 ≤ 8 ms、p95 ≤ 16 ms、p99 ≤ 33 ms、>16 ms 占比 < 1% | 每次按键到像素 |
| `scroll` | 掉帧率 < 1%（帧 > 16.67 ms 算掉帧） | 每帧的绘制耗时 |
| `fold` | ≤ 16 ms（取最大值） | 每次折叠/展开 |
| `fold.shape` | 0（取最大值） | 一次折叠期间的 shape 调用次数 |
| `open` | p95 ≤ 400 ms | 打开 5 万字到可输入 |
| `cold-start` | p95 ≤ 2000 ms | 冷启动到首屏 |
| `hot-start` | p95 ≤ 600 ms | 热启动到首屏 |
| `tab-switch` | p95 ≤ 100 ms | 切换标签 |
| `long-task` | 超过 50 ms 的 **0 次** | UI 线程每个长任务 |
| `memory` | ≤ 200 MB（取最大值，单位 MB） | 单标签内存 |
| `incremental-build` | p95 ≤ 60000 ms | 增量构建 |

**分位数用最近秩**：把样本排序，取第 `ceil(p/100 × n)` 个（不插值）。样本少时它向上取整，
比如 3 个样本的 p95 就是最大值。>阈值 用严格大于；占比 < 1% 也是严格的（正好 1% 不算过）。

`check` 对没测到的指标打 `not measured` 而不判失败；`stats` 只出数字。退出码：
0 全部达标，1 有指标超预算，2 用法或输入错误。

### 与产品的接口

产品这一轮不需要接：任何能写出上面那种行的东西都能喂进来。将来接的时候，导出端负责
「一次按键一条 `k2p`、一帧一条 `scroll`」，`bench` 不猜、不补、不算产品该算的东西。
