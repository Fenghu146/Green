# Green — Linux 版

运行：解压后执行 `./green`（或 `./green 笔记.md`）。

## 需要的系统库

这个二进制是动态链接的，桌面发行版一般都带着；缺了会报 `error while loading shared libraries`：

```text
libxkbcommon.so.0 libxkbcommon-x11.so.0 libwayland-client.so.0
libxcb.so.1 libxcb-xkb.so.1 libdbus-1.so.3 libgl.so.1 / libvulkan.so.1
```

Debian / Ubuntu：

```sh
sudo apt install libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 \
  libxcb1 libxcb-xkb1 libdbus-1-3 libgl1 libvulkan1
```

Fedora：

```sh
sudo dnf install libxkbcommon libxkbcommon-x11 wayland-libs-client \
  libxcb libxcb-keysyms dbus-libs mesa-libGL vulkan-loader
```

构建机是 Ubuntu 22.04（glibc 2.35），所以 **glibc 2.35 及以上** 的发行版可以直接跑：
Debian 12、Ubuntu 22.04 及更新、Fedora 36 及更新。更老的发行版会报
`GLIBC_2.35 not found`，那需要自己从源码构建。

## 装成桌面应用（可选）

```sh
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/256x256/apps
cp green ~/.local/bin/green
cp fonts -r ~/.local/share/green/fonts        # 与可执行文件同级时程序自己会找
cp green.desktop ~/.local/share/applications/
cp green.png ~/.local/share/icons/hicolor/256x256/apps/green.png
```

直接解压运行也可以；`green.desktop` 里的 `Exec=green %F` 需要 `green` 在 `PATH` 上。

## 数据在哪

`~/.local/share/Green/green/`（遵循 `XDG_DATA_HOME`）：设置、会话、日志、插件。
插件每个一个文件夹：`plugins/<id>/manifest.json`。

## 文件关联

`green.desktop` 声明了 `text/markdown`。装好后 `xdg-mime default green.desktop text/markdown`，
或右键任一 `.md` →「打开方式」→ Green。

## 自带字体

`fonts/` 随包：Noto Sans CJK SC Regular/Bold、Noto Sans Math、Noto Sans Mono（SIL OFL 1.1）。
编辑器与 PDF 导出用同一批文件，不依赖系统字体。

## 许可

Green 的代码是 GPL-3.0-or-later，全文见包内 `LICENSE`；第三方依赖与许可见
`THIRD-PARTY-NOTICES.txt`；字体许可见 `LICENSES/OFL-1.1.txt`。
