# STATUS

下次合并还会撞的地方（文件级归属）：

- `crates/ely-platform/tests/packaging.rs`、`packaging/apps.tsv`、`deny.toml`、`THIRD-PARTY-NOTICES.txt`、`scripts/notices*.sh` 归工具链：整份取它的版本，别的线只提需求，不手改许可证与通知。
- `apps/green/src/main.rs` 与 `apps/green/Cargo.toml` 两家共写（插件 / `--capture` / S2a）：冲突只加不删，合完核对 `portable::adopt()`、`capture::command_line()`、`remote::command_line()`、`ely_ui::run` 四条都在。
- `apps/green/src/view/fold_tests.rs` 与 `patches/gpui/0017-*` 归折叠线（fix/fold-relayout）：先合它；shape 门禁对帧节奏的依赖由它修，别的线不碰这条测试、不调 `keep_frame` 上限。
- `crates/ely-ui/src/{fonts,theme,opening,lib}.rs` 与 `vendor/ely-gpui-components/**` 共写：ely-ui 的字体与主题由导出线先落，S2a 的视图改动后合；vendor 一律经 `patches/` 改。
- 主线最后合；动 `Cargo.lock` 的人负责 `cargo deny` 与 notices 同步；合并验收 = `git diff origin/main` 为空 + `cargo test --workspace --no-fail-fast` 全绿。

## 已知债

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
