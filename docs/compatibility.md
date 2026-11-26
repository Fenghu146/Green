# 兼容性

这个文件说构建实际依赖了什么：Green 的版本、gpui 的版本、组件库的 pin，以及两边各站在第几个补丁上。

下面两行由脚本写出：改依赖之后跑 `python scripts/compatibility.py`，CI 跑 `--check` 核对。
`vendor/` 是否真的是「上游 + 补丁」由 `scripts/vendor-gpui.sh check`、
`scripts/vendor-components.sh check` 与 `scripts/vendor-crates.sh check` 核对，三个脚本在 CI 里每次都跑。

口径：**`patches/` 是来源，`vendor/` 是快照，CI 保证两者一致**。`vendor/` 入库，所以
`git clone` 之后不联网也能构建；快照之所以可信，是因为每一次 CI 都把它按补丁重放一遍再逐文件比对。

<!-- 下两行由 scripts/compatibility.py 写出，勿手改 -->
版本：Green 0.1.17。
依赖：gpui-pre 0.3.14（patches/gpui 0001–0016，16 个）；组件库 fc71e96（patches/ely-gpui-components 0001，1 个，裁掉 266 个只属于它自己仓库的路径）；crates.io 的另外五个 crate（patches/crates-io 0001，5 个）。