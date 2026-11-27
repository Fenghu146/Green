# 打包与发行

给下次动 `.github/workflows/release.yml` 或 `packaging/` 的人。每条都是踩过之后写的，尤其
「macOS 两坑」与「smoke 跑解压出来的那份」。

## 包里有什么

| 平台 | 归档 | 内容 |
| --- | --- | --- |
| Windows x64 | `green-windows-x86_64.zip` | `green.exe`、`fonts/`、`data/README.txt`（便携标记）、`README.txt`、`LICENSE`、`THIRD-PARTY-NOTICES.txt`、`LICENSES/` |
| macOS arm64 / x64 | `green-macos-<arch>.zip` | `Green.app/`（`Contents/MacOS/green`、`Contents/Resources/fonts`、`icon.icns`、`Info.plist`、`LICENSE`、`LICENSES/`） |
| Linux x64 | `green-linux-x86_64.tar.gz` | `green`、`fonts/`、`green.desktop`、`green.png`、`README.txt`、`LICENSE`、`THIRD-PARTY-NOTICES.txt`、`LICENSES/` |

每个平台另附 `SHA256SUMS-<平台>.txt`；发布页再合成一份 `SHA256SUMS.txt`，并单独挂
`LICENSE`（GPL-3.0 全文）——分发二进制时带许可不是可选项。

## 字体放哪：三处不一样

`ely-ui::fonts::dir()` 的查找顺序是 `<exe>/fonts` → （仅 macOS）`<exe>/../Resources/fonts`
→ 源码树的 `assets/fonts`。所以：

- Windows / Linux：`fonts/` 就在可执行文件旁边。
- macOS：**必须放 `Contents/Resources/fonts`**。放进 `Contents/MacOS` 会让 `codesign`
  把每个字体当成“嵌套代码”，报 `code object is not signed at all. In subcomponent: …fonts/…`，
  整个 bundle 签不上。资源就放 Resources，这是 bundle 的规矩，也是这条报错的来由。

## macOS：strip 之后必须重新签名

1. 复制二进制进 `Green.app/Contents/MacOS/green`；
2. `strip -x`（Mach-O 的 strip 会把签名作废）；
3. `codesign --force --sign - "$app/Contents/MacOS/green"` —— **ad-hoc 签名**。arm64 macOS
   不运行没有签名的 Mach-O，少了这一步应用根本起不来；
4. `Contents/` 齐了之后再 `codesign --force --sign - "$app"`（bundle 的封印要覆盖最终的
   `Resources/`），然后 `codesign --verify --verbose`。

没有 Apple 开发者账号也够用：ad-hoc 签名不需要账号，它只解决“能不能启动”，不解决
Gatekeeper——未公证的应用第一次仍会被拦，README 与发布说明里写了右键打开和
`xattr -d com.apple.quarantine` 两条路。

## smoke 跑的是解压出来的那份

每个平台都是先做出归档，再解压到一个干净目录，从**解压后的路径**启动二进制：

- Windows：`unzip` 后跑 `green.exe --capture …`；
- macOS：`ditto -x -k` 解出 `Green.app`，先 `codesign --verify`，再跑
  `Contents/MacOS/green`（`ELY_SMOKE=1` 退出 0 + `screencapture`）；
- Linux：`tar -xzf` 后 `ELY_SMOKE=1 xvfb-run ./green`，再 Xvfb + `import` 截图，断言
  `opened <文件> as tab 0` 且画面颜色数 > 16。

跑 staging 目录会漏掉“归档里少字体/少 dylib”这一类错误；差的就是这一层。

## `--capture` 只有 Windows

`PlatformWindow::render_to_image` 只有 Windows（DirectX）实现，macOS 与 Linux 落到 trait
默认实现，输出 `render_to_image not implemented for this platform`。因此：

- Windows 用 `--capture` 出图，退出码可信；
- macOS 上同样的失败**退出码是 0**（提前 `cx.quit()`），只能看 PNG 在不在、是否空白——
  **只查退出码会把“渲染坏了”判成通过**；
- Linux 上失败给退出码 3。

想在 macOS / Linux 上用 `--capture`，得先 vendor `gpui-pre-wgpu` 并在 surface renderer 上
补离屏读回；在那之前，这两格的证据是 `ELY_SMOKE=1` 退出 0 加真实窗口截图。

## Linux 的 glibc 下限与系统库

发行包在 `ubuntu-22.04` 上构建，产物最高符号版本 `GLIBC_2.35`（`objdump -T … | grep -o
'GLIBC_[0-9.]*' | sort -uV | tail -3`）。**不要换回 `ubuntu-latest`**：它已经是 26.04
（glibc 2.41），编出来的东西在老发行版上直接 `GLIBC_2.4x not found`。

运行时系统库（`packaging/linux/depends.tsv` 有完整映射）：`libxkbcommon(+x11)`、
`libwayland-client`、`libxcb(+xkb)`、`libdbus-1`、`libGL`/`libvulkan1`。构建机另需
`libdbus-1-dev`（`keyring` → `libdbus-sys`）、`libxkbcommon-dev`、`libwayland-dev`、
`libfontconfig1-dev`、`xvfb`、`imagemagick`（smoke 用）。

## Windows 的便携目录

zip 里带一个空的 `data/`（内含 `README.txt`）是有意的：`apps/green/src/portable.rs` 看到
`<exe>/data` 就把它当数据根，设置、会话、日志、插件都落在 `data\green\`，删掉 `data\`
就回到 `%LOCALAPPDATA%\Green\green`。macOS 与 Linux 不发 `data/`，数据走各自的系统目录。

## 发布闸门

- `version` job 在任何构建之前把 tag 顶到 `Cargo.toml` 的 `version`：不等就红，因为推上去的
  tag 改不了。`workflow_dispatch` 只演练，不建 release（`publish` 的 `if` 只认 `refs/tags/v*`）。
- 正式发布用 `git tag v1.0.0 && git push origin v1.0.0`，tag 只指向 main 上一个 CI 全绿、
  且 dispatch 四格全绿的 commit。
