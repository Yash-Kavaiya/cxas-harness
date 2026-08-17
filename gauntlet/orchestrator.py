#!/usr/bin/env python3
"""Provider-agnostic Gauntlet Loop orchestrator.

Roles are separated structurally, not by instruction:

  orchestrator -- plans, fans out, merges. Never implements, never critiques.
  builder      -- one per piece, sees the task and its own workspace.
  critic       -- blind. Sees only the evidence bundle from evidence.py.

The orchestrator is deliberately dumb: it routes prompts and records verdicts.
Every judgement belongs to a critic, and every judgement is made from evidence
that this file never lets the builder influence.
"""
import json
import re
import shlex
import subprocess
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from gauntlet.evidence import _coverage, build_bundle, render_bundle

AGENTS_DIR = Path(__file__).resolve().parent / "agents"

BUILDER_FALLBACK = "You are a builder. Improve the piece against its quality bar."
CRITIC_FALLBACK = "You are a blind critic. Judge only the evidence you are given."


def load_config(path):
    with open(path, "rb") as fh:
        return tomllib.load(fh)


def parse_verdict(text):
    """Extract the critic's JSON verdict.

    An unparseable response is a FAILED round, never an approval: a critic that
    cannot state a verdict has not granted one.
    """
    for match in re.finditer(r"\{.*?\}", text or "", re.DOTALL):
        try:
            obj = json.loads(match.group(0))
        except json.JSONDecodeError:
            continue
        if "verdict" in obj:
            return {
                "score": obj.get("score", 0),
                "verdict": "PASS" if str(obj["verdict"]).upper() == "PASS" else "FAIL",
                "biggest_gap": obj.get("biggest_gap", ""),
            }
    return {"score": 0, "verdict": "FAIL", "biggest_gap": "unparseable critic response"}


def invoke_agent_result(agent_cmd, prompt, timeout=1800):
    """Run any agent CLI that reads stdin and writes stdout.

    No provider SDK is imported; swapping providers is a config change.

    Returns the exit status alongside the output. The builder's status used to
    be discarded, so an agent that crashed halfway through an edit was
    critiqued as though it had stopped deliberately -- a wasted round, and a
    partially-applied edit graded as intent.
    """
    try:
        proc = subprocess.run(
            shlex.split(agent_cmd),
            input=prompt,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return {
            "ok": proc.returncode == 0,
            "stdout": proc.stdout,
            "detail": "" if proc.returncode == 0 else f"exit code {proc.returncode}",
        }
    except (FileNotFoundError, OSError, subprocess.TimeoutExpired) as exc:
        return {"ok": False, "stdout": f"agent invocation failed: {exc}", "detail": str(exc)}


def invoke_agent(agent_cmd, prompt, timeout=1800):
    """Output only. A failed invocation returns prose, not JSON, so
    `parse_verdict` resolves it to FAIL: a missing or hung agent must never
    read as a passing critique."""
    return invoke_agent_result(agent_cmd, prompt, timeout)["stdout"]


def _role_prompt(name, fallback):
    path = AGENTS_DIR / f"{name}.md"
    return path.read_text(encoding="utf-8") if path.exists() else fallback


class CallBudget:
    """Counts agent invocations and refuses to hand out more than the cap.

    Counted rather than priced: `agent_cmd` is any CLI reading stdin and
    writing stdout, so no cost ever comes back and a dollar budget could only
    have been decorative. A cap that is checked is worth more than a figure
    that is not.
    """

    def __init__(self, limit):
        self.limit = int(limit or 0)
        self.used = 0

    def spend(self):
        """Claim one invocation. False when the cap is already reached."""
        if self.limit and self.used >= self.limit:
            return False
        self.used += 1
        return True

    def exhausted(self):
        return bool(self.limit) and self.used >= self.limit


def rc_gate(config, repo_root):
    """Whether a clean sweep also clears the release-candidate bar.

    Separate from the per-piece verdict on purpose: every critic passing means
    nine crates are individually defensible, which is not the same as the
    workspace being releasable.
    """
    minimum = int(config.get("rc_coverage_min", 0) or 0)
    coverage = _coverage(repo_root)
    declared = coverage["v1_methods"] + coverage["v1beta_methods"]
    return {
        "required": minimum,
        "declared": declared,
        "release_candidate": declared >= minimum,
    }


def run_piece(piece, config, repo_root, run_dir, budget=None):
    """Build and critique a single piece until its critic passes or rounds run out."""
    repo_root, run_dir = Path(repo_root), Path(run_dir)
    run_dir.mkdir(parents=True, exist_ok=True)
    budget = budget if budget is not None else CallBudget(config.get("max_agent_calls", 0))

    issues = config.get("issues", {}).get(piece, [])
    agent_cmd = config["agent_cmd"]
    max_rounds = int(config.get("max_rounds", 8))

    builder_role = _role_prompt("builder", BUILDER_FALLBACK)
    critic_role = _role_prompt("critic", CRITIC_FALLBACK)

    history = []
    verdict = {"verdict": "FAIL", "score": 0, "biggest_gap": "not yet run"}

    for round_no in range(1, max_rounds + 1):
        # Two calls per round, so a round that cannot afford both is not
        # started. Spending the builder call and then stopping would leave the
        # crate edited and unreviewed, which is the worst of both.
        if budget.limit and budget.limit - budget.used < 2:
            verdict = {
                "verdict": "FAIL",
                "score": verdict["score"],
                "biggest_gap": (
                    f"agent-call cap reached ({budget.used}/{budget.limit}); "
                    "raise max_agent_calls in gauntlet/config.toml to continue"
                ),
            }
            history.append({"round": round_no, "budget_exhausted": True, **verdict})
            print(f"[{piece}] stopped: {verdict['biggest_gap']}")
            break

        gap = verdict["biggest_gap"] if round_no > 1 else ""
        builder_prompt = (
            f"{builder_role}\n\n"
            f"Piece: {piece}\n"
            f"Assigned issues: {', '.join(issues) or 'none'}\n"
            + (f"Top-priority fix from the critic: {gap}\n" if gap else "")
        )
        budget.spend()
        built = invoke_agent_result(agent_cmd, builder_prompt)
        if not built["ok"]:
            # A crashed builder is a failed round, matching the discipline
            # already applied to the critic. Critiquing a half-applied edit
            # grades an accident as though it were a decision.
            verdict = {
                "verdict": "FAIL",
                "score": 0,
                "biggest_gap": f"builder invocation failed: {built['detail']}",
            }
            history.append({"round": round_no, "builder_failed": True, **verdict})
            print(f"[{piece}] round {round_no}: builder failed -- {built['detail']}")
            continue

        # Evidence is collected by code, after the builder has finished, so the
        # builder cannot shape what the critic sees.
        bundle = build_bundle(piece=piece, repo_root=repo_root, issues=issues)
        (run_dir / f"evidence-round-{round_no}.md").write_text(
            render_bundle(bundle), encoding="utf-8"
        )

        critic_prompt = (
            f"{critic_role}\n\n{render_bundle(bundle)}\n\n"
            "Respond with JSON only: "
            '{"score": <0-100>, "verdict": "PASS"|"FAIL", "biggest_gap": "<one gap>"}'
        )
        budget.spend()
        verdict = parse_verdict(invoke_agent(agent_cmd, critic_prompt))
        history.append({"round": round_no, **verdict})

        scorecard = {"piece": piece, "rounds": round_no, "history": history, **verdict}
        (run_dir / "scorecard.json").write_text(
            json.dumps(scorecard, indent=2), encoding="utf-8"
        )
        print(
            f"[{piece}] round {round_no}: {verdict['verdict']} "
            f"({verdict['score']}) {verdict['biggest_gap']}"
        )

        if verdict["verdict"] == "PASS":
            break

    return {"piece": piece, "rounds": len(history), "history": history, **verdict}


def main(argv):
    here = Path(__file__).resolve().parent
    repo_root = here.parent
    config = load_config(here / "config.toml")
    pieces = [argv[0]] if argv else config["pieces"]

    budget = CallBudget(config.get("max_agent_calls", 0))
    results = [
        run_piece(p, config, repo_root, here / "runs" / p, budget=budget) for p in pieces
    ]

    failed = [r["piece"] for r in results if r["verdict"] != "PASS"]
    print(f"\n{len(results) - len(failed)}/{len(results)} pieces passed")
    if budget.limit:
        print(f"agent calls: {budget.used}/{budget.limit}")
    if failed:
        print(f"still failing: {', '.join(failed)}")
        if budget.exhausted():
            print(
                "the run stopped on the agent-call cap, not on a verdict -- "
                "raise max_agent_calls in gauntlet/config.toml to go further"
            )
        return 1

    gate = rc_gate(config, repo_root)
    if not gate["release_candidate"]:
        # Every critic passing is not the same as the workspace being
        # releasable, so a clean sweep below the bar exits non-zero rather than
        # reading as a release candidate.
        print(
            f"not a release candidate: {gate['declared']} CES methods declared, "
            f"rc_coverage_min is {gate['required']}"
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
