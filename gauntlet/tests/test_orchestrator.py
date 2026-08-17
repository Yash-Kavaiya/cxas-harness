import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from gauntlet.orchestrator import load_config, parse_verdict, run_piece

STUB = f'"{sys.executable}" "{ROOT / "gauntlet" / "tests" / "stub_agent.py"}"'


def _config(**over):
    cfg = {
        "agent_cmd": STUB,
        "max_rounds": 3,
        "pieces": ["cxas-proto"],
        "issues": {"cxas-proto": ["284"]},
        "rc_coverage_min": 0,
    }
    cfg.update(over)
    return cfg


def test_load_config_reads_agent_cmd_and_pieces():
    cfg = load_config(ROOT / "gauntlet" / "config.toml")
    assert "agent_cmd" in cfg
    assert isinstance(cfg["pieces"], list) and cfg["pieces"]


def test_config_assigns_every_catalogued_issue_to_a_piece():
    cfg = load_config(ROOT / "gauntlet" / "config.toml")
    assigned = {int(i) for ids in cfg["issues"].values() for i in ids}
    catalogued = {
        27, 46, 54, 55, 86, 99, 131, 136, 168, 188, 206, 252, 256, 263,
        270, 284, 298, 345, 350, 355, 386, 394, 397, 401, 403,
    }
    assert catalogued - assigned == set(), f"unassigned issues: {catalogued - assigned}"


def test_every_piece_with_issues_is_in_the_pieces_list():
    cfg = load_config(ROOT / "gauntlet" / "config.toml")
    for piece in cfg["issues"]:
        assert piece in cfg["pieces"], f"{piece} has issues but is never built"


def test_parse_verdict_reads_json_verdict():
    v = parse_verdict('{"score": 95, "verdict": "PASS", "biggest_gap": "none"}')
    assert v["verdict"] == "PASS"
    assert v["score"] == 95


def test_parse_verdict_finds_json_embedded_in_prose():
    v = parse_verdict(
        'Here is my assessment:\n{"score": 10, "verdict": "FAIL", "biggest_gap": "x"}\nDone.'
    )
    assert v["verdict"] == "FAIL"


def test_malformed_verdict_is_a_failed_round_not_approval():
    # Silence is not consent: an unparseable critic response must never be
    # treated as a pass.
    v = parse_verdict("this is not json at all")
    assert v["verdict"] == "FAIL"
    assert "unparseable" in v["biggest_gap"].lower()


def test_empty_verdict_is_a_failed_round():
    assert parse_verdict("")["verdict"] == "FAIL"


def test_run_piece_stops_when_critic_passes(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "pass"
    result = run_piece("cxas-proto", _config(), ROOT, tmp_path)
    assert result["verdict"] == "PASS"
    assert result["rounds"] == 1


def test_run_piece_honours_max_rounds_when_critic_keeps_failing(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "fail"
    result = run_piece("cxas-proto", _config(max_rounds=2), ROOT, tmp_path)
    assert result["verdict"] == "FAIL"
    assert result["rounds"] == 2


def test_run_piece_writes_a_scorecard(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "pass"
    run_piece("cxas-proto", _config(), ROOT, tmp_path)
    assert (tmp_path / "scorecard.json").exists()


def test_run_piece_retains_evidence_per_round_for_audit(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "fail"
    run_piece("cxas-proto", _config(max_rounds=2), ROOT, tmp_path)
    assert (tmp_path / "evidence-round-1.md").exists()
    assert (tmp_path / "evidence-round-2.md").exists()


def test_garbage_verdict_never_counts_as_pass(tmp_path):
    os.environ["GAUNTLET_STUB_MODE"] = "garbage"
    result = run_piece("cxas-proto", _config(max_rounds=1), ROOT, tmp_path)
    assert result["verdict"] == "FAIL"


def test_missing_agent_binary_never_counts_as_pass(tmp_path):
    # If the configured agent CLI is not installed, the piece must fail rather
    # than silently report success.
    result = run_piece(
        "cxas-proto", _config(agent_cmd="definitely-not-an-agent-binary", max_rounds=1), ROOT, tmp_path
    )
    assert result["verdict"] == "FAIL"


def test_a_crashed_builder_is_a_failed_round_not_a_critiqued_one(tmp_path):
    # The builder's exit status used to be discarded, so an agent that died
    # halfway through an edit was critiqued as though it had stopped
    # deliberately: a wasted round, and a partially-applied edit graded as
    # intent. The critic side has always had this discipline; the builder side
    # now matches it.
    os.environ["GAUNTLET_STUB_MODE"] = "crash"
    try:
        result = run_piece("cxas-proto", _config(max_rounds=2), ROOT, tmp_path)
    finally:
        os.environ["GAUNTLET_STUB_MODE"] = "pass"

    assert result["verdict"] == "FAIL"
    assert any(r.get("builder_failed") for r in result["history"]), result["history"]
    assert "builder invocation failed" in result["biggest_gap"]


def test_a_crashed_builder_does_not_burn_the_critic_call(tmp_path):
    # Two calls per round are budgeted; a round that never reaches the critic
    # must not be charged for one.
    from gauntlet.orchestrator import CallBudget

    os.environ["GAUNTLET_STUB_MODE"] = "crash"
    budget = CallBudget(6)
    try:
        run_piece("cxas-proto", _config(max_rounds=3), ROOT, tmp_path, budget=budget)
    finally:
        os.environ["GAUNTLET_STUB_MODE"] = "pass"

    # Three rounds, builder only: three calls, not six.
    assert budget.used == 3, budget.used
