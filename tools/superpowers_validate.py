"""Validate the Superpowers spec/plan set for cxas-harness.

This is the shipped entry point for the spec/plan quality bar: it reads
docs/superpowers/ (not a copy of those rules) and returns structured findings.
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass, field
from pathlib import Path

SPEC_NAME = re.compile(r"^\d{4}-\d{2}-\d{2}-.+-design\.md$")
PLAN_NAME = re.compile(r"^\d{4}-\d{2}-\d{2}-.+\.md$")
PLACEHOLDER = re.compile(
    r"TBD|TODO|implement later|fill in details|Similar to Task",
    re.IGNORECASE,
)
SPEC_SECTIONS = (
    "Architecture",
    "Components",
    "Data flow",
    "Error handling",
    "Testing",
)
PLAN_HEADER_FIELDS = (
    "**Goal:**",
    "**Architecture:**",
    "**Tech Stack:**",
    "**Spec:**",
    "## Global Constraints",
)
REQUIRED_PHRASES = (
    "cxas-harness",
    "cxas-scrapi",
    "issue-driven quality bar",
)
PHASES = tuple(range(6))
CATALOGED_ISSUES = (
    27,
    46,
    54,
    55,
    86,
    99,
    131,
    136,
    168,
    188,
    206,
    252,
    256,
    263,
    270,
    284,
    298,
    345,
    350,
    355,
    386,
    394,
    397,
    401,
    403,
)
ISSUE_ROW = re.compile(r"\|\s*#(\d+)\s*\|")


@dataclass
class Finding:
    code: str
    path: str
    message: str


@dataclass
class Report:
    ok: bool
    specs: list[str] = field(default_factory=list)
    plans: list[str] = field(default_factory=list)
    findings: list[Finding] = field(default_factory=list)

    def fail(self, code: str, path: Path | str, message: str) -> None:
        self.ok = False
        self.findings.append(Finding(code, str(path), message))


def repo_root() -> Path:
    here = Path(__file__).resolve()
    return here.parent.parent


def docs_root(root: Path) -> Path:
    return root / "docs" / "superpowers"


def list_specs(root: Path) -> list[Path]:
    spec_dir = docs_root(root) / "specs"
    if not spec_dir.is_dir():
        return []
    return sorted(
        p for p in spec_dir.iterdir() if p.is_file() and SPEC_NAME.match(p.name)
    )


def list_plans(root: Path) -> list[Path]:
    plan_dir = docs_root(root) / "plans"
    if not plan_dir.is_dir():
        return []
    return sorted(
        p
        for p in plan_dir.iterdir()
        if p.is_file() and PLAN_NAME.match(p.name) and not p.name.endswith("-design.md")
    )


def scan_placeholders(path: Path, text: str, report: Report) -> None:
    for match in PLACEHOLDER.finditer(text):
        report.fail(
            "placeholder",
            path,
            f"forbidden token {match.group(0)!r} at index {match.start()}",
        )


def check_spec(path: Path, text: str, report: Report) -> None:
    lower_headings = text
    for section in SPEC_SECTIONS:
        heading = re.compile(rf"^## +{re.escape(section)}\s*$", re.MULTILINE)
        if not heading.search(lower_headings):
            report.fail("spec_section", path, f"missing section {section!r}")
    for phrase in REQUIRED_PHRASES:
        if phrase.lower() not in text.lower():
            report.fail("spec_phrase", path, f"missing required phrase {phrase!r}")
    if path.is_file() and path.stat().st_size == 0:
        report.fail("empty", path, "spec file is empty")


def spec_path_from_plan(text: str) -> str | None:
    match = re.search(r"^\*\*Spec:\*\*\s+(\S+)", text, re.MULTILINE)
    if not match:
        return None
    return match.group(1).strip().strip("`").strip("[]")


def check_plan(path: Path, text: str, report: Report, root: Path) -> None:
    for field_name in PLAN_HEADER_FIELDS:
        if field_name not in text:
            report.fail("plan_header", path, f"missing header field {field_name!r}")
    spec_rel = spec_path_from_plan(text)
    if not spec_rel:
        report.fail("plan_spec", path, "Spec: path missing")
    else:
        spec_file = root / spec_rel
        if not spec_file.is_file():
            report.fail("plan_spec", path, f"Spec path does not exist: {spec_rel}")
    if "- [ ]" not in text:
        report.fail("plan_tasks", path, "no checkbox tasks")
    if "```" not in text:
        report.fail("plan_code", path, "no fenced code blocks in tasks")
    if path.is_file() and path.stat().st_size == 0:
        report.fail("empty", path, "plan file is empty")


def check_coverage(root: Path, report: Report) -> None:
    coverage = docs_root(root) / "coverage-map.md"
    if not coverage.is_file():
        report.fail("coverage", coverage, "coverage map missing")
        return
    text = coverage.read_text(encoding="utf-8")
    scan_placeholders(coverage, text, report)
    for phase in PHASES:
        if f"| {phase} |" not in text and f"| {phase} " not in text:
            report.fail("coverage_phase", coverage, f"phase {phase} not listed")
    spec_mentions = re.findall(
        r"docs/superpowers/specs/\d{4}-\d{2}-\d{2}-[a-z0-9-]+-design\.md", text
    )
    plan_mentions = re.findall(
        r"docs/superpowers/plans/\d{4}-\d{2}-\d{2}-[a-z0-9-]+\.md", text
    )
    if len(set(spec_mentions)) < 6:
        report.fail(
            "coverage_phase",
            coverage,
            f"expected 6 spec paths, found {sorted(set(spec_mentions))}",
        )
    if len(set(plan_mentions)) < 6:
        report.fail(
            "coverage_phase",
            coverage,
            f"expected 6 plan paths, found {sorted(set(plan_mentions))}",
        )
    found_issues = {int(n) for n in ISSUE_ROW.findall(text)}
    missing = [n for n in CATALOGED_ISSUES if n not in found_issues]
    if missing:
        report.fail("coverage_issue", coverage, f"issues missing from map: {missing}")
    extra_required = set(CATALOGED_ISSUES)
    if extra_required - found_issues:
        report.fail(
            "coverage_issue",
            coverage,
            f"catalog incomplete: {sorted(extra_required - found_issues)}",
        )
    for spec_rel in set(spec_mentions):
        if not (root / spec_rel).is_file():
            report.fail("coverage_path", coverage, f"spec path missing on disk: {spec_rel}")
    for plan_rel in set(plan_mentions):
        if not (root / plan_rel).is_file():
            report.fail("coverage_path", coverage, f"plan path missing on disk: {plan_rel}")


def validate(root: Path | None = None) -> Report:
    root = root or repo_root()
    report = Report(ok=True)
    specs = list_specs(root)
    plans = list_plans(root)
    report.specs = [str(p.relative_to(root)).replace("\\", "/") for p in specs]
    report.plans = [str(p.relative_to(root)).replace("\\", "/") for p in plans]
    if not specs:
        report.fail("gating_tree", docs_root(root) / "specs", "no YYYY-MM-DD-*-design.md specs")
    if not plans:
        report.fail("gating_tree", docs_root(root) / "plans", "no YYYY-MM-DD-*.md plans")
    for path in specs:
        text = path.read_text(encoding="utf-8")
        scan_placeholders(path, text, report)
        check_spec(path, text, report)
    for path in plans:
        text = path.read_text(encoding="utf-8")
        scan_placeholders(path, text, report)
        check_plan(path, text, report, root)
    check_coverage(root, report)
    return report


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=None,
        help="Repository root (defaults to parent of tools/)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print the full report as JSON",
    )
    args = parser.parse_args(argv)
    report = validate(args.root)
    if args.json:
        print(json.dumps({**asdict(report)}, indent=2))
    else:
        print(f"specs={len(report.specs)} plans={len(report.plans)} ok={report.ok}")
        for spec in report.specs:
            print(f"spec {spec}")
        for plan in report.plans:
            print(f"plan {plan}")
        for finding in report.findings:
            print(f"FAIL {finding.code} {finding.path}: {finding.message}")
        if report.ok:
            print("PASS superpowers spec/plan quality bar")
    return 0 if report.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
