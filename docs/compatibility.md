# 兼容性

这个文件说构建实际依赖了什么：Green 的版本、gpui 的版本、组件库的 pin，以及两边各站在第几个补丁上。

下面两行由脚本写出：改依赖之后跑 `python3 scripts/compatibility.py`，CI 跑 `--check` 核对。
`vendor/` 是否真的是「上游 + 补丁」由 `scripts/vendor-gpui.sh check` 与
`scripts/vendor-components.sh check` 核对，两个脚本在 CI 里每次都跑。

<!-- 下两行由 scripts/compatibility.py 写出，勿手改 -->
版本：Green 0.1.17。
依赖：gpui-pre 0.3.14（patches/gpui 0001–0016，16 个）；组件库 fc71e96（patches/ely-gpui-components 0001，1 个，裁掉 266 个只属于它自己仓库的路径）；crates.io 的另外五个 crate（patches/crates-io 0001，5 个）。