#!/usr/bin/env python3
"""Green's CSV/TSV plugin: table, summary, ragged rows and truncation.

The host turns the blocks into Markdown, so data cells go through literal()
while the summary deliberately uses inline Markdown for its italics.
"""

import base64
import binascii
import csv
import io
import json
import math
import sys

LIMIT = 500
SHOWN = 200


def literal(text: str) -> str:
    """A cell's text as Markdown shows it, not as Markdown means it."""
    text = text.replace("\\", "\\\\")
    for mark in "`*_[]<>|":
        text = text.replace(mark, "\\" + mark)
    return text.replace("\r\n", "<br>").replace("\n", "<br>").replace("\r", "<br>")


def read_text(request) -> str:
    content = request.get("content")
    if content is None:
        # Over 8 MB the host sends the path and no bytes.
        with open(request["path"], "rb") as file:
            content = base64.b64encode(file.read()).decode("ascii")
    return base64.b64decode(content).decode("utf-8-sig")


def summary(head, data, stats, short, truncated) -> str:
    parts = [f"行数：{data}", f"列数：{len(head)}"]
    means = [
        f"{literal(head[at])} {total / count:.4g}"
        for at, (total, count, numeric) in enumerate(stats)
        if numeric and count
    ]
    parts.append("数值列均值：" + ("、".join(means) if means else "无"))
    for line, missing in short[:5]:
        parts.append(f"第 {line} 行少 {missing} 列")
    if len(short) > 5:
        parts.append(f"另有 {len(short) - 5} 行不齐")
    if truncated:
        parts.append(f"已截断（只显示前 {SHOWN} 行）")
    return "*摘要：" + " · ".join(parts) + "*"


def main() -> int:
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8")

    request = json.load(sys.stdin)
    if request.get("apiVersion") != 1:
        print(f"this plugin speaks apiVersion 1, not {request.get('apiVersion')}",
              file=sys.stderr)
        return 2
    try:
        text = read_text(request)
    except (OSError, ValueError, binascii.Error, UnicodeError) as error:
        print(f"cannot read the file: {error}", file=sys.stderr)
        return 2

    if not text.strip():
        print(json.dumps({
            "apiVersion": 1,
            "blocks": [{"kind": "paragraph", "text": "文件是空的，没有表格可显示。"}],
            "meta": {"rows": 0},
        }, ensure_ascii=False))
        return 0

    delimiter = "\t" if request.get("ext", "").lower() == ".tsv" else ","
    reader = csv.reader(io.StringIO(text), delimiter=delimiter)
    head = next(reader)
    width = len(head)
    stats = [[0.0, 0, True] for _ in head]
    shown = [head]
    short = []
    data = 0
    for row in reader:
        data += 1
        if len(row) < width:
            short.append((data, width - len(row)))
            row = row + [""] * (width - len(row))
        row = row[:width]
        if data <= SHOWN:
            shown.append([literal(cell) for cell in row])
        for at, cell in enumerate(row):
            value = cell.strip()
            if not value:
                continue
            try:
                number = float(value)
            except ValueError:
                stats[at][2] = False
                continue
            if not math.isfinite(number):
                stats[at][2] = False
                continue
            if stats[at][2]:
                stats[at][0] += number
                stats[at][1] += 1

    truncated = data > LIMIT
    blocks = [
        {"kind": "table", "head": [literal(cell) for cell in head], "rows": shown[1:]},
        {"kind": "paragraph", "text": summary(head, data, stats, short, truncated)},
    ]
    print(json.dumps({
        "apiVersion": 1,
        "blocks": blocks,
        "meta": {"rows": data, "columns": width, "shown": len(shown) - 1},
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
