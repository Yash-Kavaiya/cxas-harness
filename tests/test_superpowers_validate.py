"""Drive the shipped Superpowers validator against the real spec/plan tree."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

import superpowers_validate as sv  # noqa: E402


class SuperpowersValidateTests(unittest.TestCase):
    def test_validator_passes_on_this_repo(self) -> None:
        report = sv.validate(ROOT)
        if not report.ok:
            detail = "\n".join(
                f"{f.code} {f.path}: {f.message}" for f in report.findings
            )
            self.fail(f"shipped validator rejected the spec/plan set:\n{detail}")
        self.assertGreaterEqual(len(report.specs), 6)
        self.assertGreaterEqual(len(report.plans), 6)
        for spec in report.specs:
            self.assertTrue(spec.endswith("-design.md"), spec)
            self.assertTrue((ROOT / spec).is_file(), spec)
            self.assertGreater((ROOT / spec).stat().st_size, 0)
        for plan in report.plans:
            self.assertTrue((ROOT / plan).is_file(), plan)
            self.assertGreater((ROOT / plan).stat().st_size, 0)

    def test_cli_entry_point_returns_zero(self) -> None:
        code = sv.main(["--root", str(ROOT)])
        self.assertEqual(code, 0)

    def test_catalog_has_exactly_the_twenty_five_open_issues(self) -> None:
        self.assertEqual(len(sv.CATALOGED_ISSUES), 25)
        self.assertEqual(len(set(sv.CATALOGED_ISSUES)), 25)

    def test_placeholder_scan_uses_the_verification_regex(self) -> None:
        """The shipped scanner, not a copy in this test, owns the token list."""
        pattern = sv.PLACEHOLDER.pattern
        for token in (
            "TBD",
            "TODO",
            "implement later",
            "fill in details",
            "Similar to Task",
        ):
            self.assertRegex(token, sv.PLACEHOLDER)

    def test_missing_spec_section_is_reported_by_validator(self) -> None:
        report = sv.Report(ok=True)
        sv.check_spec(Path("synthetic.md"), "# Title\n\n## Architecture\n", report)
        codes = {f.code for f in report.findings}
        self.assertIn("spec_section", codes)
        self.assertFalse(report.ok)


if __name__ == "__main__":
    unittest.main()
