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
- **待决：CI 里要不要装 Typst。** `crates/green-math/src/tests.rs` 的 `every_covered_sample_compiles_in_typst` 与 `crates/green-export/tests/export.rs` 的端到端测试都读 `GREEN_TYPST`，未设时直接返回；它们的 eprintln 不是 `SKIP:`，所以 `skips.sh` 不会因此红。是否在这条 test job 安装 Typst 并导出路径，还没决定。
- **事实集中到一处：`ci.yml` 只描述行为。** 不在注释里写「目前还没有 / as of ...」这类会过期的状态；当前状态、已查明原因与待决项都写在 `STATUS.md`。

## 已查明的 CI 红（别再归错因）

- run `45808353158`（head `ffe922d`）的 `test` job 红**不是 `skips.sh` 假红**：第 6 步 `cargo test --workspace` 退出 101，`-p green-plugins --test process` 的 `the_shipped_log_plugin_speaks_the_same_contract_from_node` 在 `crates/green-plugins/tests/process.rs:154` 对 `Option<&[Block]>` 的 `.expect("blocks")` panic（该二进制 14 passed / 1 failed）；第 7 步 `scripts/skips.sh` 是 success，日志里没有 `SKIP:` 或 `GREEN_TYPST`。真因是插件测试失败，由另一条线修。
