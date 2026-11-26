# green-bench

> **已被 v0.2 取代。** 这里测的是 v0.1 的原生排版栈（ropey + cosmic-text）。v0.2 改用基座的 GPUI Markdown 组件，这组数字不再代表 v0.2 的性能；v0.2 的探针见 `research/probe/`（D1–D2 建立）。保留此目录仅作历史记录。

设计文档 v0.1 §9.2 里那组数字的来源。**不是产品代码**，不进发布包。

```powershell
cargo run --release
```

它做六件事：

1. 生成一份 50,130 字符的中文 Markdown（348 块，含标题/段落/列表/代码块/表格）。
2. `pulldown-cmark` 全量解析计时；`comrak` 建 AST 计时（对照）。
3. `ropey` 4000 次随机单字符插入/删除的延迟分位。
4. `cosmic-text` 排版：整篇一个 buffer vs 按块；单块重排；首屏（填满 1200px）。
5. 16 个 `FontSystem` 各自排版（用于证明「天真并行是负优化」）。
6. 进程工作集（Windows `K32GetProcessMemoryInfo`）。

注意事项：

- `cosmic-text` 只是**排版的参照实现**，不代表 Green 的最终选型（设计文档推荐 `parley`）。
- 只做同机前后对比，不要跨机器比绝对值；结果受字体、CPU 频率、杀软影响。
- `FontSystem::new()` 的 180–195 ms 是字体枚举成本；真实实现应缓存字体索引，不宜照抄这里的初始化方式。

