# 打包与发行

给下次动 `.github/workflows/release.yml` 或 `packaging/` 的人。每条都是踩过之后写的。
安装包的失败恢复见 `release-recovery.md`。

## 发布物

| 平台 | 安装包 | 便携版 |
| --- | --- | --- |
| Windows x64 | `Green-<版本>-x64-setup.exe`（NSIS） | `Green-<版本>-windows-x64-portable.zip` |
| macOS arm64 | `Green-<版本>-arm64.dmg` | `Green-<版本>-macos-arm64.zip` |
| macOS x64 | `Green-<版本>-x64.dmg` | `Green-<版本>-macos-x64.zip` |
| Linux x64 | 无 | `Green-<版本>-linux-x64.tar.gz` |

每个平台另有一张 `smoke-<平台>.png`；发布页还有 `SHA256SUMS.txt`、`LICENSE`（GPL-3.0 全文）、
`THIRD-PARTY-NOTICES.txt`。包里也各带一份许可。

## Windows：NSIS，每用户，不写文件关联

`packaging/windows/green.nsi`：

- `RequestExecutionLevel user`——**不请求提权**。装到 `$LOCALAPPDATA\Programs\Green`。静默安装（`/S`）
  在 CI 里几秒内完成，本身就是“没有弹过 UAC”的证据：真弹了会一直等人点。
- 写的东西只有三样：安装目录、开始菜单快捷方式、`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Green`
  卸载项（带 `DisplayVersion`、`UninstallString`、`NoModify`/`NoRepair`）。
- **`HKCU\Software\Classes` 一个字节都不写，不做 `.md` 关联。** `OpenWithProgids` 的值、
  `Applications\green.exe` 整个键、`SHChangeNotify` 不调用导致的图标缓存不刷——这三处残留没人验证过，
  1.0.0 不做（`later.md` 记着）。CI 在装完之后断言 `HKCU\Software\Classes\Green.Markdown` 不存在。
- 卸载必须清干净：CI 跑 `Uninstall.exe /S`，然后断言安装目录与上面两个注册表键都不在了。
- 若哪天 `RequestExecutionLevel user` 实测出现提权弹窗，**直接换 Inno Setup**（`PrivilegesRequired=lowest`），
  别和 NSIS 搏斗：每用户安装、不弹 UAC 是对朋友的承诺，弹一次就破功。

## macOS：字体位置、签名顺序、dmg

1. **字体放 `Contents/Resources/fonts`。** 放进 `Contents/MacOS` 会让 `codesign` 把每个字体当成
   “嵌套代码”，报 `code object is not signed at all. In subcomponent: …fonts/…`，整个 bundle 签不上。
   `ely-ui::fonts::dir()` 的查找顺序因此是 `<exe>/fonts` → `Contents/Resources/fonts` → 源码树。
2. **`strip -x` 之后必须重新签名。** 编辑 Mach-O 会作废签名，而 arm64 macOS 不运行没有签名的二进制。
   顺序是：复制 → strip → `codesign --force --sign - <二进制>`（ad-hoc）→ 补齐 `Contents/` →
   `codesign --force --sign - Green.app`（bundle 的封印要覆盖最终的 `Resources/`）→ `codesign --verify`。
3. **ad-hoc 签名不能“清理”掉。** 完全没有签名的 `.app` 在 macOS 上报的是“**已损坏，无法打开**”，
   而不是“未识别开发者”——用户会以为下载坏了。有 ad-hoc 签名才落到可解释的那条提示上，
   右键打开或 `xattr -d com.apple.quarantine` 才有效。它不需要 Apple 账号，只解决“能不能启动”，
   不解决 Gatekeeper。
4. **dmg**：`hdiutil create -format UDZO`（必须压缩，否则是原始大小）、`-srcfolder` 保留
   `/Applications` 符号链接、未签名 dmg 挂载本身没有额外提示（提示出现在打开 `.app` 时）。
   `hdiutil: create failed - Resource busy` 来自上一次没 detach 的 `/Volumes/Green <n>`：
   开建之前只按名字 detach `/Volumes/Green` 与 `/Volumes/Green <数字>`，**不加 `-force`**。

## Linux：glibc 与系统库

在 `ubuntu-22.04` 上构建，产物最高符号版本 `GLIBC_2.35`（`objdump -T … | grep -o 'GLIBC_[0-9.]*' |
sort -uV | tail -3`）。**不要换回 `ubuntu-latest`**：它已经是 26.04（glibc 2.41），编出来的东西在老发行版上
直接 `GLIBC_2.4x not found`。

运行时需要 `libxkbcommon(+x11)`、`libwayland-client`、`libxcb(+xkb)`、`libdbus-1`、`libGL`/`libvulkan1`
（映射在 `packaging/linux/depends.tsv`）。构建机另需 `libdbus-1-dev`（`keyring` → `libdbus-sys`）、
`libxkbcommon-dev`、`libwayland-dev`、`libfontconfig1-dev`、`xvfb`、`imagemagick`。

## smoke 跑的是装完/解出来的那份

每个平台都先做出安装包或归档，再按用户实际拿到它的方式启动：

- Windows：`/S` 静默安装 → 从 `%LOCALAPPDATA%\Programs\Green` 启动 `--capture` → `Uninstall.exe /S` → 断言清干净；
- macOS：挂载 dmg → `codesign --verify` → 把 `Green.app` 拷到一个空目录（等同拖进 Applications）→ 卸载映像 → `ELY_SMOKE=1` 启动 → `screencapture`；
- Linux：解压 tar.gz → `ELY_SMOKE=1 xvfb-run ./green` → Xvfb + `import` 截图。

跑构建目录会漏掉“安装包里少字体/少库”这一类错误——字体在包里的相对位置正是这样坏的。
截图要**有人看一眼**：中文与公式都在，才算这一格通过。

## `--capture` 只有 Windows

`PlatformWindow::render_to_image` 只有 Windows（DirectX）实现，macOS 与 Linux 落到 trait 默认实现。
三个后果：

- Windows 用 `--capture` 出图，退出码可信；
- macOS 上同样的失败**退出码是 0**（提前 `cx.quit()`），只能看 PNG 在不在、是否空白；
- Linux 上失败给退出码 3。

想在 macOS / Linux 上用 `--capture`，得先 vendor `gpui-pre-wgpu` 并在 surface renderer 上补离屏读回。

## 发布闸门

- `version` job 在构建之前把三样东西对齐：tag、`Cargo.toml` 的版本、`CHANGELOG.md` 里有没有
  `## <版本>`。任一不符就红，因为推上去的 tag 改不了。
- `publish` 只在 `refs/tags/v*` 上跑，且第一步先断言**七个资产 + 四张 smoke 图 + LICENSE +
  THIRD-PARTY-NOTICES 都在**，缺一个就不建 release——少资产的 release 比没有 release 糟。
- 建 release 只有一个 job 做（`download-artifact` + `gh release create`），不让各平台自己上传：
  任何一格失败都不会留下公开但残缺的 release。
- 每个装工具、下依赖的步骤都有 `timeout-minutes`；一个卡住的步骤被 runner 杀掉后，靠 `needs`
  会在下游表现为“跳过”，而不是“失败”。
- job summary 里列每个资产的字节数，作为下一版的基线：任何一项跳到 MB 级都要先查清是加了资源，
  还是打包脚本把不该带的带进去了。
