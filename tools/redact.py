#!/usr/bin/env python3
"""Redacts a real note into a committed skeleton: structure in, prose out.

The repository is public. A note a person wrote for themselves is not ours to
publish, but its *shape* is what the layout has to survive — headings, tables,
code, formulas, numbering, symbols. So every run of prose that is long enough to
be a sentence becomes `（正文占位，共 N 字）`, while structure stays.

Usage:
    python tools/redact.py SRC_DIR OUT_DIR        write the skeletons
    python tools/redact.py --verify OUT_DIR       fail on prose the redactor left
"""

import os
import re
import sys

# A run this short and free of Chinese is numbering, a bullet or punctuation.
KEEP_UNDER = 4
# A table cell longer than this in Chinese is prose, not a label or a number.
CELL_PROSE = 6

# Protected spans, in order: display math, inline math, code spans, entities.
SPANS = re.compile(r"(\$\$.*?\$\$|\$[^$\n]+\$|`[^`\n]+`|&#?[A-Za-z0-9]+;)")
# Superscripts and subscripts carry meaning a layout has to place; not prose.
SCRIPT = re.compile(r"[\u00b2\u00b3\u00b9\u2070-\u209f\u1d62-\u1d6a]")


def segments(text):
    """`text` as (piece, protected) pairs; a protected piece is kept verbatim."""
    at = 0
    for found in SPANS.finditer(text):
        if found.start() > at:
            yield text[at : found.start()], False
        yield found.group(), True
        at = found.end()
    if at < len(text):
        yield text[at:], False


def redact_cell(cell):
    """A table keeps its shape, its formulas and its escaped pipes; prose out."""
    pieces = []
    for run, protected in segments(cell):
        if not run:
            continue
        chinese = len(re.findall(r"[\u4e00-\u9fff]", run))
        pieces.append(run if protected or chinese < CELL_PROSE else f"（{chinese} 字）")
    return "".join(pieces)


def redact_row(line):
    # A pasted table can start with an escaped pipe; it is still a row.
    if not line.lstrip().startswith(("|", "\\|")):
        return line
    indent = line[: len(line) - len(line.lstrip())]
    # An escaped pipe is a character in a cell, not a column boundary.
    cells = re.split(r"(?<!\\)\|", line.strip())
    if all(set(cell.strip()) <= set("-: ") for cell in cells if cell.strip()):
        return line
    return indent + "|".join(redact_cell(cell) for cell in cells)


def redact_line(line):
    if line.strip().startswith("```") or line.lstrip().startswith("```"):
        return line
    # Structure that must survive verbatim.
    if line.startswith("#") or line.strip() == "":
        return line
    if line.lstrip().startswith(("|", "\\|")):
        return redact_row(line)
    pieces = []
    for run, protected in segments(line):
        if not run:
            continue
        if protected:
            pieces.append(run)
            continue
        # Keep the line's own markers: bullets, numbers, quotes, rules.
        marker = ""
        while run and run[0] in " \t>-*+|0123456789.)[ ]" and len(marker) < 8:
            marker += run[0]
            run = run[1:]
        if not marker and run[:1] in " \t":
            marker, run = run[:1], run[1:]
        kept = SCRIPT.findall(run)
        bare = SCRIPT.sub("", run)
        prose = len(bare.strip())
        chinese = len(re.findall(r"[\u4e00-\u9fff]", bare))
        if prose == 0:
            pieces.append(marker + "".join(kept))
        elif chinese < 2 and prose < KEEP_UNDER:
            pieces.append(marker + run)
        else:
            pieces.append(f"{marker}（正文占位，共 {prose} 字）")
    return "".join(pieces)


def skeleton(text):
    out = []
    for line in text.replace("\r\n", "\n").split("\n"):
        out.append(redact_line(line))
    return "\n".join(out)


def leftover(text):
    """Prose the redactor left behind: a run of Chinese it should have caught."""
    hits = []
    for number, line in enumerate(text.split("\n"), 1):
        if line.strip().startswith(("#", "```")):
            continue
        if line.lstrip().startswith(("|", "\\|")):
            for cell in re.split(r"(?<!\\)\|", line.strip()):
                for piece, protected in segments(cell):
                    if protected:
                        continue
                    run = SCRIPT.sub("", piece)
                    run = re.sub(r"（\d+ 字）", "", run)
                    if len(re.findall(r"[\u4e00-\u9fff]", run)) >= CELL_PROSE:
                        hits.append(f"{number}: cell {run.strip()[:40]}")
            continue
        for piece, protected in segments(line):
            if protected:
                continue
            run = SCRIPT.sub("", piece)
            run = re.sub(r"（正文占位，共 \d+ 字）", "", run)
            if len(run.strip()) >= 12 and re.search(r"[\u4e00-\u9fff]{6}", run):
                hits.append(f"{number}: {run.strip()[:60]}")
    return hits


def main():
    args = sys.argv[1:]
    if args[:1] == ["--verify"]:
        folder = args[1]
        bad = 0
        for name in sorted(os.listdir(folder)):
            if not name.endswith(".md"):
                continue
            with open(os.path.join(folder, name), encoding="utf-8") as text:
                hits = leftover(text.read())
            if hits:
                bad += len(hits)
                print(f"{name}: {len(hits)} lines still carry prose")
                for hit in hits[:5]:
                    print(f"  {hit}")
        if bad:
            sys.exit(f"redact: {bad} runs of prose survive")
        print(f"redact: {folder} holds structure only")
        return
    if len(args) != 2:
        sys.exit(__doc__)
    src, out = args
    os.makedirs(out, exist_ok=True)
    for name in sorted(os.listdir(src)):
        if not name.endswith(".md"):
            continue
        with open(os.path.join(src, name), encoding="utf-8") as text:
            body = skeleton(text.read())
        with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as text:
            text.write(body)
        print(f"{name}: {len(body)} characters")


if __name__ == "__main__":
    main()
