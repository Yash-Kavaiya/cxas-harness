#!/usr/bin/env python3
# Copyright 2026 The cxas-harness Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
"""Emit the CES method table from the vendored discovery documents.

The table is checked in rather than built at compile time on purpose: a
`build.rs` that parses JSON would make every downstream build depend on the
reference files, and one that *fetched* them would make builds depend on the
network. Checking it in keeps the build hermetic; `cxas-parity`'s
`declared_table_matches_discovery_exactly` then fails if the checked-in table
and the pinned reference ever disagree, so a stale table cannot survive a
reference refresh.

Usage:
    python tools/generate_methods.py            # rewrite the table
    python tools/generate_methods.py --check    # fail if it is stale
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REFERENCE = ROOT / "reference" / "ces"
TARGET = ROOT / "crates" / "cxas-core" / "src" / "rest" / "method_table.rs"
VERSIONS = ("v1", "v1beta")

LICENSE = """// Copyright 2026 The cxas-harness Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
"""


def collect(resources: dict | None, out: list) -> None:
    """Walk the nested `resources` tree, collecting every declared method."""
    for _, resource in sorted((resources or {}).items()):
        for _, method in sorted((resource.get("methods") or {}).items()):
            out.append((method["id"], method["httpMethod"], method["path"]))
        collect(resource.get("resources"), out)


def methods_for(version: str) -> list[tuple[str, str, str]]:
    doc = json.loads((REFERENCE / f"{version}.discovery.json").read_text(encoding="utf-8"))
    found: list[tuple[str, str, str]] = []
    collect(doc.get("resources"), found)
    if not found:
        raise SystemExit(f"{version}: discovery document declares no methods")
    found.sort()
    return found


def render() -> str:
    per_version = {v: methods_for(v) for v in VERSIONS}
    total = sum(len(m) for m in per_version.values())
    counts = ", ".join(f"{v} {len(per_version[v])}" for v in VERSIONS)

    lines = [
        LICENSE,
        "//! Every CES REST method, generated from the vendored discovery documents.",
        "//!",
        "//! Do not edit by hand: run `python tools/generate_methods.py`. The",
        "//! `declared_table_matches_discovery_exactly` parity test fails if this file",
        "//! and `reference/ces/` ever disagree, in either direction.",
        "//!",
        f"//! {total} methods ({counts}) at the revisions recorded in `PINNED.toml`.",
        "",
        "use super::method::{beta, v1, MethodSpec};",
        "",
        "/// Every method CES declares, addressable through [`super::RequestBuilder`].",
        "///",
        "/// Being addressable is not the same as being wrapped in a typed helper:",
        "/// see `MODELLED` for the subset this workspace models with its own types.",
        "pub const METHODS: &[MethodSpec] = &[",
    ]

    for version in VERSIONS:
        ctor = "v1" if version == "v1" else "beta"
        lines.append(f"    // ---- {version} ({len(per_version[version])} methods) ----")
        for mid, verb, path in per_version[version]:
            lines.append(f'    {ctor}("{mid}", "{verb}", "{path}"),')
    lines.append("];")
    lines.append("")
    return "\n".join(lines)


def display(path: Path) -> str:
    """A path to show a human, relative to the repo when it is inside it."""
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        # Under test, or pointed elsewhere deliberately. Reporting the absolute
        # path is better than crashing while reporting an error.
        return str(path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the table is stale")
    args = parser.parse_args()

    rendered = render()
    if args.check:
        current = TARGET.read_text(encoding="utf-8") if TARGET.exists() else ""
        if current != rendered:
            print(f"STALE {display(TARGET)}: re-run tools/generate_methods.py", file=sys.stderr)
            return 1
        print(f"OK {display(TARGET)} matches reference/ces/")
        return 0

    TARGET.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"wrote {display(TARGET)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
