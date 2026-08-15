#!/usr/bin/env python3
"""Deterministic evidence bundle for blind critics.

This is code, not an agent. The critic sees exactly what this produces and
nothing else -- no source, no diff, no commit messages, no builder rationale.
That exclusion is what makes a critic unpersuadable by explanation, and it is
asserted by `tests/test_evidence.py::test_bundle_excludes_source_code`.
"""
import json
import subprocess
from pathlib import Path

# Keys that must never appear in a bundle. Asserted by the test suite so the
# blindness guarantee survives future edits to this file.
FORBIDDEN_KEYS = frozenset(
    {"source", "diff", "rationale", "commit_message", "builder_notes"}
)


def _run(cmd, cwd, timeout=900):
    try:
        proc = subprocess.run(
            cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, shell=False
        )
        return {
            "exit_code": proc.returncode,
            "stdout": proc.stdout[-20000:],
            "stderr": proc.stderr[-20000:],
        }
    except FileNotFoundError:
        # A missing toolchain is a failed round, never a silent pass.
        return {"exit_code": 127, "stdout": "", "stderr": f"not found: {cmd[0]}"}
    except subprocess.TimeoutExpired:
        return {"exit_code": 124, "stdout": "", "stderr": f"timed out after {timeout}s"}


def _coverage(repo_root):
    """Count CES methods the vendored discovery documents declare."""
    ref = Path(repo_root) / "reference" / "ces"
    out = {}
    for version in ("v1", "v1beta"):
        path = ref / f"{version}.discovery.json"
        count = 0
        revision = "missing"
        if path.exists():
            doc = json.loads(path.read_text(encoding="utf-8"))
            revision = doc.get("revision", "unknown")

            def walk(resources):
                nonlocal count
                for res in (resources or {}).values():
                    count += len(res.get("methods") or {})
                    walk(res.get("resources"))

            walk(doc.get("resources"))
        out[f"{version}_methods"] = count
        out[f"{version}_revision"] = revision
    return out


def _binary_size(repo_root):
    for candidate in (
        Path(repo_root) / "target" / "release" / "cxas.exe",
        Path(repo_root) / "target" / "release" / "cxas",
    ):
        if candidate.exists():
            return candidate.stat().st_size
    return 0


def build_bundle(piece, repo_root, issues):
    """Collect everything a blind critic is allowed to see about `piece`."""
    repo_root = Path(repo_root)
    return {
        "piece": piece,
        "issues": list(issues),
        "test_output": _run(["cargo", "test", "-p", piece], repo_root),
        "clippy_output": _run(
            ["cargo", "clippy", "-p", piece, "--all-targets"], repo_root
        ),
        "coverage": _coverage(repo_root),
        "binary_size": _binary_size(repo_root),
    }


def render_bundle(bundle):
    """Format a bundle as the critic's prompt input."""
    cov = bundle["coverage"]
    return "\n".join(
        [
            f"# Evidence for piece: {bundle['piece']}",
            f"Assigned issues: {', '.join(bundle['issues']) or 'none'}",
            "",
            "## cargo test",
            f"exit_code: {bundle['test_output']['exit_code']}",
            "```",
            bundle["test_output"]["stdout"] or bundle["test_output"]["stderr"],
            "```",
            "",
            "## cargo clippy",
            f"exit_code: {bundle['clippy_output']['exit_code']}",
            "```",
            bundle["clippy_output"]["stderr"] or bundle["clippy_output"]["stdout"],
            "```",
            "",
            "## CES discovery coverage",
            f"v1: {cov['v1_methods']} methods (revision {cov['v1_revision']})",
            f"v1beta: {cov['v1beta_methods']} methods (revision {cov['v1beta_revision']})",
            "",
            "## Binary size",
            f"{bundle['binary_size']} bytes",
        ]
    )
