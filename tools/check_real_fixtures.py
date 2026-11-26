#!/usr/bin/env python3
"""Export the CI-positive real skeletons and audit what a reader gets.

Reads `fixtures/real/ci-positive.txt`, exports each skeleton through Green's
own CLI, then checks the PDF: total size, fully embedded subset fonts, no
image XObjects. Every failed check names the file, the check, actual and
expected, so a red job points at a file instead of an anonymous assert.

Usage:
    python tools/check_real_fixtures.py --green target/debug/green.exe \
        --out <scratch-dir>
"""

import argparse
import os
import pathlib
import re
import shutil
import subprocess
import sys
import time

SIZE_LIMIT = 3 * 1024 * 1024
FONT_ROW = re.compile(r"\s+(yes|no)\s+(yes|no)\s+(yes|no)\s+\d+\s+\d+\s*$")
SUBSET = re.compile(r"^[A-Z]{6}\+")
IMAGE = re.compile(rb"/Subtype\s*/Image\b")
FONT_NAME = re.compile(rb"/BaseFont\s*/([^\s/>\[]+)")
DESCRIPTOR = re.compile(rb"/FontDescriptor\s*<<(.{0,1200}?)>>", re.S)


def env_with(green, data):
    child = os.environ.copy()
    child["ELY_DATA_ROOT"] = str(data)
    return child


def wait_for_door(data, seconds=60):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if data.exists() and any(p.name == "instance.door" for p in data.rglob("instance.door")):
            return True
        time.sleep(0.2)
    return False


def export(green, fixture, pdf, data, errors):
    data.mkdir(parents=True, exist_ok=True)
    app = subprocess.Popen(
        [str(green), str(fixture)],
        env=env_with(green, data),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        if not wait_for_door(data):
            errors.append(f"{fixture} [start]: actual=no instance.door expected=an app holding {data}")
            return
        done = subprocess.run(
            [str(green), "export", str(fixture), str(pdf)],
            env=env_with(green, data),
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=180,
        )
        if done.returncode:
            said = (done.stderr or done.stdout).strip().replace("\n", " | ")[:400]
            errors.append(f"{fixture} [export]: actual=exit {done.returncode}: {said} expected=exit 0")
    except (OSError, subprocess.TimeoutExpired) as error:
        errors.append(f"{fixture} [export]: actual={error} expected=a PDF written")
    finally:
        app.terminate()
        try:
            app.wait(timeout=15)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait(timeout=15)
        shutil.rmtree(data, ignore_errors=True)


def fonts_from_pdffonts(pdf):
    out = subprocess.run(
        ["pdffonts", str(pdf)],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    ).stdout
    rows = [m.groups() for line in out.splitlines() if (m := FONT_ROW.search(line))]
    return [(emb, sub) for emb, sub, _ in rows]


def fonts_from_pdf(pdf):
    raw = pdf.read_bytes()
    names = [m.group(1).decode("latin1") for m in FONT_NAME.finditer(raw)]
    subsets = {n[:7] for n in names if SUBSET.match(n)}
    descriptors = [m.group(1) for m in DESCRIPTOR.finditer(raw)]
    embedded = sum(
        1
        for prefix in subsets
        if any(
            prefix.encode("latin1") in d
            and (b"/FontFile2" in d or b"/FontFile3" in d)
            for d in descriptors
        )
    )
    return names, len(subsets), embedded


def images(pdf):
    if shutil.which("pdfimages"):
        out = subprocess.run(
            ["pdfimages", "-list", str(pdf)],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=True,
        ).stdout
        return sum(1 for line in out.splitlines() if re.match(r"^\s*\d+\s+\d+\s+", line)), "pdfimages"
    return len(IMAGE.findall(pdf.read_bytes())), "pdf bytes"


def audit(pdf, fixture, errors):
    size = pdf.stat().st_size
    if size > SIZE_LIMIT:
        errors.append(f"{fixture} [size]: actual={size}B expected<={SIZE_LIMIT}B")
    if shutil.which("pdffonts"):
        fonts = fonts_from_pdffonts(pdf)
        if not fonts:
            errors.append(f"{fixture} [fonts]: actual=0 font rows expected=at least one embedded subset font")
        bad = [(emb, sub) for emb, sub in fonts if emb != "yes" or sub != "yes"]
        if bad:
            errors.append(f"{fixture} [pdffonts sub]: actual={bad} expected=all emb=yes sub=yes")
        font_note = f"fonts {len(fonts)} with emb/sub=yes"
    else:
        names, subsets, embedded = fonts_from_pdf(pdf)
        if not names:
            errors.append(f"{fixture} [fonts]: actual=no /BaseFont expected=at least one embedded subset font")
        if len(subsets) != len(names):
            errors.append(f"{fixture} [font subset]: actual={names} expected=a 6-letter subset prefix on every name")
        if embedded != len(subsets):
            errors.append(f"{fixture} [font embed]: actual={embedded}/{len(subsets)} expected=every font embedded")
        font_note = f"fonts {embedded}/{len(subsets)} embedded subsets (pdf bytes)"
    count, via = images(pdf)
    if count:
        errors.append(f"{fixture} [images]: actual={count} expected=0 (via {via})")
    return f"size={size}B, {font_note}, images={count} ({via})"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--green", type=pathlib.Path, required=True)
    parser.add_argument("--out", type=pathlib.Path, required=True)
    parser.add_argument("--list", type=pathlib.Path, default=pathlib.Path("fixtures/real/ci-positive.txt"))
    args = parser.parse_args()
    if not args.green.is_file():
        sys.exit(f"check_real_fixtures: {args.green} is not a Green binary")
    if not os.environ.get("GREEN_TYPST"):
        sys.exit("check_real_fixtures: GREEN_TYPST is not set; use the Typst installed by the test job")
    lines = [line.strip() for line in args.list.read_text(encoding="utf-8").splitlines() if line.strip()]
    if not lines:
        sys.exit(f"check_real_fixtures: {args.list} is empty")

    args.out.mkdir(parents=True, exist_ok=True)
    errors = []
    report = []
    for number, listed in enumerate(lines, 1):
        fixture = pathlib.Path(listed)
        if not fixture.is_file():
            errors.append(f"{fixture} [fixture]: actual=missing expected=a committed skeleton")
            continue
        pdf = args.out / f"{number}.pdf"
        if pdf.exists():
            pdf.unlink()
        export(args.green, fixture, pdf, args.out / f"data-{number}", errors)
        if not pdf.is_file():
            errors.append(f"{fixture} [pdf]: actual=missing expected=a written PDF")
            continue
        try:
            report.append(f"OK {fixture}: {audit(pdf, fixture, errors)}")
        except (OSError, subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
            errors.append(f"{fixture} [audit]: actual={error} expected=readable PDF audit")
    for line in report:
        print(line)
    for line in errors:
        print(f"FAIL {line}", file=sys.stderr)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as out:
            out.write("\n".join(f"- {line}" for line in report + errors) + "\n")
    if errors:
        sys.exit(f"check_real_fixtures: {len(errors)} failed checks")
    print(f"check_real_fixtures: {len(lines)} real skeletons export and audit clean")


if __name__ == "__main__":
    main()
