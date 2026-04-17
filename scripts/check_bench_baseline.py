#!/usr/bin/env python3
"""Check Criterion bench output against a committed Sigil baseline."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=Path)
    parser.add_argument("--criterion-dir", type=Path, default=Path("target/criterion"))
    parser.add_argument("--max-regression-percent", type=float, default=20.0)
    parser.add_argument(
        "--budget-only",
        action="store_true",
        help="Check only budget_ns. Use for cross-machine CI smoke gates.",
    )
    return parser.parse_args()


def load_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as file:
        return json.load(file)


def criterion_slope_ns(criterion_dir: Path, bench: str) -> float:
    estimates = load_json(criterion_dir / bench / "new" / "estimates.json")
    try:
        return float(estimates["slope"]["point_estimate"])
    except (KeyError, TypeError, ValueError) as err:
        raise SystemExit(f"missing Criterion slope for {bench}: {err}") from err


def main() -> int:
    args = parse_args()
    baseline = load_json(args.baseline)
    failures: list[str] = []

    for bench in baseline["benches"]:
        name = bench["name"]
        current = criterion_slope_ns(args.criterion_dir, name)
        budget = float(bench["budget_ns"])
        baseline_estimate = float(bench["point_estimate_ns"])
        allowed = baseline_estimate * (1.0 + args.max_regression_percent / 100.0)

        status = "ok"
        if current > budget:
            status = "over budget"
            failures.append(f"{name}: {current:.3f} ns > budget {budget:.3f} ns")
        if not args.budget_only and current > allowed:
            status = "regressed"
            failures.append(f"{name}: {current:.3f} ns > allowed {allowed:.3f} ns")

        print(
            f"{name}: {current:.3f} ns "
            f"(baseline {baseline_estimate:.3f}, budget {budget:.3f}) {status}"
        )

    if failures:
        print("\nBenchmark gate failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
