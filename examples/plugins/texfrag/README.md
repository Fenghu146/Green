# LaTeX fragment plugin for Green

Opens a `.texfrag` file as typeset formulas. Each non-empty line is one formula;
a line can hold `$$...$$`, or a `$$` / `$$` pair can wrap several lines. Ten
formulas with separators are in `sample.texfrag`.

This plugin is a diagnostic for the contract: a plugin's blocks become a
derived Markdown document, so formula blocks are emitted as raw `$$\n...\n$$`
paragraphs, never escaped with `literal()`. Escaping backslashes would turn
`\frac` into text. Every other data string uses the same escaping rule as the
CSV/TSV/log examples.

## Install

Copy this folder into Green's data folder under `plugins\texfrag\` (on Windows,
`%LOCALAPPDATA%\Green\green\plugins\texfrag\` unless the data folder was moved
in Settings). Restart Green once when the folder is new; editing the script or
manifest afterwards needs no restart. Python 3.8 or newer on `PATH`, no
packages. Delete the folder to uninstall.

## Contract

`apiVersion: 1`, `timeoutMs: 8000`. Over 8 MB the host omits `content` and the
plugin reads `path` itself. Invalid `apiVersion` exits 2 with the reason on
stderr; an empty file produces one paragraph saying there are no formulas.

See `../../docs/plugin-authoring.md` for the full contract and the other
examples for the `literal()` escaping rule.
