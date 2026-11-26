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
## 已知债

### 真实笔记夹具：能过才进 CI，其余进已知债

九份本人笔记的脱敏骨架在 `fixtures/real/skeletons/`；原文在 `fixtures/real/local/`（`.gitignore` 排除），
不入库。2026-11-26 逐份实测：6 份导出成功、3 份整份失败；成功里面有 3 份的 PDF 公式退化成源码。
**不用 xfail 把坏结果钉成基线。** CI 正样本白名单只有 `fixtures/real/ci-positive.txt` 中的三份：
`指令系统`、`数的表示与运算｜完整复习笔记`、`中央处理器CPU`；它们的屏幕显示与导出均已核对通过。
其余六份不进 CI，按下面的债逐条修；修好一份就把那一份骨架加进白名单，CI 的真实夹具随之多一份。

| 文档（骨架） | 用户会看到什么 | 归属模块与要做的事 |
| --- | --- | --- |
| `6.2`、`6.4`、`6.5` | 点导出后整份失败，报 `typst exit code: 1` / `unknown variable: nf`、`ne`、`n2`，拿不到任何 PDF | `crates/green-math/src/lib.rs` 的 `identifiers()`：裸标识符规则只拆 `B052H` 这种“大写字母+数字”，没覆盖字母+字母（`nf`、`ne`）和小写字母+数字（`n2`）。补规则并在 `crates/green-math/src/tests.rs` 加断言。这是转换缺陷，不是环境问题 |
| 同上三份 | 一条坏公式把整篇导出炸掉，而不是只坏那一条 | `crates/green-export/src/export.rs`：`typst::source()` 的转换失败已有单条降级，`compiler.compile()` 的 Typst 编译失败却直接 `?` 返回。按 `docs/design.md` §7.8 让坏公式降级为字面文本、其余照常导出；这是设计要求的实现缺口，不是新需求 |
| `6.1`、`6.3`、`数的表示与计算` | PDF 里公式直接显示成源码：`\displaystyle\sum`、`\dfrac`、`\begin{cases}`、`\bigl`、`\mathbb{R}`、`\begin{align*}`；屏幕上都正常 | `crates/green-math/src/lib.rs` 的 TeX→Typst 规则缺这些宏；`crates/green-export/src/typst.rs` 的 `settled()` 只兜住转换器返回 Err，不兜“转成源码但编译通过”。补规则后要用 PDF 文本层断言源码不出现 |
| 8/9 份（除 `数的表示与计算`）；`6.1`、`6.3` 的 PDF 文本层残留最多 | 屏幕上没事，但从 PDF 复制文字会带出不可见的 U+200B，搜索也可能对不上 | 导出线生成 Typst 文本的入口（`crates/green-export/src/typst.rs` 一带）清理零宽字符；屏幕渲染已正常，不要在编辑器层改 |
| 九份骨架的侧栏交互（重点抽 `中央处理器CPU` 的 268 个标题、`指令系统` 的 107 个标题） | 折叠箭头与标题层级在静态截图里可见，但侧栏没展开，点击跳转从未验证 | `apps/green/src/side.rs`、`apps/green/src/keys.rs` 的 `toggle_outline`；`--capture` 只有静帧，必须另排一次真窗口/窗口测试：展开大纲、核对层级与折叠箭头、点击标题跳转 |

每修好一条，把对应骨架加进 `fixtures/real/ci-positive.txt`，并从上面的待修表里移出；没通过就留在债里。
白名单是唯一入口，生成骨架后仍要过 `python tools/redact.py --verify fixtures/real/skeletons`。
本轮按边界不改 `.github/`，CI 接线由 CI 线读这份白名单。

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
