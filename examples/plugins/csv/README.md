# CSV plugin for Green

Opens `.csv` as a table. Python 3.8 or newer on `PATH`; no packages.

## Install

Copy this folder to Green's data folder under `plugins\csv\` (on Windows,
`%LOCALAPPDATA%\Green\green\plugins\csv\` unless the data folder was moved in
Settings):

```text
%LOCALAPPDATA%\Green\green\plugins\csv\manifest.json
%LOCALAPPDATA%\Green\green\plugins\csv\csv_plugin.py
```

Green reads the folder at startup. Install = add a folder, uninstall = delete
it, switch off = `"enabled": false` or rename the folder. Nothing is compiled
and Green is not changed.

## Manifest

```json
{
  "id": "csv", "name": "CSV", "version": "0.1.0", "apiVersion": 1,
  "extensions": [".csv"], "command": ["python", "csv_plugin.py"],
  "timeoutMs": 3000, "enabled": true, "capabilities": ["blocks@1"]
}
```

- `apiVersion` is an integer; this Green speaks `1` and refuses anything else.
- `extensions` are matched case-insensitively; `csv` and `.csv` are the same.
- `command` runs with the plugin's folder as its working directory.
- `timeoutMs` defaults to 3000; 50 to 600000 is accepted.
- `enabled` defaults to true; `capabilities` defaults to `["blocks@1"]`.
- Unknown fields are ignored on both sides, and a field added later must be
  optional with a default.

## One run

Green writes one JSON object to stdin and closes it:

```json
{"apiVersion": 1, "path": "C:\\notes\\data.csv", "ext": ".csv",
 "content": "bmFtZSxxdHkKYXBwbGUsMw==", "contentOmitted": false,
 "capabilities": ["blocks@1"]}
```

`content` is base64 of the file's bytes; above 8 MB it is `null` and
`contentOmitted` is `true`, so the plugin reads `path` itself.

The plugin writes one JSON object to stdout and exits 0:

```json
{"apiVersion": 1, "blocks": [{"kind": "table",
 "head": ["name", "qty"], "rows": [["apple", "3"]]}], "meta": {"rows": 1}}
```

A non-zero exit, a timeout (the process is killed), or an answer that is not
JSON leaves the file as plain text and says why in Green's status area. A block
of a kind Green does not know is dropped whole; unknown fields inside a known
block are ignored.

## Blocks

| `kind` | fields |
| --- | --- |
| `paragraph` | `text` (inline Markdown) |
| `heading` | `level` 1–6, `text` |
| `list` | `ordered` (bool), `start` (number), `items` (strings) |
| `code` | `language`, `text` |
| `quote` | `blocks` (blocks) |
| `table` | `head` (strings), `rows` (string arrays), `aligns` (`left`/`center`/`right`) |
| `image` | `url`, `alt`, `link` |
| `rule` | — |
| `alert` | `level` (`note`/`tip`/`important`/`warning`/`caution`), `text` |

## Adding a format

Copy this folder, rename `id`, point `extensions` at the new suffix and
`command` at the new script. Green's product code changes by zero files: the
extension goes into the registry at startup and every plugin's blocks render
the same way.
