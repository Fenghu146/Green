# Log plugin for Green (enhanced)

Opens `.log` with a warning alert when errors exist (`共 N 行，error M 条，
warn K 条`), then one table per level group — error (`ERROR`/`FATAL`), warn
(`WARN`/`WARNING`), and the rest — separated by rules. `FATAL` counts as an
error; lines without a level go into the last group with `—` in the time and
level columns. Node 18 or newer on `PATH`, no packages. `apiVersion: 1`,
`timeoutMs: 20000`.

## Edge cases

- No errors: no alert at all. Empty file: one paragraph saying the log is empty.
- A single line longer than 500 characters is folded into a `code` block every
  120 characters instead of stretching a table cell; only its first 5,000
  characters are shown, with a note when the rest is omitted.
- Files over 8 MB arrive with `content: null` / `contentOmitted: true`. The
  plugin opens `path` and streams only the first 2,000 lines (64 KB reads), so
  a 9.35 MB log finishes in 126 ms inside the timeout. A paragraph says how
  many lines were read.
- `apiVersion != 1` exits 2 with the reason on stderr; Green falls back to the
  plain-text view. Table cells use the stock log example's `literal()` so `*`,
  `|` and `\` stay literal.

## Install

Copy this folder into Green's data folder under `plugins\log-enhanced\` (on
Windows, `%LOCALAPPDATA%\Green\green\plugins\log-enhanced\` unless the data
folder was moved in Settings). Restart Green when the folder is new. Delete the
folder to uninstall. The manifest declares `timeoutMs: 20000`; interpreter
cold starts on windows-latest reach 7.47 s, so the 8 s default is not used.
The stock `log` example claims `.log` too; both can stay
installed. The host picks the first `id` in dictionary order, and this plugin's
id starts with `0`, so it sorts before `log` and wins deterministically.

`sample.log` has two errors, two warnings, ordinary lines and one long
compressed-style line.
