# 许可与资产审计

S6 的一部分。对象是 `green.exe` 实际链接的依赖（`Cargo.lock` 解析出的 normal + build 图），
以及随包的字体、图标和示例插件。**本文件是工程审计，不是法律意见。**

生成依赖清单的命令（`green` 的图，不含 dev-dependencies）：

```sh
cargo tree -p green -e normal,build --prefix none --format "{p}|{l}"
```

`THIRD-PARTY-NOTICES.txt` 由 `cargo about generate packaging/notices.hbs` 生成（整个 workspace，
覆盖锁文件；比 `green.exe` 实际用到的多，但不会漏）。

## 白名单

MIT / Apache-2.0 / BSD（2、3、0 条款）/ ISC / Zlib / Unicode-3.0 / CC0-1.0 / OFL-1.1。
双许可只要有一个落在白名单内即算通过（表里写选中的那个）。AGPL / MPL / SSPL 一律标红。

## 结论

- `green.exe` 的图：**582** 个包，其中 9 个是本项目自己的 crate（GPL-3.0-or-later），
  **573 个第三方**。
- 第三方里 **569 个在白名单内**；**3 个需要单独说明**；**1 个标红**（MPL-2.0）。
- 整个 `Cargo.lock`（1081 个包，含其他 workspace 应用与 dev 依赖）另有 6 个 MPL-2.0 的包，
  **不在 `green.exe` 里**；没有 AGPL / SSPL。

| 判定 | 数量 | 说明 |
| --- | ---: | --- |
| 白名单 | 569 | 见下方完整表 |
| 需说明 | 3 | `clipboard-win`、`error-code`（BSL-1.0）；`webpki-roots`（CDLA-Permissive-2.0） |
| 红（MPL-2.0） | 1 | `option-ext 0.2.0` |

## 红：`option-ext 0.2.0`（MPL-2.0）

依赖链：`directories 6.0.0` → `dirs-sys 0.5.0` → `option-ext 0.2.0`，被 `ely-platform` 与
`ely-office` 用来找数据目录。

- MPL-2.0 是**文件级**弱 copyleft：它不要求 Green 整体改许可证，但对 `option-ext` 的源文件
  有“可获取对应源码”的义务。我们没有修改它。
- 现在的 `deny.toml` 把 MPL-2.0 放进了 allow，所以 `cargo deny` 不会报；本审计按你的白名单
  标红。两处口径需要统一。
- 两条路，二选一：
  1. **接受并履行**：在包内附 `option-ext` 的源码（单文件）或书面源码提供声明；`cargo about`
     已经把 MPL 全文放进 NOTICES。
  2. **移除**：把 `directories` 换成 MIT/Apache 的 `etcetera`（或自己读 `%LOCALAPPDATA%`），
     依赖链就消失。这是 `crates/ely-platform` 的改动，不在本线范围内。

## 需说明

| crate | 版本 | 许可证 | 义务 |
| --- | --- | --- | --- |
| `clipboard-win` | 5.4.3 | BSL-1.0 | Boost 许可：保留版权与许可文本，不背书 |
| `error-code` | 3.4.0 | BSL-1.0 | 同上 |
| `webpki-roots` | 1.0.11 | CDLA-Permissive-2.0 | Mozilla 根证书数据；保留声明，不暗示背书；`deny.toml` 已有例外 |

## 锁文件里不在 `green.exe` 的 MPL 包

`cbindgen`、`cssparser`、`cssparser-macros`、`dtoa-short`、`dwrote`、`selectors`。
它们属于其他 workspace 应用或构建工具，**不随 `green.exe` 分发**；如果以后打包那些应用，
要按同样的口径处理。

## 资产

### 字体（`assets/fonts/`，只读核对，未改动）

| 字体 | 文件 | 版权行 | OFL |
| --- | --- | --- | --- |
| Noto Sans CJK SC Regular | `NotoSansCJKsc-Regular.otf` | `© 2014-2021 Adobe (http://www.adobe.com/)`（字体 name 表） | **缺**：`OFL.txt` 是通用文本，没有这行版权 |
| Noto Sans CJK SC Bold | `NotoSansCJKsc-Bold.otf` | 同上 | 同上 |
| Noto Sans Math | **缺** | — | **缺** |
| Noto Sans Mono | **缺** | — | **缺** |

结论：**Noto Sans Math 与 Noto Sans Mono 完全缺失**；已有的 CJK 两个字重共用一个没有版权行的
`OFL.txt`。按“每个字体族的 OFL 必须带自己的版权行”，这两项都要补。`assets/fonts/` 归 S5，
本线没有加文件。

### 图标

`assets/icon.svg`、`assets/icon-small.svg` 是原创母版；`assets/icons/green.ico`、`green.png`
由 `scripts/icon.py` 生成。均为本项目作品，GPL-3.0-or-later。`assets/icons/` 里其余
`ely-*` 图标是基座 ElyOffice 的素材（MIT），Green 的包不带它们。

### 示例插件

`examples/plugins/csv`、`tsv`、`log` 是本项目代码，GPL-3.0-or-later。它们是独立进程，
不编进 `green.exe`。

## 我们的义务清单

1. **GPL-3.0-or-later（本项目代码）**：包内附 GPL 全文；提供对应源码（仓库与 tag）；
   不额外加限制。
2. **MIT / Apache-2.0 / BSD / ISC / Zlib / Unicode / CC0**：保留版权与许可文本
   （`THIRD-PARTY-NOTICES.txt`），不暗示作者背书。
3. **OFL-1.1（字体）**：每个字体族带自己的版权行与 OFL 全文；不单独售卖字体；
   不改名、不改字形（Reserved Font Name）。
4. **MPL-2.0（`option-ext`）**：提供该文件的可获取源码，或书面源码提供声明。
5. **BSL-1.0 / CDLA-Permissive-2.0**：保留声明，不背书。

## 插件与 GPL

插件是**独立进程**，只经 stdin/stdout 的 JSON 与 Green 通信，不链接 Green 的代码。
按通常的工程判断，这**通常不构成衍生作品**，因此第三方插件可以不是 GPL。
这是工程判断，**不是法律意见**；真要对外分发收费插件前，应由律师确认。

## 完整清单（`green.exe` 的图，573 个第三方 crate）

“范围”是 `运行` 或 `构建`（构建期依赖不随包分发）。判定里的许可证是双许可中选中的那个。

| crate | 版本 | 许可证（声明） | 范围 | 判定 |
| --- | --- | --- | --- | --- |
| `ab_glyph` | 0.2.32 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `ab_glyph_rasterizer` | 0.1.10 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `accesskit` | 0.24.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `accesskit_consumer` | 0.38.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `accesskit_windows` | 0.34.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `adler2` | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | 运行 | ✓ 0BSD |
| `aead` | 0.6.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `aes` | 0.9.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `aes-gcm` | 0.11.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `aho-corasick` | 1.1.5 | Unlicense OR MIT | 运行 | ✓ MIT |
| `alacritty_terminal` | 0.26.0 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `anstream` | 1.0.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `anstyle` | 1.0.14 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `anstyle-parse` | 1.0.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `anstyle-query` | 1.1.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `anstyle-wincon` | 3.0.11 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `anyhow` | 1.0.104 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `approx` | 0.5.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `arboard` | 3.6.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `arrayref` | 0.3.9 | BSD-2-Clause | 运行 | ✓ BSD-2-Clause |
| `arrayvec` | 0.7.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `async-channel` | 2.5.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `async-task` | 4.7.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `atomic-waker` | 1.1.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `az` | 1.3.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `backtrace` | 0.3.76 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `barcoders` | 2.0.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `base64` | 0.22.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `base64` | 0.23.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `biblatex` | 0.12.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `bincode` | 1.3.3 | MIT | 运行 | ✓ MIT |
| `bit-set` | 0.8.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `bit-vec` | 0.8.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `bit_field` | 0.10.3 | Apache-2.0/MIT | 运行 | ✓ Apache-2.0 |
| `bitflags` | 1.3.2 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `bitflags` | 2.13.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `block-buffer` | 0.10.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `block-buffer` | 0.12.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `bumpalo` | 3.20.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `by_address` | 1.2.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `bytemuck` | 1.25.2 | Zlib OR Apache-2.0 OR MIT | 运行 | ✓ Zlib |
| `bytemuck_derive` | 1.12.1 | Zlib OR Apache-2.0 OR MIT | 运行 | ✓ Zlib |
| `byteorder` | 1.5.0 | Unlicense OR MIT | 运行 | ✓ MIT |
| `byteorder-lite` | 0.1.0 | Unlicense OR MIT | 运行 | ✓ MIT |
| `bytes` | 1.12.1 | MIT | 运行 | ✓ MIT |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `chinese-number` | 0.7.8 | MIT | 运行 | ✓ MIT |
| `chinese-variant` | 1.1.6 | MIT | 运行 | ✓ MIT |
| `chrono` | 0.4.45 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ciborium` | 0.2.2 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `ciborium-io` | 0.2.2 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `ciborium-ll` | 0.2.2 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `cipher` | 0.5.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `citationberg` | 0.7.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `clipboard-win` | 5.4.3 | BSL-1.0 | 运行 | ⚠ 需说明 |
| `cmov` | 0.5.4 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `cobs` | 0.3.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `codex` | 0.3.0 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `color` | 0.3.3 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `color_quant` | 1.1.0 | MIT | 运行 | ✓ MIT |
| `colorchoice` | 1.0.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `comemo` | 0.5.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `comemo-macros` | 0.5.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `concurrent-queue` | 2.5.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `const-oid` | 0.10.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `convert_case` | 0.10.0 | MIT | 运行 | ✓ MIT |
| `core_detect` | 1.0.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `core_maths` | 0.1.1 | MIT | 运行 | ✓ MIT |
| `cpubits` | 0.1.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `cpufeatures` | 0.2.17 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `cpufeatures` | 0.3.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crc32fast` | 1.5.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crossbeam-deque` | 0.8.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crossbeam-epoch` | 0.9.21 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crossbeam-utils` | 0.8.23 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crypto-common` | 0.1.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `crypto-common` | 0.2.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `csv` | 1.4.0 | Unlicense/MIT | 运行 | ✓ MIT |
| `csv-core` | 0.1.13 | Unlicense/MIT | 运行 | ✓ MIT |
| `ctr` | 0.10.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ctutils` | 0.4.6 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `cursor-icon` | 1.2.0 | MIT OR Apache-2.0 OR Zlib | 运行 | ✓ MIT |
| `data-url` | 0.3.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `deranged` | 0.5.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `derive_more` | 2.1.1 | MIT | 运行 | ✓ MIT |
| `derive_more-impl` | 2.1.1 | MIT | 运行 | ✓ MIT |
| `digest` | 0.10.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `digest` | 0.11.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `directories` | 6.0.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `dirs-sys` | 0.5.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `displaydoc` | 0.2.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `dunce` | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | 运行 | ✓ CC0-1.0 |
| `dyn-clone` | 1.0.20 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ecow` | 0.2.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `either` | 1.18.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ely-account` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-agent` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-calendar` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-canvas` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-charts` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-chat` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-collab` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-core` | 0.3.0 | MIT | 运行 | ✓ MIT |
| `ely-dashboard` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-debug` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-desktop` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-devtools` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-documents` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-editor` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-files` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-finance` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-generative` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-git` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-gpui-component` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-interaction` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-mail` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-maps` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-media` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-messaging` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-misc` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-onboarding` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-project` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-rendering` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-settings` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-terminal` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `ely-tooling` | 0.2.0 | MIT | 运行 | ✓ MIT |
| `embed-resource` | 3.0.11 | MIT | 构建 | ✓ MIT |
| `emojis` | 0.9.0 | (MIT OR Apache-2.0) AND Unicode-3.0 | 运行 | ✓ MIT AND Unicode-3.0 |
| `encoding_rs` | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | 运行 | ✓ Apache-2.0 AND BSD-3-Clause |
| `entities` | 1.0.1 | MIT | 运行 | ✓ MIT |
| `enum-ordinalize` | 4.4.2 | MIT | 运行 | ✓ MIT |
| `enum-ordinalize-derive` | 4.4.2 | MIT | 运行 | ✓ MIT |
| `enumn` | 0.1.14 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `env_filter` | 2.0.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `env_logger` | 0.11.11 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `equivalent` | 1.0.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `error-code` | 3.4.0 | BSL-1.0 | 运行 | ⚠ 需说明 |
| `etagere` | 0.2.15 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `euclid` | 0.22.14 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `event-listener` | 5.4.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `event-listener-strategy` | 0.5.4 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `exr` | 1.74.2 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `fancy-regex` | 0.14.0 | MIT | 运行 | ✓ MIT |
| `fancy-regex` | 0.16.2 | MIT | 运行 | ✓ MIT |
| `fastrand` | 2.5.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `fax` | 0.2.7 | MIT | 运行 | ✓ MIT |
| `fdeflate` | 0.3.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `fearless_simd` | 0.4.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `fearless_simd` | 1.3.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `flate2` | 1.1.10 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `float-cmp` | 0.9.0 | MIT | 运行 | ✓ MIT |
| `flume` | 0.12.0 | Apache-2.0/MIT | 运行 | ✓ Apache-2.0 |
| `fnv` | 1.0.7 | Apache-2.0 / MIT | 运行 | ✓ Apache-2.0 |
| `foldhash` | 0.2.0 | Zlib | 运行 | ✓ Zlib |
| `font-types` | 0.11.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `fontdb` | 0.23.0 | MIT | 运行 | ✓ MIT |
| `form_urlencoded` | 1.2.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-channel` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-core` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-executor` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-io` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-macro` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-sink` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-task` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `futures-util` | 0.3.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `g2gen` | 1.2.2 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `g2p` | 1.2.2 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `g2poly` | 1.2.2 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `generic-array` | 0.14.7 | MIT | 运行 | ✓ MIT |
| `getrandom` | 0.2.17 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `getrandom` | 0.3.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `getrandom` | 0.4.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ghash` | 0.6.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `gif` | 0.14.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `glidesort` | 0.1.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `gpui-pre` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-collections` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-derive-refineable` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-http-client` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-macros` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-platform` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-refineable` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-scheduler` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-shared-string` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-sum-tree` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-util` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-windows` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-ztracing` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `gpui-pre-ztracing-macro` | 0.3.14 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `guillotiere` | 0.7.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `half` | 2.7.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hash32` | 0.3.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hashbrown` | 0.16.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hashbrown` | 0.17.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hayagriva` | 0.10.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hayro` | 0.7.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-ccitt` | 0.3.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-ccitt` | 0.4.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-cmap` | 0.1.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-interpret` | 0.7.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-jbig2` | 0.3.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-jpeg2000` | 0.3.5 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-postscript` | 0.2.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-svg` | 0.7.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `hayro-syntax` | 0.7.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `heapless` | 0.9.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `heck` | 0.5.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `home` | 0.5.12 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `html5ever` | 0.39.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `http` | 1.5.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `http-body` | 1.1.0 | MIT | 运行 | ✓ MIT |
| `httparse` | 1.10.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hybrid-array` | 0.4.15 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `hypher` | 0.1.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `icu_collator` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_collator_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_collections` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_locale` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_locale_core` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_locale_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_locale_fallback` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_locale_fallback_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_normalizer` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_normalizer_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_properties` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_properties_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_provider` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_provider_blob` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_segmenter` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `icu_segmenter_data` | 2.4.0 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `idna` | 1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `idna_adapter` | 1.2.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `image` | 0.25.10 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `image-webp` | 0.2.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `imagesize` | 0.14.0 | MIT | 运行 | ✓ MIT |
| `indexmap` | 2.14.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `inout` | 0.2.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `inventory` | 0.3.24 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `is_terminal_polyfill` | 1.70.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `iso_country` | 0.1.4 | MIT | 运行 | ✓ MIT |
| `iso_currency` | 0.7.1 | MIT | 运行 | ✓ MIT |
| `isocountry` | 0.3.2 | MIT | 运行 | ✓ MIT |
| `itertools` | 0.14.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `jiff` | 0.2.37 | Unlicense OR MIT | 运行 | ✓ MIT |
| `jiff-core` | 0.1.1 | Unlicense OR MIT | 运行 | ✓ MIT |
| `jiff-tzdb` | 0.1.8 | Unlicense OR MIT | 运行 | ✓ MIT |
| `jiff-tzdb-platform` | 0.1.3 | Unlicense OR MIT | 运行 | ✓ MIT |
| `json5` | 1.3.1 | MIT | 运行 | ✓ MIT |
| `kamadak-exif` | 0.6.2 | BSD-2-Clause | 运行 | ✓ BSD-2-Clause |
| `keyring` | 3.6.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `kurbo` | 0.13.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `lazy_static` | 1.5.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `lebe` | 0.5.3 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `libc` | 0.2.189 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `libm` | 0.2.16 | MIT | 运行 | ✓ MIT |
| `linebender_resource_handle` | 0.1.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `linked-hash-map` | 0.5.6 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `lipsum` | 0.9.1 | MIT | 运行 | ✓ MIT |
| `litemap` | 0.8.3 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `lock_api` | 0.4.14 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `log` | 0.4.34 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `lru` | 0.18.5 | MIT | 运行 | ✓ MIT |
| `lyon_geom` | 1.0.19 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `lyon_path` | 1.0.19 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `markup5ever` | 0.39.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `memchr` | 2.8.3 | Unlicense OR MIT | 运行 | ✓ MIT |
| `memmap2` | 0.9.11 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `mermaid-rs-renderer` | 0.3.1 | MIT | 运行 | ✓ MIT |
| `mime` | 0.3.17 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `mime_guess` | 2.0.5 | MIT | 运行 | ✓ MIT |
| `miniz_oxide` | 0.8.9 | MIT OR Zlib OR Apache-2.0 | 运行 | ✓ MIT |
| `miniz_oxide` | 0.9.1 | MIT OR Zlib OR Apache-2.0 | 运行 | ✓ MIT |
| `miow` | 0.6.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `moxcms` | 0.8.1 | BSD-3-Clause OR Apache-2.0 | 运行 | ✓ BSD-3-Clause |
| `multiversion_no_op` | 1.0.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `mutate_once` | 0.1.2 | BSD-2-Clause | 运行 | ✓ BSD-2-Clause |
| `new_debug_unreachable` | 1.0.6 | MIT | 运行 | ✓ MIT |
| `notify` | 8.2.0 | CC0-1.0 | 运行 | ✓ CC0-1.0 |
| `notify-types` | 2.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num-bigint` | 0.4.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num-complex` | 0.4.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num-conv` | 0.2.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `num_cpus` | 1.17.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `once_cell` | 1.21.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `once_cell_polyfill` | 1.70.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `option-ext` | 0.2.0 | MPL-2.0 | 运行 | ✗ MPL-2.0 |
| `owned_ttf_parser` | 0.25.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `palette` | 0.7.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `palette_derive` | 0.7.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `palette_math` | 0.7.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `parking` | 2.2.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `parking_lot` | 0.12.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `parking_lot_core` | 0.9.12 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `paste` | 1.0.15 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `peniko` | 0.6.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `percent-encoding` | 2.3.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `phf` | 0.11.3 | MIT | 运行 | ✓ MIT |
| `phf` | 0.13.1 | MIT | 运行 | ✓ MIT |
| `phf_generator` | 0.11.3 | MIT | 运行 | ✓ MIT |
| `phf_generator` | 0.13.1 | MIT | 运行 | ✓ MIT |
| `phf_macros` | 0.11.3 | MIT | 运行 | ✓ MIT |
| `phf_macros` | 0.13.1 | MIT | 运行 | ✓ MIT |
| `phf_shared` | 0.11.3 | MIT | 运行 | ✓ MIT |
| `phf_shared` | 0.13.1 | MIT | 运行 | ✓ MIT |
| `pic-scale` | 0.7.14 | BSD-3-Clause OR Apache-2.0 | 运行 | ✓ BSD-3-Clause |
| `pico-args` | 0.5.0 | MIT | 运行 | ✓ MIT |
| `pin-project` | 1.1.13 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `pin-project-internal` | 1.1.13 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `pin-project-lite` | 0.2.17 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `piper` | 0.2.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `plist` | 1.10.4 | MIT | 运行 | ✓ MIT |
| `png` | 0.17.16 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `png` | 0.18.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `polling` | 3.11.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `pollster` | 0.4.0 | Apache-2.0/MIT | 运行 | ✓ Apache-2.0 |
| `polycool` | 0.4.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `polyval` | 0.7.3 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `portable-atomic` | 1.15.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `postcard` | 1.2.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `potential_utf` | 0.1.6 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `powerfmt` | 0.2.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ppv-lite86` | 0.2.21 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `precomputed-hash` | 0.1.1 | MIT | 运行 | ✓ MIT |
| `proc-macro-crate` | 3.5.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `proc-macro-hack` | 0.5.20+deprecated | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `proc-macro2` | 1.0.107 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `profiling` | 1.0.18 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `profiling-procmacros` | 1.0.18 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `psm` | 0.1.32 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `pulldown-cmark` | 0.13.6 | MIT | 运行 | ✓ MIT |
| `pulp` | 0.22.3 | MIT | 运行 | ✓ MIT |
| `pulp-wasm-simd-flag` | 0.1.1 | MIT | 运行 | ✓ MIT |
| `pxfm` | 0.1.30 | BSD-3-Clause OR Apache-2.0 | 运行 | ✓ BSD-3-Clause |
| `qoi` | 0.4.1 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `qrcode` | 0.14.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `quick-error` | 2.0.1 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `quick-xml` | 0.38.4 | MIT | 运行 | ✓ MIT |
| `quick-xml` | 0.41.0 | MIT | 运行 | ✓ MIT |
| `quick-xml` | 0.42.0 | MIT | 运行 | ✓ MIT |
| `quote` | 1.0.47 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand` | 0.8.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand` | 0.9.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand_chacha` | 0.3.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand_chacha` | 0.9.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand_core` | 0.10.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand_core` | 0.6.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rand_core` | 0.9.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ratex-font` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-font-loader` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-katex-fonts` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-layout` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-lexer` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-parser` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-svg` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-types` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `ratex-unicode-font` | 0.1.14 | MIT | 运行 | ✓ MIT |
| `raw-cpuid` | 11.6.0 | MIT | 运行 | ✓ MIT |
| `raw-window-handle` | 0.6.2 | MIT OR Apache-2.0 OR Zlib | 运行 | ✓ MIT |
| `rayon` | 1.12.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rayon-core` | 1.13.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `read-fonts` | 0.39.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `reborrow` | 0.5.5 | MIT | 运行 | ✓ MIT |
| `ref-cast` | 1.0.27 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ref-cast-impl` | 1.0.27 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `regex` | 1.13.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `regex-automata` | 0.4.18 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `regex-lite` | 0.1.9 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `regex-syntax` | 0.8.11 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `resvg` | 0.46.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `rfd` | 0.17.2 | MIT | 运行 | ✓ MIT |
| `rgb` | 0.8.53 | MIT | 运行 | ✓ MIT |
| `ring` | 0.17.14 | Apache-2.0 AND ISC | 运行 | ✓ Apache-2.0 AND ISC |
| `roman-numerals-rs` | 3.1.0 | 0BSD OR CC0-1.0 | 运行 | ✓ 0BSD |
| `roxmltree` | 0.21.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rqrr` | 0.11.0 | (MIT OR Apache-2.0) AND ISC | 运行 | ✓ MIT AND ISC |
| `rust-embed` | 8.12.0 | MIT | 运行 | ✓ MIT |
| `rust-embed-impl` | 8.12.0 | MIT | 运行 | ✓ MIT |
| `rust-embed-utils` | 8.12.0 | MIT | 运行 | ✓ MIT |
| `rust_decimal` | 1.43.0 | MIT | 运行 | ✓ MIT |
| `rustc-demangle` | 0.1.28 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `rustc-hash` | 2.1.3 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `rustls` | 0.23.46 | Apache-2.0 OR ISC OR MIT | 运行 | ✓ Apache-2.0 |
| `rustls-pki-types` | 1.15.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `rustls-webpki` | 0.103.16 | ISC | 运行 | ✓ ISC |
| `rustybuzz` | 0.20.1 | MIT | 运行 | ✓ MIT |
| `ryu` | 1.0.23 | Apache-2.0 OR BSL-1.0 | 运行 | ✓ Apache-2.0 |
| `same-file` | 1.0.6 | Unlicense/MIT | 运行 | ✓ MIT |
| `schemars` | 1.2.2 | MIT | 运行 | ✓ MIT |
| `schemars_derive` | 1.2.2 | MIT | 运行 | ✓ MIT |
| `scopeguard` | 1.2.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `seahash` | 4.1.0 | MIT | 运行 | ✓ MIT |
| `semver` | 1.0.28 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde` | 1.0.229 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_core` | 1.0.229 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_derive` | 1.0.229 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_derive_internals` | 0.30.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_json` | 1.0.151 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_path_to_error` | 0.1.20 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_spanned` | 0.6.9 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `serde_urlencoded` | 0.7.1 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `serde_yaml` | 0.9.34+deprecated | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `sha1_smol` | 1.0.1 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `sha2` | 0.11.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `simd-adler32` | 0.3.10 | MIT | 运行 | ✓ MIT |
| `simdutf8` | 0.1.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `similar` | 3.2.0 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `simplecss` | 0.2.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `siphasher` | 1.0.3 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `skrifa` | 0.42.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `slab` | 0.4.12 | MIT | 运行 | ✓ MIT |
| `slotmap` | 1.1.1 | Zlib | 运行 | ✓ Zlib |
| `smallvec` | 1.16.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `smol_str` | 0.3.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `socks` | 0.3.4 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `spin` | 0.10.1 | MIT | 运行 | ✓ MIT |
| `spin` | 0.9.9 | MIT | 运行 | ✓ MIT |
| `stable_deref_trait` | 1.2.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `stacker` | 0.1.25 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `stacksafe` | 1.0.3 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `stacksafe-macro` | 1.0.3 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `static_assertions` | 1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `strict-num` | 0.1.1 | MIT | 运行 | ✓ MIT |
| `string_cache` | 0.9.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `strum` | 0.27.2 | MIT | 运行 | ✓ MIT |
| `strum` | 0.28.0 | MIT | 运行 | ✓ MIT |
| `strum_macros` | 0.27.2 | MIT | 运行 | ✓ MIT |
| `strum_macros` | 0.28.0 | MIT | 运行 | ✓ MIT |
| `subtle` | 2.6.1 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `svg_fmt` | 0.4.5 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `svgtypes` | 0.16.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `syn` | 2.0.119 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `syn` | 3.0.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `synstructure` | 0.14.0 | MIT | 运行 | ✓ MIT |
| `syntect` | 5.3.0 | MIT | 运行 | ✓ MIT |
| `sys-locale` | 0.3.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `system-fonts` | 0.1.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `taffy` | 0.15.1 | MIT | 运行 | ✓ MIT |
| `tempfile` | 3.27.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `tendril` | 0.5.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `thin-vec` | 0.2.21 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `thiserror` | 1.0.69 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `thiserror` | 2.0.21 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `thiserror-impl` | 1.0.69 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `thiserror-impl` | 2.0.21 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `tiff` | 0.11.3 | MIT | 运行 | ✓ MIT |
| `time` | 0.3.55 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `time-core` | 0.1.9 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `time-macros` | 0.2.32 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `tiny-skia` | 0.11.4 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `tiny-skia-path` | 0.11.4 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `tiny-skia-path` | 0.12.0 | BSD-3-Clause | 运行 | ✓ BSD-3-Clause |
| `tinystr` | 0.8.5 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `tinyvec` | 1.13.3 | Zlib OR Apache-2.0 OR MIT | 运行 | ✓ Zlib |
| `toml` | 0.8.23 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_datetime` | 0.6.11 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_datetime` | 1.1.1+spec-1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_edit` | 0.22.27 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_edit` | 0.25.15+spec-1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_parser` | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `toml_write` | 0.1.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `tracing` | 0.1.44 | MIT | 运行 | ✓ MIT |
| `tracing-attributes` | 0.1.31 | MIT | 运行 | ✓ MIT |
| `tracing-core` | 0.1.36 | MIT | 运行 | ✓ MIT |
| `ttf-parser` | 0.25.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `two-face` | 0.4.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `typed-arena` | 2.0.2 | MIT | 运行 | ✓ MIT |
| `typenum` | 1.20.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `typst` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-assets` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-eval` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-html` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-layout` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-library` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-macros` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-realize` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-svg` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-syntax` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-timing` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `typst-utils` | 0.15.1 | Apache-2.0 | 运行 | ✓ Apache-2.0 |
| `ucd-trie` | 0.1.7 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unic-langid` | 0.9.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unic-langid-impl` | 0.9.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unic-langid-macros` | 0.9.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unic-langid-macros-impl` | 0.9.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicase` | 2.10.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-bidi` | 0.3.18 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-bidi-mirroring` | 0.4.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `unicode-ccc` | 0.4.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `unicode-ident` | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | 运行 | ✓ MIT AND Unicode-3.0 |
| `unicode-math-class` | 0.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-normalization` | 0.1.25 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-properties` | 0.1.3 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `unicode-script` | 0.5.8 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-segmentation` | 1.13.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unicode-vo` | 0.1.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `unicode-width` | 0.2.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `universal-hash` | 0.6.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `unsafe-libyaml` | 0.2.11 | MIT | 运行 | ✓ MIT |
| `unscanny` | 0.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `untrusted` | 0.9.0 | ISC | 运行 | ✓ ISC |
| `ureq` | 3.4.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `ureq-proto` | 0.6.4 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `url` | 2.5.9 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `usvg` | 0.46.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `usvg` | 0.47.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `utf16_iter` | 1.1.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `utf8-zero` | 0.8.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `utf8_iter` | 1.0.4 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `utf8parse` | 0.2.2 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `utf_types` | 1.0.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `uuid` | 1.26.1 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `vello_common` | 0.0.8 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `vello_cpu` | 0.0.8 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `vte` | 0.15.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `waker-fn` | 1.2.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `walkdir` | 2.5.0 | Unlicense/MIT | 运行 | ✓ MIT |
| `wasmi` | 1.1.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `wasmi_collections` | 1.1.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `wasmi_core` | 1.1.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `wasmi_ir` | 1.1.0 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `wasmparser` | 0.239.0 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `web-time` | 1.1.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `web_atoms` | 0.2.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `webpki-roots` | 1.0.11 | CDLA-Permissive-2.0 | 运行 | ⚠ 需说明 |
| `weezl` | 0.1.12 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `which` | 8.0.6 | MIT | 运行 | ✓ MIT |
| `winapi` | 0.3.9 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `winapi-util` | 0.1.11 | Unlicense OR MIT | 运行 | ✓ MIT |
| `windows` | 0.62.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-collections` | 0.3.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-core` | 0.62.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-future` | 0.3.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-implement` | 0.60.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-interface` | 0.59.3 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-link` | 0.2.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-numerics` | 0.3.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-registry` | 0.6.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-result` | 0.4.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-strings` | 0.5.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-sys` | 0.59.0 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-sys` | 0.60.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-sys` | 0.61.2 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-targets` | 0.52.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-targets` | 0.53.5 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows-threading` | 0.2.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows_x86_64_msvc` | 0.52.6 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `windows_x86_64_msvc` | 0.53.1 | MIT OR Apache-2.0 | 运行 | ✓ MIT |
| `winnow` | 0.7.15 | MIT | 运行 | ✓ MIT |
| `winnow` | 1.0.4 | MIT | 运行 | ✓ MIT |
| `write16` | 1.0.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `writeable` | 0.6.4 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `xmlwriter` | 0.1.0 | MIT | 运行 | ✓ MIT |
| `yaml-rust` | 0.4.5 | MIT/Apache-2.0 | 运行 | ✓ MIT |
| `yoke` | 0.8.4 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `yoke-derive` | 0.8.4 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zerocopy` | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT | 运行 | ✓ BSD-2-Clause |
| `zerocopy-derive` | 0.8.58 | BSD-2-Clause OR Apache-2.0 OR MIT | 运行 | ✓ BSD-2-Clause |
| `zerofrom` | 0.1.8 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zerofrom-derive` | 0.1.8 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zeroize` | 1.9.0 | Apache-2.0 OR MIT | 运行 | ✓ Apache-2.0 |
| `zerotrie` | 0.2.5 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zerovec` | 0.11.9 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zerovec-derive` | 0.11.7 | Unicode-3.0 | 运行 | ✓ Unicode-3.0 |
| `zlib-rs` | 0.6.8 | Zlib | 运行 | ✓ Zlib |
| `zmij` | 1.0.23 | MIT | 运行 | ✓ MIT |
| `zune-core` | 0.5.3 | MIT OR Apache-2.0 OR Zlib | 运行 | ✓ MIT |
| `zune-inflate` | 0.2.54 | MIT OR Apache-2.0 OR Zlib | 运行 | ✓ MIT |
| `zune-jpeg` | 0.5.15 | MIT OR Apache-2.0 OR Zlib | 运行 | ✓ MIT |
