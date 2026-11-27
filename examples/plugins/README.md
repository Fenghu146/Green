# Green 实用插件

三个外部进程插件，验证插件契约在真实用途下的表现。**不需要重新编译 Green**；
每个目录自带 `manifest.json`、脚本和 `README.md`。

| 目录 | 后缀 | 做什么 |
| --- | --- | --- |
| `texfrag/` | `.texfrag` | 每条 `$$...$$` 公式排成块级公式，条间有分隔线。已验证：公式走的是和内置视图同一套 Markdown/Typst 管线，19 个 Block 里 10 条全部排版。 |
| `csv-enhanced/` | `.csv`、`.tsv` | 表格 + 斜体摘要（行数、列数、数值列均值）；空文件提示、缺列补齐并注明、超过 500 行只显示前 200 行并注明已截断。 |
| `log-enhanced/` | `.log` | 有 error 才放顶部 alert，正文按 error / warn / 其余分组表格；超长行折成 code 块，超过 8 MB 按 path 只读前 2000 行。 |

## 安装

把目录整个拷到 Green 的数据目录：

```text
%LOCALAPPDATA%\Green\green\plugins\texfrag\
%LOCALAPPDATA%\Green\green\plugins\csv-enhanced\
%LOCALAPPDATA%\Green\green\plugins\log-enhanced\
```

数据目录在设置里搬过就跟着搬后的路径走。新目录要在 Green 启动时发现，所以
**第一次安装后重启一次**；之后改脚本或清单不用重启，下次打开文件时按文件指纹
重新加载。

仓库里本来就有 `csv/`、`tsv/`、`log/` 三个基础示例，声明了同样的后缀。两者
并存是明确支持的：宿主按 `manifest.json` 里 `id` 的字典序取第一个（与文件夹
顺序、清单文件顺序无关），增强版的 id 以 `0` 开头，排在 `csv` / `tsv` / `log`
之前。所以两套都装时 `.csv` / `.tsv` / `.log` 一定由增强版接手，不靠安装顺序
碰运气。只想留一套时，删掉或停用示例目录即可。

依赖只有 `python`（texfrag、csv-enhanced）和 `node`（log-enhanced），都从 `PATH`
找，不需要安装包。

## 卸载

删掉对应的插件目录，重启 Green。临时停用可以把 `manifest.json` 里的 `enabled`
改成 `false`，不用删文件。

## 文本与转义

插件输出的是 Block，宿主把它们拼成派生 Markdown 再打开：文本字段会按行内
Markdown 再解析一次。所以数据单元格要走各脚本里的 `literal()` 转义字面字符；
想让 `$$...$$` 变成公式就**不要**转义 `$` 和反斜杠。完整规则见
`../../docs/plugin-authoring.md`。

截图在 `.amp/in/artifacts/plugin-<name>.png`，用同一个已编译的 `green.exe --capture`
离屏生成；边界用例结果见各插件的 `README.md`。
