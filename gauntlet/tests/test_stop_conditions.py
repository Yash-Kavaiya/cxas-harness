"""The stop conditions, which used to exist only in prose.

`budget_usd` and `rc_coverage_min` were both documented as the loop's
deliberate stop conditions and neither was read by any code. That is exactly
the failure this repository was built to catch, and it mattered more here than
elsewhere: the control nobody had implemented was the one standing between a
user and 144 model invocations.

So these tests exist to make the claim falsifiable. A cap that is only
described is not a cap.
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

import pytest

from gauntlet.orchestrator import CallBudget, load_config, rc_gate, run_piece

STUB = f'"{sys.executable}" "{ROOT / "gauntlet" / "tests" / "stub_agent.py"}"'


@pytest.fixture
def always_failing_critic(monkeypatch):
    """A critic that never passes, so the loop runs until something stops it.

    Without this the stub passes on round one and every cap looks like it
    works, which is precisely the kind of test that proves nothing.
    """
    monkeypatch.setenv("GAUNTLET_STUB_MODE", "fail")


def _config(**over):
    cfg = {
        "agent_cmd": STUB,
        "max_rounds": 8,
        "pieces": ["cxas-proto"],
        "issues": {"cxas-proto": ["284"]},
        "rc_coverage_min": 0,
        "max_agent_calls": 0,
    }
    cfg.update(over)
    return cfg


# --------------------------------------------------------------- CallBudget


def test_a_zero_cap_means_unlimited():
    # The documented meaning of 0, and the default for anyone who never edits
    # the config. Turning 0 into "no calls allowed" would silently disable the
    # loop for every existing user.
    budget = CallBudget(0)
    for _ in range(100):
        assert budget.spend()
    assert not budget.exhausted()


def test_a_cap_stops_handing_out_calls_once_reached():
    budget = CallBudget(3)
    assert [budget.spend() for _ in range(5)] == [True, True, True, False, False]
    assert budget.used == 3
    assert budget.exhausted()


def test_a_cap_never_overspends_by_one():
    # The off-by-one that matters: a check written as `used > limit` lets one
    # extra call through, and one extra call is one extra model invocation.
    budget = CallBudget(1)
    assert budget.spend()
    assert not budget.spend()
    assert budget.used == 1


def test_a_missing_cap_is_treated_as_unlimited_not_as_zero():
    assert CallBudget(None).spend()


# ------------------------------------------------------- enforcement in-loop


def test_the_cap_actually_stops_the_loop(tmp_path, always_failing_critic):
    # The stub critic always fails, so without a cap this would run all 8
    # rounds. With room for two rounds it must stop after two.
    budget = CallBudget(4)
    result = run_piece(
        "cxas-proto",
        _config(max_rounds=8),
        ROOT,
        tmp_path / "run",
        budget=budget,
    )
    assert budget.used == 4
    assert result["rounds"] <= 3, result["history"]


def test_a_round_that_cannot_afford_both_calls_is_never_started(tmp_path, always_failing_critic):
    # Spending the builder call and then stopping would leave the crate edited
    # and unreviewed -- the worst of both. An odd cap must not buy half a round.
    budget = CallBudget(3)
    run_piece("cxas-proto", _config(max_rounds=8), ROOT, tmp_path / "run", budget=budget)
    assert budget.used == 2, "an odd cap bought half a round"


def test_stopping_on_the_cap_is_reported_as_a_failure_not_a_pass(tmp_path, always_failing_critic):
    # Running out of budget is not approval. A loop that reported PASS here
    # would hand back an unreviewed crate as though a critic had cleared it.
    budget = CallBudget(2)
    result = run_piece(
        "cxas-proto",
        _config(max_rounds=8),
        ROOT,
        tmp_path / "run",
        budget=budget,
    )
    assert result["verdict"] == "FAIL"


def test_the_cap_message_names_the_knob_that_raises_it(tmp_path, always_failing_critic):
    # Two calls buys exactly one round; the second is refused with a message a
    # reader can act on.
    budget = CallBudget(2)
    result = run_piece(
        "cxas-proto", _config(max_rounds=8), ROOT, tmp_path / "run", budget=budget
    )
    stopped = [h for h in result["history"] if h.get("budget_exhausted")]
    assert stopped, result["history"]
    assert "max_agent_calls" in stopped[0]["biggest_gap"]


def test_one_budget_is_shared_across_pieces(tmp_path, always_failing_critic):
    # Per-piece budgets would multiply the cap by nine, which is not what
    # "hard cap on total agent invocations for one run" says.
    budget = CallBudget(4)
    run_piece("cxas-proto", _config(), ROOT, tmp_path / "a", budget=budget)
    run_piece("cxas-utils", _config(), ROOT, tmp_path / "b", budget=budget)
    assert budget.used == 4


# ------------------------------------------------------------------ RC gate


def test_the_rc_gate_reads_the_real_vendored_reference():
    gate = rc_gate({"rc_coverage_min": 0}, ROOT)
    assert gate["declared"] == 170, gate
    assert gate["release_candidate"]


def test_the_rc_gate_refuses_a_bar_the_reference_cannot_clear():
    # A clean sweep below the bar must not read as a release candidate: every
    # critic passing means nine crates are individually defensible, which is
    # not the same as the workspace being releasable.
    gate = rc_gate({"rc_coverage_min": 5000}, ROOT)
    assert not gate["release_candidate"]
    assert gate["required"] == 5000


def test_a_missing_rc_setting_defaults_to_no_bar():
    assert rc_gate({}, ROOT)["release_candidate"]


def test_the_shipped_config_declares_both_stop_conditions():
    # The regression guard for the original defect: a config key that no code
    # reads is worse than no key at all, because it reads as a control.
    cfg = load_config(ROOT / "gauntlet" / "config.toml")
    assert "max_agent_calls" in cfg
    assert "rc_coverage_min" in cfg
    assert "budget_usd" not in cfg, (
        "budget_usd cannot be honest here -- agent_cmd is any stdin/stdout CLI, "
        "so no provider ever reports a cost back"
    )
