# tag 已经推上去，但 Release 没出来

这条流程只在出事那天有用，所以先写下来。**tag 一旦推到远端就是既成事实：不移动、不删除、不强推。**

## 症状

- tag `v1.0.0` 在远端存在，但 Releases 页面没有这一版；
- 或者 Release 出来了，却少了某个平台的资产；
- 或者 `publish` job 被跳过（上游某格红），于是“构建失败”看起来像“还没有发布”。

## 先判断是哪一种

```sh
gh run list --workflow=release.yml --limit 5     # 找到 tag 那次运行
gh run view <run-id> --json jobs -q '.jobs[] | "\(.name): \(.status) \(.conclusion // "")"'
```

- `version` 红 → 版本、tag、CHANGELOG 三者不一致。修主分支上的版本或补 CHANGELOG，**发新的版本号**，
  旧的 tag 留着不动。
- 某一格 `build` 红 → 该平台的产物没出来，`publish` 因 `needs` 被跳过，所以没有 Release。修代码，
  推一条新提交，然后在**同一个 tag** 上重新跑（见下）。
- `publish` 红在“资产不齐”断言 → 说明有平台的 `upload-artifact` 没成功；同样先修再重跑。
- `publish` 红在 `gh release create` → 看它的输出：权限（`contents: write`）、同名 release 已存在、
  网络。

## 重跑：dispatch 指向那个 tag

`workflow_dispatch` 可以指定任意 ref，包括 tag。指向 tag 时 `GITHUB_REF_NAME` 就是 `v1.0.0`，
`publish` 的 `if: startsWith(github.ref, 'refs/tags/v')` 照样成立：

```sh
gh workflow run release.yml --ref v1.0.0
gh run watch
```

它会**重新构建四格、重新打包、重新做 smoke**，然后建 release。多花十几分钟，换来的是
“发布出去的东西确实是由这个 tag 的代码构建的”，而不是某个工作区里恰好能跑的二进制作数。

前提：`release.yml` 必须在默认分支上（GitHub 只允许 dispatch 默认分支已有的工作流）。

## 只补资产、不想重建

可以，但要守两条：

1. 用的是**那一版构建出来的**同一批文件（同一个 run 的 artifacts，`gh run download <run-id>`），
   不要用本地现编的二进制顶替；
2. 补完重新生成 `SHA256SUMS.txt`，并核对发布说明里的哈希表——说明是工作流生成的，手工补资产后
   它会和资产对不上。

## 不要在什么情况下做

- **不要 `git tag -f`、不要删远端 tag 重推。** 已经有人下过的东西会静默变掉。
- **不要手工 `gh release create` 一版“差不多”的。** 少一个平台的 release 比没有 release 更糟：
  下载的人不会检查自己拿到的是不是全的。
- **不要把 tag 指到一个还没跑绿 main 的 commit 上。** 版本闸门只能挡住版本号不一致，
  挡不住“代码本身红了”。

## 预防这一页用不上

- 版本、tag、`CHANGELOG.md` 三者在 `version` job 里对齐，构建之前就红；
- `publish` 第一步断言资产齐全；
- 每个装工具的步骤都有 `timeout-minutes`，卡住的步骤会红，而不是被 runner 杀掉后让下游“跳过”；
- 发布前在 main 上 `workflow_dispatch` 演练一次四格全绿，再打 tag。
