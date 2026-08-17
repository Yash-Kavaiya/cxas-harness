#!/usr/bin/env python3
"""Deterministic evidence bundle for blind critics.

This is code, not an agent. The critic sees exactly what this produces and
nothing else -- no source, no diff, no commit messages, no builder rationale.
That exclusion is what makes a critic unpersuadable by explanation, and it is
asserted by `tests/test_evidence.py::test_bundle_excludes_source_code`.
"""
import json
import re
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


def _test_spans(text):
    """Yield (name, span_text) for every annotated test function in a file.

    Only annotated tests count. Scanning for any `fn` near the issue number
    also matched source functions whose doc comments cite it -- and a source
    function can never appear in `cargo test` output, so every issue would have
    reported a permanent "not run" and the signal would have been noise.
    """
    attr = re.compile(r"#\[(?:tokio::)?test\]")
    sig = re.compile(r"\bfn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")

    for hit in attr.finditer(text):
        name_match = sig.search(text, hit.end())
        if not name_match:
            continue
        open_brace = text.find("{", name_match.end())
        if open_brace == -1:
            continue

        depth, index, in_str, esc = 0, open_brace, False, False
        while index < len(text):
            ch = text[index]
            if in_str:
                if esc:
                    esc = False
                elif ch == "\\":
                    esc = True
                elif ch == '"':
                    in_str = False
            elif ch == '"':
                in_str = True
            elif ch == "{":
                depth += 1
            elif ch == "}":
                depth -= 1
                if depth == 0:
                    break
            index += 1

        yield name_match.group(1), text[hit.start() : index + 1]


def _issue_tests(repo_root, piece, issue):
    """Annotated tests in `piece` whose body or attributes name this issue.

    Found by scanning rather than by a hand-maintained map, because a map is
    one more thing that can silently disagree with the code. A test that stops
    naming its issue stops counting as evidence for it, which is the correct
    direction to fail.
    """
    crate = Path(repo_root) / "crates" / piece
    if not crate.is_dir():
        return []

    needle = re.compile(r"#\s*" + re.escape(str(issue)) + r"\b")
    names = set()

    for path in sorted(crate.rglob("*.rs")):
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        for name, span in _test_spans(text):
            if needle.search(span):
                names.add(name)
    return sorted(names)


def _issue_repro(repo_root, piece, issues, test_output):
    """Per-issue evidence, read out of the test run already performed.

    Deliberately reports what it actually knows -- "a test naming this issue
    ran and passed" -- and not "the issue is closed". Those are different
    claims, and conflating them is precisely how #284 stayed broken through 78
    green tests. The critic is told the difference in the rendered bundle.
    """
    combined = f"{test_output.get('stdout', '')}\n{test_output.get('stderr', '')}"
    report = []

    for issue in issues:
        names = _issue_tests(repo_root, piece, issue)
        if not names:
            report.append(
                {
                    "issue": str(issue),
                    "tests": [],
                    "passed": 0,
                    "failed": 0,
                    "not_run": 0,
                    "status": "NO TEST NAMES THIS ISSUE",
                }
            )
            continue

        passed, failed, not_run = [], [], []
        for name in names:
            if re.search(r"^test .*\b" + re.escape(name) + r"\b.* \.\.\. ok\b", combined, re.M):
                passed.append(name)
            elif re.search(r"^test .*\b" + re.escape(name) + r"\b.* \.\.\. FAILED\b", combined, re.M):
                failed.append(name)
            else:
                not_run.append(name)

        if failed:
            status = "FAILING"
        elif passed and not not_run:
            status = "ALL NAMED TESTS PASSED"
        elif passed:
            status = "PARTIALLY RUN"
        else:
            status = "NOT RUN"

        report.append(
            {
                "issue": str(issue),
                "tests": names,
                "passed": len(passed),
                "failed": len(failed),
                "not_run": len(not_run),
                "status": status,
            }
        )
    return report


def _edit_scope(repo_root, piece):
    """Which paths the builder touched, as paths only -- never content.

    builder.md forbids editing outside the piece; without this nothing checked.
    File names are metadata: they say *where* a change landed without saying
    what it was, so the critic's blindness survives intact.
    """
    changed = _run(["git", "status", "--porcelain"], repo_root)
    if changed["exit_code"] != 0:
        return {"available": False, "in_scope": [], "out_of_scope": [], "detail": changed["stderr"]}

    allowed = f"crates/{piece}/"
    in_scope, out_of_scope = [], []
    for line in changed["stdout"].splitlines():
        path = line[3:].strip().strip('"')
        if not path:
            continue
        # Renames read as "old -> new"; judge the destination.
        path = path.split(" -> ")[-1]
        (in_scope if path.startswith(allowed) else out_of_scope).append(path)

    return {
        "available": True,
        "in_scope": sorted(in_scope),
        "out_of_scope": sorted(out_of_scope),
        "detail": "",
    }


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
    test_output = _run(["cargo", "test", "-p", piece], repo_root)
    return {
        "piece": piece,
        "issues": list(issues),
        "test_output": test_output,
        "clippy_output": _run(
            ["cargo", "clippy", "-p", piece, "--all-targets"], repo_root
        ),
        "issue_repro": _issue_repro(repo_root, piece, issues, test_output),
        "edit_scope": _edit_scope(repo_root, piece),
        "coverage": _coverage(repo_root),
        "binary_size": _binary_size(repo_root),
    }


def _render_issues(report):
    if not report:
        return "no issues assigned to this piece"
    lines = []
    for row in report:
        names = ", ".join(row["tests"]) if row["tests"] else "(none found)"
        lines.append(
            f"- #{row['issue']}: {row['status']} "
            f"[passed {row['passed']} / failed {row['failed']} / not run {row['not_run']}] "
            f"tests naming it: {names}"
        )
    return "\n".join(lines)


def _render_scope(scope):
    if not scope.get("available"):
        return f"scope unavailable: {scope.get('detail', 'no git status')}"
    out = scope.get("out_of_scope", [])
    lines = [f"files changed inside the piece: {len(scope.get('in_scope', []))}"]
    if out:
        lines.append(f"OUT OF SCOPE ({len(out)}) -- the builder may only edit its own crate:")
        lines.extend(f"  {p}" for p in out)
    else:
        lines.append("no out-of-scope edits")
    return "\n".join(lines)


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
            "## Assigned issues",
            "A test naming an issue passing is evidence that a test ran, not proof",
            "that the issue is closed. #284's closing test passed for months while",
            "asserting an enum CES has never used. Judge the claim, not the tick.",
            "",
            _render_issues(bundle.get("issue_repro", [])),
            "",
            "## Edit scope",
            _render_scope(bundle.get("edit_scope", {})),
            "",
            "## CES discovery coverage",
            f"v1: {cov['v1_methods']} methods (revision {cov['v1_revision']})",
            f"v1beta: {cov['v1beta_methods']} methods (revision {cov['v1beta_revision']})",
            "",
            "## Binary size",
            f"{bundle['binary_size']} bytes",
        ]
    )
