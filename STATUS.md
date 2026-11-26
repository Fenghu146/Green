# STATUS

下次合并还会撞的地方（文件级归属）：

- `crates/ely-platform/tests/packaging.rs`、`packaging/apps.tsv`、`deny.toml`、`THIRD-PARTY-NOTICES.txt`、`scripts/notices*.sh` 归工具链：整份取它的版本，别的线只提需求，不手改许可证与通知。
- `apps/green/src/main.rs` 与 `apps/green/Cargo.toml` 两家共写（插件 / `--capture` / S2a）：冲突只加不删，合完核对 `portable::adopt()`、`capture::command_line()`、`remote::command_line()`、`ely_ui::run` 四条都在。
- `apps/green/src/view/fold_tests.rs` 与 `patches/gpui/0017-*` 归折叠线（fix/fold-relayout）：先合它；shape 门禁对帧节奏的依赖由它修，别的线不碰这条测试、不调 `keep_frame` 上限。
- `crates/ely-ui/src/{fonts,theme,opening,lib}.rs` 与 `vendor/ely-gpui-components/**` 共写：ely-ui 的字体与主题由导出线先落，S2a 的视图改动后合；vendor 一律经 `patches/` 改。
- 主线最后合；动 `Cargo.lock` 的人负责 `cargo deny` 与 notices 同步；合并验收 = `git diff origin/main` 为空 + `cargo test --workspace --no-fail-fast` 全绿。

## 工作规矩

- **合并即删 worktree，不做归档。** 分支进 main 后立刻 `git worktree remove`（不加 `--force`）；它报不干净就停下查清，别强删。删的是工作区，分支本身留在远端。
- **`D:\cargo-target\<线>` 谁建谁删，一条线收工就删自己的 target 目录。** worktree 分支合入 main 后立刻删，不做归档；删不掉先查占用，不用 `--force`。
- **`D:\cargo-target` 下的 `debug` / `release` / `release-fast` 是无主池。** 机器级 `CARGO_TARGET_DIR` 保留，它拦住"仓库里长出 `target\`"那类事故；代价是没设 per-line target dir 的构建全落进这三个扁平目录，所以只要没有构建在跑就整个删掉。判定"在跑"以 `<profile>\.cargo-lock` 能否独占打开为准（cargo 把它建在 profile 目录里，不在 target 根），再叠加无 `cargo`/`rustc` 进程、15 分钟无写入。每条线仍然必须显式设 `CARGO_TARGET_DIR=D:\cargo-target\<line>`，不设就是占着无主池。
- 回收磁盘用 `scripts/disk-prune`（dry run 先看判决），删除前脚本会重探一次锁。
- **已知债：磁盘阈值散在三处**（`.github/workflows/ci.yml` 的 runner 数字、`scripts/disk.sh` 的 30/50、`scripts/disk-prune` 的判定逻辑），迟早会分叉。应收口到一处：阈值只写在 `disk.sh`，CI 只调用它，`disk-prune` 只读它的默认值。
- **已定：CI 装 Typst 0.15.1。** test job 下载 v0.15.1 的 Windows 二进制（缓存 key 带版本号），解压后把路径写进 `GREEN_TYPST`；`crates/green-math/src/tests.rs` 的 `every_covered_sample_compiles_in_typst` 与 `crates/green-export/tests/export.rs` 的端到端测试因此真正执行。下载失败、找不到 `typst.exe` 或版本不符就让 job 红，不静默退化成跳过——真编译测试是挡住非法 Typst 的唯一防线，静默跳过等于没有这个测试。
- **事实集中到一处：`ci.yml` 只描述行为。** 不在注释里写「目前还没有 / as of ...」这类会过期的状态；当前状态、已查明原因与待决项都写在 `STATUS.md`。
- **CI 的 `cancel-in-progress` 会吞掉中间提交的信号。** `concurrency` 按 ref 分组并取消进行中的 run；频繁推 main 时，被取消的中间提交永远拿不到完整绿。发版前要一次「完整绿」，以最后那个 HEAD 的完整 run 为准，别拿被取消的 run 说事。

## 已查明的 CI 红（别再归错因）

- run `45808353158`（head `ffe922d`）的 `test` job 红**不是 `skips.sh` 假红**：第 6 步 `cargo test --workspace` 退出 101，`-p green-plugins --test process` 的 `the_shipped_log_plugin_speaks_the_same_contract_from_node` 在 `crates/green-plugins/tests/process.rs:154` 对 `Option<&[Block]>` 的 `.expect("blocks")` panic（该二进制 14 passed / 1 failed）；第 7 步 `scripts/skips.sh` 是 success，日志里没有 `SKIP:` 或 `GREEN_TYPST`。真因是插件测试失败，由另一条线修。
- 历史记录：2026-11-26 的 main 上 `test-app` job（`cargo test -p green`）因两处测试代码错误红——`apps/green/src/output/tests.rs` 调用了不是 green dev-dependency 的 `ely_pdf_engine`，`apps/green/src/desk/tests.rs` 调用了不存在的 `Desk::tree_failed`；当时 S2 工作树已有修复。该 job 故意保持启用，关掉它只会让同类破坏无人发现。这只记当时的原因，当前状态以 CI 结果为准。
## S2b 修复落地状态

- 裸标识符 `nf / ne / n2` 不再被当成 Typst 的单个未知变量：`green-math`
  把「任何裸 ASCII 字母数字串逐字符拆」写成契约并加真编译测试，提交 `fd87c3d`。
  旧 main 的转换器才会在真实笔记里报 `unknown variable nf / ne / n2`。
- 一条公式编译失败不再连坐整篇 PDF：`crates/green-export` 先进程内真编译
  每条公式，转换或编译失败都降级为字面 TeX，再编译整篇，提交 `3258d54`。
- U+200B 在写进 Typst 前从正文和公式源码两处剥掉（同一提交），PDF 文本层
  不再残留不可见字符。
- 真实笔记 6.2 / 6.4 / 6.5 现在能导出：降级 6 / 9 / 8 条公式，分别 13 / 14 / 8 页、
  415 / 474 / 311 KB，其余公式正常排版。**降级不是通过**：这三份仍不进 CI 正样本，
  等真实笔记那条线复验后再决定夹具。

## 已知债

### 组件库快照反向依赖 `green-math`

`patches/ely-gpui-components/fc71e96-0004` 给 `ely-documents` 加了
`green-math` 依赖（带 version，才能过 cargo-deny 的 wildcard 检查）。这等于让
「上游快照 + 补丁」反过来牵住我们自己的 crate：从 pin 重建 vendor 时，仓库路径必须存在。
长期方向是**依赖倒置**：组件库不改，由调用方（app）把转换好的 Typst 源码传进去，
vendor 于是只跟着 pin 走。本轮为赶 S2b 不做倒置，但改 vendor 或重建脚本前先看这条。

### S2b 阶段二：大纲标题里的公式还没接回

合并 `origin/main` 时大纲采用了 main 的折叠树（`Outline::item` + `on_fold`），
分支里旧的「标题里的行内公式画成图」接线（`Outline::drawn`）随之替换。现在含公式的标题显示源码；
转换器和 Typst 进程内渲染器都还在，第二阶段在 main 的树上重新接 `documents::inline_math` 即可。
块级 `$$…$$` 渲染不依赖这一项，已完成。

### 还叫 ely-mark / ElyMark 的地方

产品名与二进制已经是 Green：`AppId::Mark` 的 `name()` 是 `Green`、`slug()` 是 `green`、bundle id 是
`com.fenghu.green`，数据在 `%LOCALAPPDATA%\Green\green`，单实例用数据目录里的 `instance.lock` 与
`instance.door`（没有命名管道），日志是 `data/green/ely.log`，打包脚本写 `green.exe`，也不写注册表。
剩下的都在文案与内部标识里：

| 位置 | 内容 | 改名的牵连 |
| --- | --- | --- |
| `apps/green/src/remote.rs:16-26,37` | CLI 用法串把命令写成 `ely-mark …` | 只影响 `--help` 与误用时的文字；解析按参数，与名字无关 |
| `crates/ely-platform/tests/update_sources.rs:77` | 更新清单名 `ely-mark-<platform>.json` | 属发布契约：改名要同改远端资产名，旧客户端会找不到 |
| `crates/ely-ui/src/i18n/**/tree_places.rs`（en 153、zh 290、de/ja/ko/fr/vi 114、es 177） | `tree.tool.in_mark` = “Open in ElyMark” | 属于 ElyTree 的「在…中打开」；Green 不发树应用，现在看不到 |
| `crates/ely-ui/src/commands.rs:32,67`、`shell/root.rs:18,320` | action `OpenElyMark` | 内部标识，四处一起改；无数据影响 |
| `crates/ely-platform/src/git.rs:193-220` | 测试里造假路径 `ElyMark.app/.../ely-mark` | 仅测试数据；生产按 `green` 查找 |
| `tests/scripts/windows-names.sh:10,18`、`tests/perf/**`、`scripts/panics.tsv`、`docs/base/**` | 基座时代的 `-p ely-mark` 与旧路径 | 重跑这些旧脚本前要改成 `-p green`；历史记录不必改 |
| 各模块注释（`apps/green/src/*.rs`、`ely-ui/src/i18n/*/mark.rs` 首行等） | 文档字符串仍写 ElyMark | 纯文字 |

**要改就一次改完**：先定更新资产名（发布契约），再统一 action 名、CLI 文案、i18n 与注释。
数据目录已是 `Green/green`，不需要迁移；单独改一半会让测试里的字符串断言与发布脚本各说各话。

### Node 插件测试的冷启动边界

`crates/green-plugins/tests/process.rs` 的 Node 用例在 CI 上偶发 `blocks = None`：run 45808353158 失败；
同样的代码在 run 45800810110 通过，但用时 3.59 s，失败那次用时 7.47 s。该测试的清单当时没写
`timeoutMs`，走默认 3000 ms；超时后 supervisor 还要各等 2 s 关 stdout / stderr 管道。把 PATH 上的
`node` 换成延迟 2.95 s 的壳后，旧默认 3000 ms 下 10/10 失败，8000 ms 默认下 10/10 通过。
本机 Node v24.22.1 和 v22.23.2 无延迟时，两种超时各跑 30 次都是 0/30 失败。
据此这是阈值附近的 runner 差异，不是 JSON 契约、路径编码、CRLF / BOM、8 MB 截断或测试间并发。
产品默认值现在是 8000（上限仍 30000），首次与后续打开同值；测试清单不再写 `timeoutMs`，直接走默认，
`examples/plugins/log/manifest.json` 也从 3000 升到 8000。
超时降级时状态栏用 `office.error.plugin_timeout` 单独说“插件超时（N ms）……timeoutMs”，
契约错误仍走 `office.error.open`；测试断言打印 `View` 的 reason，`Timeout(...)` 与 `Exit` / `InvalidJson`
一眼可分。以后再红先看 reason 与耗时，不要改插件输出。
