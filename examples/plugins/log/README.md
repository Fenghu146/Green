# Log plugin for Green

Turns a `.log` into blocks: a warning alert when any line is an `ERROR` or
`FATAL`, a `time`/`level`/`message` table, and a code block for lines that are
neither. Node 18 or newer on `PATH`; no packages.

## Install

Copy this folder to Green's data folder under `plugins\log\` (on Windows,
`%LOCALAPPDATA%\Green\green\plugins\log\` unless the data folder was moved in
Settings), then restart Green.

## Why Node

The CSV plugin is Python. This one is Node on purpose: the contract is JSON on
stdin and stdout, so the language does not matter. The same `green.exe` ran
both with no rebuild.

The contract is the one in `../csv/README.md`; `../../docs/plugin-authoring.md`
covers it in full.
