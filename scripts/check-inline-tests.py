#!/usr/bin/env python3
"""Reject Rust test bodies embedded in production source files."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOTS = (Path("crates"), Path("examples"))
TEST_FILES = {"test.rs", "tests.rs"}
INLINE_MODULE = re.compile(
    r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]"
    r"(?:\s*#\s*\[[^]]+\])*"
    r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{",
    re.MULTILINE,
)
TEST_FUNCTION = re.compile(
    r"^\s*#\s*\[\s*(?:(?:tokio|async_std)::)?test(?:\([^]]*\))?\s*\]",
    re.MULTILINE,
)


def production(path: Path) -> bool:
    if "src" not in path.parts:
        return False
    name = path.name
    return name not in TEST_FILES and not name.endswith(("_test.rs", "_tests.rs"))


def line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def main() -> int:
    failures: list[str] = []
    files = sorted(path for root in ROOTS for path in root.glob("**/src/**/*.rs"))
    for path in files:
        if not production(path):
            continue
        text = path.read_text(encoding="utf-8")
        for match in INLINE_MODULE.finditer(text):
            failures.append(f"{path}:{line(text, match.start())}: inline test module")
        for match in TEST_FUNCTION.finditer(text):
            failures.append(f"{path}:{line(text, match.start())}: inline test function")

    if failures:
        print("Inline-test gate failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(
            "Move each test to a sibling *_tests.rs file and keep only a cfg(test) path module declaration.",
            file=sys.stderr,
        )
        return 1

    print(f"ok: {len(files)} Rust source files contain no inline test bodies")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
