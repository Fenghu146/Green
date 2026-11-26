#!/usr/bin/env python3
"""Writes the version and pin lines of docs/compatibility.md."""

import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
PAGE = "docs/compatibility.md"
MARK = "<!-- 下两行由 scripts/compatibility.py 写出，勿手改 -->"


def read(name):
    with open(os.path.join(ROOT, name), encoding="utf-8") as text:
        return text.read()


def found(pattern, name):
    """The one capture `pattern` makes in file `name`."""
    hits = re.findall(pattern, read(name), re.M)
    if len(hits) != 1:
        sys.exit(f"compatibility.py: {name} has {len(hits)} matches for {pattern}")
    return hits[0]


def patches(folder):
    """A patch folder's numbers and count; the number is the last four-digit run."""
    names = sorted(name for name in os.listdir(os.path.join(ROOT, "patches", folder)) if name.endswith(".patch"))
    if not names:
        sys.exit(f"compatibility.py: patches/{folder} holds no patch")
    numbers = [re.findall(r"\d{4}", name)[-1] for name in names]
    span = numbers[0] if len(set(numbers)) == 1 else f"{numbers[0]}–{numbers[-1]}"
    return f"{span}，{len(names)} 个"


def pruned():
    """How many paths the component library's vendoring leaves out."""
    return len([line for line in read("patches/ely-gpui-components/fc71e96-prune.txt").splitlines() if line.strip()])


def lines():
    version = found(r'^version = "([^"]+)"', "Cargo.toml")
    gpui = found(r'^gpui = \{ package = "gpui-pre", version = "=([^"]+)"', "Cargo.toml")
    rev = found(r'rev = "([0-9a-f]{40})"', "Cargo.toml")
    return [
        f"版本：Green {version}。",
        f"依赖：gpui-pre {gpui}（patches/gpui {patches('gpui')}）；"
        f"组件库 {rev[:7]}（patches/ely-gpui-components {patches('ely-gpui-components')}，"
        f"裁掉 {pruned()} 个只属于它自己仓库的路径）；"
        f"crates.io 的另外五个 crate（patches/crates-io {patches('crates-io')}）。",
    ]


def main():
    # This prints Chinese to whatever console CI gives it; cp1252 would raise.
    sys.stdout.reconfigure(encoding="utf-8")
    page = read(PAGE).split("\n")
    if page.count(MARK) != 1:
        sys.exit(f"compatibility.py: {PAGE} needs the marker line once")
    at = page.index(MARK) + 1
    want = lines()
    if sys.argv[1:] == ["--check"]:
        if page[at : at + 2] != want:
            sys.exit(f"compatibility.py: {PAGE} is not this build's; it should read:\n" + "\n".join(want))
        print(f"compatibility: {PAGE} names this build: {want[0]}")
        return
    page[at : at + 2] = want
    with open(os.path.join(ROOT, PAGE), "w", encoding="utf-8", newline="\n") as out:
        out.write("\n".join(page))
    print(f"compatibility: wrote {want[0]}")


if __name__ == "__main__":
    main()
