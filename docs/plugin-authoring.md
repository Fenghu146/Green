# 给 Green 写插件

Green 的插件是**独立进程**：Green 把一次打开写成 JSON 交给它，它把 Block 列表写成 JSON 交回来。任何能读 stdin、写 stdout 的语言都行——随包的例子有 Python（CSV）和 Node（LOG），跑的是同一个契约。

加一个格式是 **0 个 Green 产品代码文件、不重编译、不发版**（`docs/design.md` §3）。插件不是沙箱，也不是 Markdown 扩展：它就是你账户下的一个普通程序，只装信得过的。

## 装在哪

Green 的数据目录下的 `plugins/<id>/`：

| 平台 | 目录 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\Green\green\plugins\` |
| macOS | `~/Library/Application Support/Green/green/plugins/` |
| Linux | `~/.local/share/Green/green/plugins/` |

数据目录在设置里搬过就跟着走。每个插件一个文件夹，里面至少 `manifest.json` 和它要跑的文件。Green **启动时**读一遍：加、删、改插件后重启 Green 生效，不需要重编译。

## 一次运行

### Green → 插件：stdin 一个 JSON，然后关闭

```json
{
  "apiVersion": 1,
  "path": "C:\\notes\\data.csv",
  "ext": ".csv",
  "content": "bmFtZSxxdHkKYXBwbGUsMw==",
  "contentOmitted": false,
  "capabilities": ["blocks@1"]
}
```

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `apiVersion` | 整数 | 契约版本，这一版是 `1` |
| `path` | 字符串 | 文件的**原生**路径（Windows 带反斜杠），不是 URL |
| `ext` | 字符串 | 小写、带点，如 `.csv`；没有后缀时是空串 |
| `content` | 字符串或 `null` | 文件原始字节的 base64 |
| `contentOmitted` | 布尔 | `true` 表示 `content` 是 `null`，插件自己按 `path` 读 |
| `capabilities` | 字符串数组 | 宿主支持的能力，目前只有 `blocks@1` |

文件超过 8 MiB（8 × 1024 × 1024 字节）时不发字节，只发路径。

### 插件 → Green：stdout 一个 JSON，退出码 0

```json
{
  "apiVersion": 1,
  "blocks": [
    { "kind": "table", "head": ["name", "qty"], "rows": [["apple", "3"]] }
  ],
  "meta": { "rows": 1 }
}
```

| 字段 | 必填 | 含义 |
| --- | --- | --- |
| `apiVersion` | 是 | 必须等于 `1` |
| `blocks` | 否 | Block 数组，省略等于空 |
| `meta` | 否 | 任意 JSON；当前版本不显示 |

stdout 只能是这一个 JSON。调试输出写 stderr：Green 只在失败时把 stderr 末尾放进原因里，日志里不带正文。

### 编码

stdin 和 stdout 都是 UTF-8。Windows 上 Python 默认按系统代码页写管道，中文路径和内容会坏，所以 Python 插件开头都要有：

```python
for stream in (sys.stdin, sys.stdout, sys.stderr):
    stream.reconfigure(encoding="utf-8")
```

Node 默认就是 UTF-8。

## Block 九种

| `kind` | 字段 | 说明 |
| --- | --- | --- |
| `paragraph` | `text` | 段落 |
| `heading` | `level`（1–6）、`text` | 标题 |
| `list` | `ordered`（布尔，默认 false）、`start`（数字，默认 1）、`items`（字符串数组） | 列表；每一项是一行文本 |
| `code` | `language`（可选）、`text` | 代码块；围栏会自动加长以包住正文里的反引号 |
| `quote` | `blocks`（Block 数组） | 引用 |
| `table` | `head`（字符串数组，至少一列）、`rows`（字符串二维数组）、`aligns`（`left`/`center`/`right`/`none`，可选） | 表格；行不足补空，超出的列丢弃 |
| `image` | `url`、`alt`（可选）、`link`（可选） | 图片，可带外层链接 |
| `rule` | — | 分隔线 |
| `alert` | `level`（`note`/`tip`/`important`/`warning`/`caution`）、`text` | GitHub 提示块 |

```json
{ "kind": "heading", "level": 2, "text": "本月的表" }
{ "kind": "list", "ordered": true, "start": 1, "items": ["第一项", "第二项"] }
{ "kind": "code", "language": "json", "text": "{\"a\": 1}" }
{ "kind": "quote", "blocks": [{ "kind": "paragraph", "text": "引一句" }] }
{ "kind": "image", "url": "chart.png", "alt": "图", "link": "https://example.com" }
{ "kind": "rule" }
{ "kind": "alert", "level": "warning", "text": "有 1 行是错误" }
```

文本字段（`text`、标题、列表项、`alert` 的文字、表格单元格）按**行内 Markdown** 渲染，所以插件可以用 `**粗**`、`` `代码` ``、`[链接](…)`。数据型插件要让 `*`、`_`、`[` 这些字符字面显示，得自己转义；随包的 CSV/LOG 示例为了短，直接透传。表格单元格里的 `|` 由 Green 转义，换行变成 `<br>`。

不认识的 `kind` **整块丢弃**；已知 `kind` 里缺字段或类型不对的那一块也丢弃（日志里记一条），不会让整次打开失败。未知字段一律忽略。

## 清单 `manifest.json`

```json
{
  "id": "csv",
  "name": "CSV",
  "version": "0.1.0",
  "apiVersion": 1,
  "extensions": [".csv"],
  "command": ["python", "csv_plugin.py"],
  "timeoutMs": 3000,
  "enabled": true,
  "capabilities": ["blocks@1"]
}
```

| 字段 | 必填 | 默认 | 说明 |
| --- | --- | --- | --- |
| `id` | 是 | — | 名字，不能含 `/` 或 `\`；不是文件夹名 |
| `name` | 否 | — | 给人看的名字 |
| `version` | 是 | — | 插件自己的版本；缓存键的一部分 |
| `apiVersion` | 是 | — | 整数，这一版只认 `1` |
| `extensions` | 是 | — | 至少一个；`csv` 和 `.csv` 等价，统一成小写带点 |
| `command` | 是 | — | 非空数组；第一项是可执行文件（走 `PATH`），其余是参数；工作目录是插件自己的文件夹 |
| `timeoutMs` | 否 | `3000` | 一次运行的上限，允许 `50`–`600000` |
| `enabled` | 否 | `true` | `false` 时仍加载，但永不选中 |
| `capabilities` | 否 | `["blocks@1"]` | 声明会产出的能力 |

规则：

- 清单是 UTF-8；带 BOM 也能读（记事本默认会写 BOM）。
- 未知字段忽略；以后新增的字段一定是可选的、有默认值。
- 同一个 `id` 出现在两个文件夹：按文件夹名排序，后面的跳过并记日志。
- 同一个后缀被两个插件声明：按 `id` 排序，第一个生效，后一个记日志。
- 没有 `manifest.json`、JSON 语法错、`apiVersion` 超范围、`extensions` 为空、`command` 为空、`timeoutMs` 越界：这个文件夹被跳过并记日志，**不影响启动**。

## 兼容性

- `apiVersion` 是整数，这一版只认 `1`。写 `2`、写 `"1"`、写 `0` 都算坏清单，插件被跳过，Green 照常启动。
- `capabilities` 协商：宿主只调用双方都声明过的能力。省略时按 `["blocks@1"]` 算（老清单没有这个字段）。
- 两侧都忽略未知字段，所以任一侧可以先加可选字段。
- 破坏性变更 `apiVersion += 1`；旧版留两个小版本（发布策略，不是代码里的开关）。

## 失败会怎样

一律**降级成纯文本视图**，状态区给一条提示，主程序不受影响：

| 情况 | Green 做什么 |
| --- | --- |
| 超时（`timeoutMs`） | 杀掉进程，文件按文本打开 |
| 非零退出 | 文件按文本打开；原因带退出码和 stderr 末尾 |
| stdout 不是合法 JSON，或 `apiVersion` 不对 | 同上 |
| 可执行文件找不到 | 同上 |
| 清单坏 | 启动时记一条 skip 日志，插件不参与；不影响启动 |
| 没有插件认领这个后缀 | 按普通文本打开，不提示（没有出错） |

## 缓存

同一份文件、同一个插件版本，第二次打开不再跑进程。键是（规范化路径、mtime、大小、插件 `id`、插件 `version`）。内存里最多留 32 个视图，Green 退出即清空；失败不缓存。

改了脚本但没改 `version`，同一个文件在本次运行里还会拿旧结果——改版本号或重启 Green。

## 两个语言，同一个契约

| 插件 | 语言 | 后缀 | 输出 |
| --- | --- | --- | --- |
| `examples/plugins/csv` | Python | `.csv` | 一张表 |
| `examples/plugins/log` | Node | `.log` | 有 ERROR 时一条 alert，一张 time/level/message 表，无法解析的行放一个 code 块 |

实测（同一份已编译的 `green.exe`）：

```text
mark: app.log is a plugin's
mark: app.log opens through plugin log: 3 block(s)
```

## 加一个新格式

1. 在 `<数据目录>/plugins/` 下建 `<id>/`。
2. 写 `manifest.json`：`extensions` 指向新后缀，`command` 指向你的程序。
3. 写程序：读 stdin 的 JSON，把 Block 写到 stdout。
4. 重启 Green，打开那个后缀的文件。

不需要改 Green、不需要重编译、不需要发版。实测：`examples/plugins/tsv/` 只加 3 个文件（清单、脚本、README），同一个已编译的 exe 直接打开 `data.tsv`；那次提交里没有任何 `crates/` 或 `apps/` 文件。

## 调试

手动喂一次请求，不用开 Green：

```powershell
$content = [Convert]::ToBase64String([IO.File]::ReadAllBytes("sample.log"))
@{ apiVersion = 1; path = "sample.log"; ext = ".log"; content = $content;
   contentOmitted = $false; capabilities = @("blocks@1") } |
  ConvertTo-Json -Compress | node log_plugin.js
```

- 日志：`ELY_LOG=info`，Green 写到 `<数据目录>/ely.log`，开发构建同时写 stderr。插件的加载和失败在 `green::plugins`，进程和超时在 `green_plugins::supervisor`。
- `ELY_DATA_ROOT=<临时目录>` 把整个数据根目录指到别处，试插件不碰真实数据。
- 表格目前在视觉视图只画第一列（组件库的既有问题），核对内容用 Source 模式（`Ctrl+/`）。

## 限制

- 没有沙箱、没有权限声明；插件以你的身份运行。
- 一次打开一个进程；没有常驻进程，也没有增量协议。
- 插件视图是派生文档（名为原文件名），不是原文件本身：保存时要另选路径（默认 `.md`），不会写回原文件；同一个文件开两次会有两个 tab。
- 只能产出 Block；不能改 Green 的界面，也读不到正在编辑的缓冲区。
