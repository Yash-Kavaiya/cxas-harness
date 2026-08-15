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

from gauntlet.evidence import build_bundle, render_bundle

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


def invoke_agent(agent_cmd, prompt, timeout=1800):
    """Run any agent CLI that reads stdin and writes stdout.

    No provider SDK is imported; swapping providers is a config change.
    """
    try:
        proc = subprocess.run(
            shlex.split(agent_cmd),
            input=prompt,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return proc.stdout
    except (FileNotFoundError, OSError, subprocess.TimeoutExpired) as exc:
        # Returns prose, not JSON, so parse_verdict resolves it to FAIL. A
        # missing or hung agent must never read as a passing critique.
        return f"agent invocation failed: {exc}"


def _role_prompt(name, fallback):
    path = AGENTS_DIR / f"{name}.md"
    return path.read_text(encoding="utf-8") if path.exists() else fallback


def run_piece(piece, config, repo_root, run_dir):
    """Build and critique a single piece until its critic passes or rounds run out."""
    repo_root, run_dir = Path(repo_root), Path(run_dir)
    run_dir.mkdir(parents=True, exist_ok=True)

    issues = config.get("issues", {}).get(piece, [])
    agent_cmd = config["agent_cmd"]
    max_rounds = int(config.get("max_rounds", 8))

    builder_role = _role_prompt("builder", BUILDER_FALLBACK)
    critic_role = _role_prompt("critic", CRITIC_FALLBACK)

    history = []
    verdict = {"verdict": "FAIL", "score": 0, "biggest_gap": "not yet run"}

    for round_no in range(1, max_rounds + 1):
        gap = verdict["biggest_gap"] if round_no > 1 else ""
        builder_prompt = (
            f"{builder_role}\n\n"
            f"Piece: {piece}\n"
            f"Assigned issues: {', '.join(issues) or 'none'}\n"
            + (f"Top-priority fix from the critic: {gap}\n" if gap else "")
        )
        invoke_agent(agent_cmd, builder_prompt)

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

    results = [run_piece(p, config, repo_root, here / "runs" / p) for p in pieces]

    failed = [r["piece"] for r in results if r["verdict"] != "PASS"]
    print(f"\n{len(results) - len(failed)}/{len(results)} pieces passed")
    if failed:
        print(f"still failing: {', '.join(failed)}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
