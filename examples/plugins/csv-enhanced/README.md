# CSV / TSV plugin for Green (enhanced)

Opens `.csv` and `.tsv` as a table plus an italic summary line: row count,
column count, and the mean of every column whose non-empty values are all
numbers. Non-numeric columns are skipped. Python 3.8 or newer on `PATH`, no
packages. `apiVersion: 1`, `timeoutMs: 8000`.

## Edge cases

- Empty file: one paragraph saying there is nothing to show.
- Ragged rows: short rows are padded with empty cells; the summary names the
  data row and the number of missing cells (`第 N 行少 M 列`, N counts data rows
  after the header).
- More than 500 data rows: only the first 200 are put in the table; the summary
  says `已截断`. All rows are still streamed for the counts and means. Measured
  on 700 rows: 133 ms, and on 120,000 rows read through `contentOmitted`:
  280 ms, both inside `timeoutMs`.
- `content` is base64 when the file is under 8 MB; above that it is `null` and
  the plugin reads `path` itself. A bad `apiVersion` or unreadable content
  exits 2 with the reason on stderr, and Green falls back to plain text.

Cells use the same `literal()` escaping as the stock examples, so `*`, `_`,
`|`, `\` and line breaks show as data. The summary intentionally uses inline
Markdown for its italics.

## Install

Copy this folder into Green's data folder under
`plugins\csv-enhanced\` (on Windows,
`%LOCALAPPDATA%\Green\green\plugins\csv-enhanced\` unless the data folder was
moved in Settings). Restart Green when the folder is new. Delete the folder to
uninstall. The stock `csv` / `tsv` examples claim the same suffixes; both sets
can stay installed. The host picks the first `id` in dictionary order, and this
plugin's id starts with `0`, so it sorts before `csv` / `tsv` and wins
deterministically.

`sample.csv` shows a numeric column, a text column, escaped Markdown
characters and a short row; `sample.tsv` is the tab-separated case.
