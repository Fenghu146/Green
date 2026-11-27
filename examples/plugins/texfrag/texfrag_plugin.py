#!/usr/bin/env python3
"""Green's LaTeX fragment plugin: one formula per line or $$ ... $$ paragraph.

TeX is emitted raw into paragraph blocks: Green reads the derived Markdown
with the same math parser it uses for a note, so a formula can typeset. Data
text would need literal(); math deliberately must not be escaped.
"""

import base64
import json
import sys


def literal(text: str) -> str:
    """A data string as Markdown shows it, not as Markdown means it."""
    text = text.replace("\\", "\\\\")
    for mark in "`*_[]<>|":
        text = text.replace(mark, "\\" + mark)
    return text.replace("\r\n", "<br>").replace("\n", "<br>").replace("\r", "<br>")


def formulas(text: str) -> list[str]:
    found = []
    open_block = None
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped:
            continue
        if open_block is not None:
            if stripped == "$$" or stripped.endswith("$$"):
                tail = stripped[:-2].rstrip() if stripped != "$$" else ""
                if tail:
                    open_block.append(tail)
                found.append("\n".join(open_block))
                open_block = None
            else:
                open_block.append(line)
            continue
        if stripped == "$$":
            open_block = []
        elif stripped.startswith("$$") and stripped.endswith("$$") and len(stripped) > 4:
            found.append(stripped[2:-2].strip())
        else:
            found.append(line.strip())
    if open_block is not None:
        found.append("\n".join(open_block))
    return found


def main() -> int:
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8")

    request = json.load(sys.stdin)
    if request.get("apiVersion") != 1:
        print(f"this plugin speaks apiVersion 1, not {request.get('apiVersion')}",
              file=sys.stderr)
        return 2

    content = request.get("content")
    if content is None:
        with open(request["path"], "rb") as file:
            content = base64.b64encode(file.read()).decode("ascii")
    text = base64.b64decode(content).decode("utf-8-sig")

    blocks = []
    entries = formulas(text)
    if not entries:
        blocks.append({"kind": "paragraph", "text": "这个文件里还没有公式。"})
    for index, tex in enumerate(entries):
        if index:
            blocks.append({"kind": "rule"})
        if "$$" in tex:
            # An unclosed fence is data, not a formula: keep its source visible.
            blocks.append({"kind": "code", "language": "text", "text": tex})
        else:
            blocks.append({"kind": "paragraph", "text": "$$\n" + tex + "\n$$"})

    print(json.dumps({
        "apiVersion": 1,
        "blocks": blocks,
        "meta": {"formulas": len(entries)},
    }, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
