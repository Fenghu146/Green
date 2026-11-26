# patches

`vendor/` 是「上游 + 这里的补丁」，两个序列分开，因为基线不同：

| 目录 | 基线 | 重建与核对 |
| --- | --- | --- |
| `gpui/` | crates.io 的 `gpui-pre`，版本由 `Cargo.toml` 的 `gpui = { package = "gpui-pre", version = "=…" }` 一行决定 | `scripts/vendor-gpui.sh check\|write` |
| `ely-gpui-components/` | 组件库的 pin，由 `Cargo.toml` 的 `ely-gpui-component = { git = …, rev = … }` 决定 | `scripts/vendor-components.sh check\|write` |
| `crates-io/` | 五个从 crates.io 换进来的 crate 各自的版本与 tarball hash，写在 `scripts/vendor-crates.sh` 里 | `scripts/vendor-crates.sh check\|write` |

三个脚本的 `check` 都是真重放：下载/克隆基线，按序应用全部补丁，再与 `vendor/` **逐文件比对**。
不过就说不过，CI 每次跑。`write` 用同样的过程重写 `vendor/`。

## 每个补丁的头两行

```text
baseline: …
bump: …
```

- **`baseline`**：补丁能干净应用的基线。gpui 写版本号——这些 crate 不带 `.cargo_vcs_info.json`，
  没有记录 VCS commit，所以基线身份是「名字 + 版本 + tarball 的 sha256」，hash 在
  `scripts/vendor-gpui.sh` 里。组件库写 pin 的完整 sha，文件名也带它的前七位。
  脚本核对这两处与 `Cargo.toml` 一致，不一致就拒绝应用。
- **`bump`**：撞上游时怎么现场判断。两句话：一句「上游已经这样了 → 删掉本补丁」，
  一句「签名或结构变了 → 重写，不要重放」。

## 版本号只有一个来源

`Cargo.toml` 的 pin 行是唯一权威：脚本从那里读版本与 rev，`docs/compatibility.md` 由
`scripts/compatibility.py` 从同一处写出。补丁头写 baseline 是为了让人判断，脚本核对它与
pin 一致；不要在任何地方再抄一份版本号。

## 加一个补丁

1. 改 `vendor/` 里的文件——构建真正读的是它。
2. 把改动落成补丁放进对应目录，编号接着来；组件库的带基线前缀（`fc71e96-0002-…`）。
   头两行照上节写。
3. `scripts/vendor-*.sh check` 必须仍然通过：它重放序列并与 `vendor/` 逐文件比对。
4. 跑 `python scripts/compatibility.py` 更新 `docs/compatibility.md`。

## 现在有什么

- `gpui/0001–0015`：来自 ElyOffice `3ba2adc` 的 `patches/gpui`，改动原样；`baseline:` 与
  `bump:` 两行是本仓库加的。
- `gpui/0016`：Green 自己的 `shape-count` 特性（`--features shape-count`），S2 用它证明
  折叠前后没有重新排版。
- `ely-gpui-components/fc71e96-0001-marked.patch`：`==高亮==` 在 `tree` 读取里成为 `Mark` run。
- `ely-gpui-components/fc71e96-prune.txt`：组件库自己仓库的文档、CI、api 快照等 266 个路径，
  vendor 不保留。它不是补丁，是 vendoring 政策；脚本按这份清单删，删完再应用补丁。
- `crates-io/`：五个 crate 各一条 `0001`，都是上游行为不对的小修（对象路径回收、GFM 脚注链接、
  引号内含 `@` 的地址、AT-SPI 缓存信号与注销）。文件名带 crate 与版本，`baseline:` 行带
  tarball 的 sha256。

`vendor/` 目前仍在 git 里（2718 个文件），两个 `check` 比对的就是它。改成「不入库、构建时重建」
只需要把两个 `write` 接进构建之前，并 `git rm -r --cached vendor`——那是主线的决定。
