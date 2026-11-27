# Green — macOS 版

运行：双击 `Green.app`。`Green.app` 可以拖进「应用程序」，不需要安装器，不写注册表。

## 第一次打开被拦住了

这个包**没有签名**（Apple 开发者账号 99 美元一年，学生版 Green 不买）。macOS 第一次会拦：

- 右键（或按住 Control）点 `Green.app` → **打开** → 再点「打开」；
- 或者先清掉隔离属性，再双击：

  ```sh
  xattr -d com.apple.quarantine Green.app
  ```

这是 Gatekeeper 对「从网上下载的未签名应用」的常规提示，不是 Green 崩溃。系统若说「已损坏」，
同上：清掉 `com.apple.quarantine` 后再打开。

## 数据在哪

`~/Library/Application Support/Green/green/`：设置、会话、日志、插件都在那里。
插件每个一个文件夹：`plugins/<id>/manifest.json`，与编辑的文件无关。

## 文件关联

`Info.plist` 声明了 `.md` 与 `.markdown`（`net.daringfireball.markdown`）。第一次打开文档后，
右键任一 `.md` → 「显示简介」→「打开方式」→ Green →「全部更改…」即可设为默认。

## 自带字体

`Contents/Resources/fonts/` 随包：Noto Sans CJK SC Regular/Bold、Noto Sans Math、Noto Sans Mono
（SIL OFL 1.1）。编辑器与 PDF 导出用同一批文件，不依赖系统里装了什么字体。

## 许可

Green 的代码是 GPL-3.0-or-later，全文见包内 `LICENSE`（也在 `Contents/Resources/LICENSE`）；
第三方依赖与许可见 `THIRD-PARTY-NOTICES.txt`；字体许可见 `LICENSES/OFL-1.1.txt`。
