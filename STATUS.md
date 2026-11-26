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
