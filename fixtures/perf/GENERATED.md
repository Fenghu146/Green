# 生成夹具清单

由 `cargo run --manifest-path tools/Cargo.toml --bin fixtures` 写出，种子 `0x677265656E6F7465`；不要手改，改生成器再重跑。`--check` 逐字节比对已提交的副本。

| 文件 | 说明 | 字节 | 字符 | 行 | 段 | SHA-256 |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `fixtures/perf/50k-paragraphs.md` | 5 万字、350 段，正常 Markdown | 140689 | 50000 | 729 | 350 | `4ec0cc9dab2b46cb4323756447da38484a9f0916642cc6c8ba2f661afe6ebbe4` |
| `fixtures/perf/50k-single-paragraph.md` | 5 万字、单段一行、无换行（对抗用例） | 144296 | 50000 | 1 | 1 | `fd78217145613c03a8be90e724345cb28b97b97c0a5b3b2e5c8af1b63e2ad612` |
| `fixtures/perf/5mb-large.md` | 5 MB；正文 100174 字，其余为长表与日志填充 | 5000000 | 4074350 | 59672 | 821 | `e3ebb9ae9294e9bcd40fa524faaf565a42072b65c1f9572dd4f33f961d1432a6` |
| `fixtures/perf/homework-28-formulas.md` | 作业笔记，28 条公式（docs/design.md §7） | 2093 | 1468 | 83 | 30 | `2d41086190a5847adfc4ecd6388e2ed38918b67879ea7ef58339e1582c68686b` |
| `fixtures/real/images/architecture.png` | 真实文档示例用的示意图（自造，400×225） | 270313 | — | — | — | `56cb9beaf9b60d7cdec1dabfe3de853d6b2ca4eb3346180d84d7c3b756142439` |
| `fixtures/real/images/timeline.png` | 真实文档示例用的时间线图（自造，400×120） | 144198 | — | — | — | `2770803ce6ead1fac0c1e54fcac94a32fa00dbd620e5d8d1799aadba64ba6502` |
