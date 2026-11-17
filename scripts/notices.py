"""Regenerates the dependency tables in THIRD_PARTY_NOTICES.md from what the release build uses.

    python scripts/notices.py

Rust: crates linked into the Windows app (`cargo tree`, normal dependencies only).
JavaScript: packages in package-lock.json that are not dev dependencies (bundled into the UI).
"""

import json
import pathlib
import re
import subprocess
import sys
from collections import defaultdict

ROOT = pathlib.Path(__file__).resolve().parent.parent
NOTICES = ROOT / "THIRD_PARTY_NOTICES.md"
START, END = "<!-- generated:start -->", "<!-- generated:end -->"


def rust():
    out = subprocess.run(
        ["cargo", "tree", "-p", "lovelydl", "-e", "normal", "--target", "x86_64-pc-windows-msvc",
         "--prefix", "none", "--format", "{p}|{l}"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    crates = set()
    for line in out.splitlines():
        name_version, _, license = line.partition("|")
        if "(" in name_version:
            continue  # this workspace's own crates carry their path
        license = license.removesuffix(" (*)")  # cargo's mark for an already listed crate
        name, version = name_version.split(" ")[:2]
        crates.add((name, version.lstrip("v"), license or "see crate"))
    return crates


def npm():
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    packages = set()
    for path, info in lock["packages"].items():
        if not path or info.get("dev"):
            continue
        name = path.split("node_modules/")[-1]
        packages.add((name, info.get("version", "?"), info.get("license", "see package")))
    return packages


def table(items):
    by_license = defaultdict(list)
    for name, version, license in sorted(items):
        by_license[license].append(f"{name} {version}")
    lines = ["| License | Packages |", "|---|---|"]
    for license in sorted(by_license, key=lambda k: (-len(by_license[k]), k)):
        lines.append(f"| {license} | {', '.join(by_license[license])} |")
    return "\n".join(lines)


def main():
    crates, packages = rust(), npm()
    body = (
        f"{START}\n\n### Rust crates ({len(crates)})\n\n{table(crates)}\n\n"
        f"### JavaScript packages ({len(packages)})\n\n{table(packages)}\n\n{END}"
    )
    text = NOTICES.read_text(encoding="utf-8")
    if START not in text:
        sys.exit(f"{NOTICES.name} has no {START} marker")
    text = re.sub(re.escape(START) + ".*?" + re.escape(END), lambda _: body, text, flags=re.S)
    NOTICES.write_text(text, encoding="utf-8")
    print(f"{len(crates)} crates, {len(packages)} packages")


if __name__ == "__main__":
    main()
