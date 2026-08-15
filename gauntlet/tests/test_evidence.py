import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from gauntlet.evidence import FORBIDDEN_KEYS, build_bundle, render_bundle


def _bundle():
    return build_bundle(piece="cxas-proto", repo_root=ROOT, issues=["284"])


def test_bundle_has_required_evidence_keys():
    b = _bundle()
    for key in ("piece", "test_output", "clippy_output", "coverage", "issues", "binary_size"):
        assert key in b, f"missing evidence key {key}"


def test_bundle_excludes_source_code():
    # The critic must be blind. If source ever leaks into the bundle, the loop
    # degenerates into self-grading, which is the failure this design exists to
    # prevent.
    b = _bundle()
    for forbidden in FORBIDDEN_KEYS:
        assert forbidden not in b, f"evidence bundle leaked {forbidden} to the critic"


def test_rendered_bundle_contains_no_rust_source():
    text = render_bundle(_bundle())
    assert "pub enum " not in text
    assert "pub fn " not in text
    assert "impl " not in text


def test_coverage_reports_both_api_versions():
    cov = _bundle()["coverage"]
    assert cov["v1_methods"] == 66
    assert cov["v1beta_methods"] == 104


def test_piece_is_echoed_for_routing():
    assert _bundle()["piece"] == "cxas-proto"


def test_missing_piece_is_recorded_as_a_failure_not_a_pass():
    # A crate that does not exist must not read as "nothing to complain about".
    b = build_bundle(piece="cxas-does-not-exist", repo_root=ROOT, issues=[])
    assert b["test_output"]["exit_code"] != 0
