#!/usr/bin/env python3
"""Regenerate Green's icon files. Not a build step.

Run this only when the artwork changes: `assets/icons/green.ico` and
`green.png` are committed, so building and running Green needs neither this
script nor Chrome. It rasters through a local Chrome or Edge (no other SVG
renderer is assumed), so it fails on a machine without one — that is fine,
because CI never runs it.

The master draws at 48 px and up; 32 px and below get the small glyph (no main
vein, a heavier stroke, the leaf larger), so nothing is a shrunken master.
The .ico carries PNG images at each size.
"""

import os
import shutil
import struct
import subprocess
import sys
import tempfile

SIZES = [16, 24, 32, 48, 64, 128, 256]
SMALL_UP_TO = 32
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MASTER = os.path.join(ROOT, "assets", "icon.svg")
SMALL = os.path.join(ROOT, "assets", "icon-small.svg")
ICO = os.path.join(ROOT, "assets", "icons", "green.ico")
PNG = os.path.join(ROOT, "assets", "icons", "green.png")


def chrome():
    for name in ("chrome", "msedge"):
        found = shutil.which(name)
        if found:
            return found
    for place in (
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    ):
        if os.path.isfile(place):
            return place
    raise SystemExit("no Chrome or Edge to raster the icon")


def raster(browser, svg, size, folder):
    page = os.path.join(folder, f"icon-{size}.html")
    with open(page, "w", encoding="utf-8") as out:
        out.write(
            "<html><body style='margin:0;background:transparent'>"
            f"<img src='file:///{svg}' width={size} height={size} style='display:block'>"
            "</body></html>"
        )
    shot = os.path.join(folder, f"icon-{size}.png")
    subprocess.run(
        [
            browser,
            "--headless=new",
            "--disable-gpu",
            "--hide-scrollbars",
            "--default-background-color=00000000",
            f"--user-data-dir={os.path.join(folder, 'profile')}",
            f"--window-size={size},{size}",
            f"--screenshot={shot}",
            f"file:///{page}",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    with open(shot, "rb") as blob:
        return blob.read()


def ico(images):
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries, data = b"", b""
    for size, blob in images:
        edge = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", edge, edge, 0, 0, 1, 32, len(blob), offset)
        offset += len(blob)
        data += blob
    return header + entries + data


def main():
    browser = chrome()
    with tempfile.TemporaryDirectory() as folder:
        images = []
        for size in SIZES:
            svg = SMALL if size <= SMALL_UP_TO else MASTER
            blob = raster(browser, svg, size, folder)
            images.append((size, blob))
            print(f"{size:>3} px  {os.path.basename(svg)}  {len(blob)} bytes")
        os.makedirs(os.path.dirname(ICO), exist_ok=True)
        with open(ICO, "wb") as out:
            out.write(ico(images))
        big = next(blob for size, blob in images if size == 256)
        with open(PNG, "wb") as out:
            out.write(big)
    print(f"wrote {ICO} and {PNG}")


if __name__ == "__main__":
    sys.exit(main())
