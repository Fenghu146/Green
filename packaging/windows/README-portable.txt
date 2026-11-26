# Green — Windows 便携版

运行：双击 `green.exe`。

- 数据在本目录的 `data\` 下（Green 自己的文件夹是 `data\green\`）：设置、会话、日志都在那里。
- 插件放 `data\green\plugins\<id>\`，每个插件一个文件夹，里面是 `manifest.json` 和它要跑的文件。
- 删掉整个 `data\` 文件夹，Green 下次启动就回到 `%LOCALAPPDATA%\Green\green\`。
- 不写注册表，不需要安装；整个目录可以放在 U 盘或任何可写的位置。

许可：Green 的代码是 GPL-3.0-or-later，全文见 `LICENSE`；第三方依赖与许可见
`THIRD-PARTY-NOTICES.txt`；随包字体是 SIL OFL 1.1，见 `LICENSES/OFL-1.1.txt`。

这个包**不含**安装器、代码签名和自动更新。未签名，Windows 第一次运行可能提示，选择
“更多信息 → 仍要运行”。
