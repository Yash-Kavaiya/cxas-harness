"""Per-issue evidence, edit scope, and builder-crash handling.

The critic prompt has always asked whether each assigned issue is "genuinely
closed". Until now the bundle answered with the issue *number* and nothing
else, so a critic could do no better than trust a test name -- which is the
precise failure that created this project: #284's closing test passed for
months while asserting an enum CES has never used.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from gauntlet.evidence import (
    FORBIDDEN_KEYS,
    _edit_scope,
    _issue_repro,
    _issue_tests,
    _render_issues,
    _render_scope,
    _test_spans,
    build_bundle,
    render_bundle,
)

SAMPLE = '''
//! Module docs mentioning #999 outside any test.

/// Source function whose doc comment cites #284.
pub fn from_wire_name(name: &str) -> Self { todo!() }

#[test]
fn closes_the_enum_drift_bug() {
    // #284: CES declares QUEUED, not PENDING.
    assert_eq!(1, 1);
}

#[tokio::test]
async fn quota_is_reported() {
    // #263
    let brace_in_string = "a } that must not end the span";
    assert!(true);
}

#[test]
fn unrelated() {
    assert!(true);
}
'''


# ------------------------------------------------------------ span scanning


def test_only_annotated_tests_are_scanned():
    # A source function citing an issue can never appear in `cargo test`
    # output, so counting it would report a permanent "not run" for every
    # issue and turn the whole signal into noise.
    names = [name for name, _ in _test_spans(SAMPLE)]
    assert names == ["closes_the_enum_drift_bug", "quota_is_reported", "unrelated"]
    assert "from_wire_name" not in names


def test_a_brace_inside_a_string_does_not_end_a_span():
    spans = dict(_test_spans(SAMPLE))
    assert "must not end the span" in spans["quota_is_reported"]
    assert spans["quota_is_reported"].rstrip().endswith("}")


def test_a_citation_outside_every_test_is_not_credited(tmp_path):
    # #999 appears only in the module doc comment. Crediting it would let a
    # file-level comment stand in for a test, which is the same trust this
    # bundle exists to remove.
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")
    assert _issue_tests(tmp_path, "demo", "999") == []


def test_a_citation_inside_a_test_is_credited(tmp_path):
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")
    assert _issue_tests(tmp_path, "demo", "284") == ["closes_the_enum_drift_bug"]
    assert _issue_tests(tmp_path, "demo", "263") == ["quota_is_reported"]


def test_a_longer_issue_number_is_not_matched_by_a_prefix(tmp_path):
    # #28 must not match #284, or every short issue number would look closed.
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")
    assert _issue_tests(tmp_path, "demo", "28") == []


def test_a_missing_crate_yields_no_tests_rather_than_raising():
    assert _issue_tests(ROOT, "cxas-does-not-exist", "284") == []


# --------------------------------------------------------------- repro rows


def _fake_run(stdout):
    return {"exit_code": 0, "stdout": stdout, "stderr": ""}


def test_an_issue_with_no_named_test_is_reported_as_such(tmp_path):
    rows = _issue_repro(tmp_path, "demo", ["404"], _fake_run(""))
    assert rows[0]["status"] == "NO TEST NAMES THIS ISSUE"
    assert rows[0]["tests"] == []


def test_a_passing_named_test_is_reported_as_passing(tmp_path):
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")

    rows = _issue_repro(
        tmp_path, "demo", ["284"], _fake_run("test closes_the_enum_drift_bug ... ok")
    )
    assert rows[0]["status"] == "ALL NAMED TESTS PASSED"
    assert rows[0]["passed"] == 1


def test_a_failing_named_test_outranks_a_passing_one(tmp_path):
    # A piece where one closing test passes and another fails is failing.
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")

    rows = _issue_repro(
        tmp_path, "demo", ["284"], _fake_run("test closes_the_enum_drift_bug ... FAILED")
    )
    assert rows[0]["status"] == "FAILING"


def test_a_named_test_that_never_ran_is_not_counted_as_passing(tmp_path):
    # A crate that fails to compile produces no test lines at all. Reading that
    # as "nothing failed" would be the loudest possible false pass.
    crate = tmp_path / "crates" / "demo" / "tests"
    crate.mkdir(parents=True)
    (crate / "sample.rs").write_text(SAMPLE, encoding="utf-8")

    rows = _issue_repro(tmp_path, "demo", ["284"], _fake_run("error: could not compile"))
    assert rows[0]["status"] == "NOT RUN"
    assert rows[0]["passed"] == 0


def test_the_rendering_states_what_a_green_test_does_and_does_not_prove():
    text = render_bundle(build_bundle(piece="cxas-proto", repo_root=ROOT, issues=["284"]))
    assert "not proof" in text, "the critic must not read a tick as a closed issue"


# -------------------------------------------------------------- edit scope


def test_scope_separates_the_piece_from_everything_else():
    scope = _edit_scope(ROOT, "cxas-proto")
    assert scope["available"], scope
    for path in scope["in_scope"]:
        assert path.startswith("crates/cxas-proto/")
    for path in scope["out_of_scope"]:
        assert not path.startswith("crates/cxas-proto/")


def test_scope_reports_paths_and_never_content():
    # builder.md forbids editing outside the piece and nothing checked. File
    # names say where a change landed without saying what it was, so the
    # critic's blindness survives.
    rendered = _render_scope(
        {
            "available": True,
            "in_scope": ["crates/cxas-proto/src/lib.rs"],
            "out_of_scope": ["crates/cxas-core/src/auth.rs"],
            "detail": "",
        }
    )
    assert "OUT OF SCOPE" in rendered
    assert "crates/cxas-core/src/auth.rs" in rendered
    assert "pub fn " not in rendered
    assert "impl " not in rendered


def test_unavailable_scope_says_so_rather_than_claiming_a_clean_tree():
    rendered = _render_scope({"available": False, "detail": "not a git repository"})
    assert "unavailable" in rendered
    assert "no out-of-scope edits" not in rendered


# ------------------------------------------------------------- blindness


def test_the_enlarged_bundle_is_still_blind():
    bundle = build_bundle(piece="cxas-proto", repo_root=ROOT, issues=["284"])
    for forbidden in FORBIDDEN_KEYS:
        assert forbidden not in bundle, f"bundle leaked {forbidden}"

    text = render_bundle(bundle)
    assert "pub enum " not in text
    assert "pub fn " not in text
    assert "impl " not in text


def test_the_bundle_carries_the_new_evidence_keys():
    bundle = build_bundle(piece="cxas-proto", repo_root=ROOT, issues=["284"])
    assert "issue_repro" in bundle
    assert "edit_scope" in bundle


def test_no_issues_renders_as_a_statement_not_a_blank():
    assert "no issues assigned" in _render_issues([])
