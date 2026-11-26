#!/usr/bin/env python3
"""Green's TSV plugin: the CSV contract, split on tabs.

This folder is the proof that a new format costs Green no code: drop it in
Green's plugins folder and .tsv opens, with no rebuild and no release.
"""

import base64
import json
import sys


def literal(text: str) -> str:
    """A cell's text as Markdown shows it, not as Markdown means it."""
    text = text.replace("\\", "\\\\")
    for mark in "`*_[]<>|":
        text = text.replace(mark, "\\" + mark)
    return text.replace("\r\n", "<br>").replace("\n", "<br>").replace("\r", "<br>")


def main() -> int:
    # Windows pipes take the locale's encoding unless told otherwise.
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8")

    request = json.load(sys.stdin)
    if request.get("apiVersion") != 1:
        print(f"this plugin speaks apiVersion 1, not {request.get('apiVersion')}",
              file=sys.stderr)
        return 2

    content = request.get("content")
    if content is None:
        # Over 8 MB the host sends the path and no bytes.
        with open(request["path"], "rb") as file:
            content = base64.b64encode(file.read()).decode("ascii")
    text = base64.b64decode(content).decode("utf-8-sig")

    rows = [line.split("\t") for line in text.splitlines() if line.strip()]
    blocks = []
    if rows:
        width = max(len(row) for row in rows)
        rows = [row + [""] * (width - len(row)) for row in rows]
        rows = [[literal(cell) for cell in row] for row in rows]
        blocks = [{"kind": "table", "head": rows[0], "rows": rows[1:]}]

    print(json.dumps({
        "apiVersion": 1,
        "blocks": blocks,
        "meta": {"rows": max(len(rows) - 1, 0), "columns": len(rows[0]) if rows else 0},
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
