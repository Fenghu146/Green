#!/usr/bin/env python3
"""Green's CSV plugin: one request on stdin, one table block on stdout.

Contract (design §3): the host writes
{"apiVersion", "path", "ext", "content" (base64 or null), "contentOmitted",
"capabilities"} and reads {"apiVersion", "blocks", "meta"}.
"""

import base64
import csv
import io
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

    rows = list(csv.reader(io.StringIO(text)))
    if not rows:
        blocks = []
    else:
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
