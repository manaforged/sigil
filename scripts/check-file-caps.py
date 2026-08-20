#!/usr/bin/env python3
"""Shrink-only ratchet for the 400-line production file cap.

Fails when a production file (`crates/**/src/**/*.rs`) exceeds 400 lines
without being grandfathered, or when a grandfathered file grows past its
recorded count. The over-cap population can only shrink.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

CAP = 400
BASELINE = Path("scripts/file-caps-baseline.txt")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write-baseline",
        action="store_true",
        help="Regenerate the grandfather list from the current tree.",
    )
    return parser.parse_args()


def production_files(root: Path) -> list[Path]:
    return sorted(root.glob("crates/*/src/**/*.rs"))


def line_counts(files: list[Path]) -> dict[str, int]:
    return {str(path): sum(1 for _ in path.open("r", encoding="utf-8")) for path in files}


def load_baseline(path: Path) -> dict[str, int]:
    baseline: dict[str, int] = {}
    if not path.exists():
        return baseline
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        file, _, count = line.rpartition("\t")
        baseline[file] = int(count)
    return baseline


def main() -> int:
    args = parse_args()
    counts = line_counts(production_files(Path(".")))

    if args.write_baseline:
        over = {f: n for f, n in counts.items() if n > CAP}
        BASELINE.write_text(
            "".join(f"{f}\t{n}\n" for f, n in sorted(over.items())), encoding="utf-8"
        )
        print(f"wrote {len(over)} grandfathered files to {BASELINE}")
        return 0

    baseline = load_baseline(BASELINE)
    failures: list[str] = []
    cured: list[str] = []

    for file, lines in sorted(counts.items()):
        grandfathered = baseline.get(file)
        if lines <= CAP:
            if grandfathered is not None:
                cured.append(file)
        elif grandfathered is None:
            failures.append(f"{file}: {lines} lines exceeds {CAP} (not grandfathered)")
        elif lines > grandfathered:
            failures.append(
                f"{file}: {lines} lines grew past grandfathered {grandfathered}"
            )

    for file in cured:
        print(f"cured (now under {CAP}, prune from baseline): {file}")
    if failures:
        print("\nFile-cap gate failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    over = sum(1 for n in counts.values() if n > CAP)
    print(f"ok: {over} production files over {CAP} lines (shrink-only)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
