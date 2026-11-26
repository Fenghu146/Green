# Green

单一事实来源：设计在 `docs/design.md`（v0.4）。改动技术栈或范围时同步改本文件与 `docs/design.md`。

## 产品

一个离线的 Markdown 笔记应用，Windows 优先，单进程 `green.exe`。内容只存在于磁盘文件里：无数据库、无自有格式、无 sidecar。折叠与大纲只是视图状态，永不写入文件。界面按 `design.md` §8 的骨架，深浅两套主题。

## 结构

```text
apps/green/   唯一的可执行文件：外壳、标签、工具条、文档会话
crates/       基座（ElyOffice 剪枝）：ely-ui（外壳、主题、设置、i18n、字体）
              ely-platform（AppId、数据目录、单实例、日志）
              ely-office（标签、打开/保存、会话、恢复）
              ely-lok / ely-office 的 LOK 耦合保留不动：只作编译期依赖，不打包 worker、不启动
              ely-files  ely-ipc  ely-start  ely-mail-engine  ely-mail-fake
assets/fonts/ 随包字体：Noto Sans CJK SC Regular/Bold、Noto Sans Math、Noto Sans Mono（OFL-1.1），GPUI 与 Typst 共用
vendor/       经 [patch.crates-io] 换入的上游副本（gpui-pre 等），不改上游语义
packaging/    三端打包脚本；S6 使用
docs/         design.md 与历史；docs/base/ 是基座原文
```

## 规则（沿用基座）

1. 界面优先用 Ely-GPUI-Components；通用交互回组件库，不在应用里复制。
2. 出错就暴露，不吞错、不写兜底、不用默认值遮挡缺失数据。
3. 修根因，不打补丁式小修。
4. 关键路径有日志（`log`），日志不含正文与密钥。
5. 文件不超过 500 行；注释一行，只在必要时写。
6. 无 dead code，无过期代码，不过度设计。
7. 未验证的能力不写成已完成。

## 命令

```text
cargo build -p green                 调试构建（产物在仓库 target/）
cargo build -p green --release       发布构建
cargo test -p ely-platform           数据目录、AppId、单实例
ELY_SMOKE=1 target/debug/green.exe FILE   画一帧即退出
ELY_APPEARANCE=light|dark|system     覆盖主题
ELY_LOG=info                         日志级别
```

发布包布局：`green.exe` 与 `fonts/` 同级；程序先找可执行文件旁的 `fonts/`，找不到才用源码树的 `assets/fonts/`。
