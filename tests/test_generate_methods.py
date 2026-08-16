"""The generated CES method table.

The table is the one place where a mistake is invisible until a live request
fails, so the generator gets the same scrutiny the parser does: it must see
every method in a nested resource tree, refuse to emit an empty table, and
report staleness rather than silently rewriting.
"""

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import generate_methods
from generate_methods import ROOT, collect, methods_for, render


def test_collect_descends_into_nested_resources():
    # CES nests four deep -- projects.locations.apps.evaluations.results -- and
    # a walker that stops at the top level would silently miss most of the API.
    doc = {
        "apps": {
            "methods": {"list": {"id": "x.apps.list", "httpMethod": "GET", "path": "v1/apps"}},
            "resources": {
                "agents": {
                    "methods": {
                        "get": {"id": "x.apps.agents.get", "httpMethod": "GET", "path": "v1/{+name}"}
                    },
                    "resources": {
                        "tools": {
                            "methods": {
                                "get": {
                                    "id": "x.apps.agents.tools.get",
                                    "httpMethod": "GET",
                                    "path": "v1/{+name}",
                                }
                            }
                        }
                    },
                }
            },
        }
    }
    found = []
    collect(doc, found)
    assert sorted(item[0] for item in found) == [
        "x.apps.agents.get",
        "x.apps.agents.tools.get",
        "x.apps.list",
    ]


def test_collect_tolerates_a_resource_with_no_methods():
    # Container-only resources exist in discovery; they are not an error.
    found = []
    collect({"container": {"resources": {"leaf": {"methods": {}}}}}, found)
    assert found == []


def test_both_surfaces_are_read_from_the_vendored_reference():
    v1 = methods_for("v1")
    beta = methods_for("v1beta")
    assert len(v1) == 66
    assert len(beta) == 104


def test_every_method_has_a_verb_and_a_versioned_path():
    for version in ("v1", "v1beta"):
        for method_id, verb, path in methods_for(version):
            assert verb in {"GET", "POST", "PATCH", "PUT", "DELETE"}, method_id
            assert path.startswith(version + "/"), f"{method_id} -> {path}"
            assert "{" in path, f"{method_id} names no path parameter"


def test_ids_are_unique_within_a_surface():
    # A duplicate would make `method_spec` return whichever came first, which
    # is a coin flip rather than a lookup.
    for version in ("v1", "v1beta"):
        ids = [m[0] for m in methods_for(version)]
        assert len(ids) == len(set(ids))


def test_output_is_deterministic():
    # The table is checked in; an unstable ordering would show up as a diff on
    # every regeneration and train reviewers to skip it.
    assert render() == render()


def test_rendered_table_declares_every_method():
    rendered = render()
    total = len(methods_for("v1")) + len(methods_for("v1beta"))
    assert rendered.count("\n    v1(") + rendered.count("\n    beta(") == total
    assert "pub const METHODS: &[MethodSpec] = &[" in rendered
    assert rendered.endswith("];\n")


def test_evaluation_methods_are_only_emitted_on_the_beta_surface():
    # v1 declares none; emitting one there would build a URL that can only 404.
    assert not [m for m in methods_for("v1") if "valuation" in m[0]]
    assert [m for m in methods_for("v1beta") if "valuation" in m[0]]


def test_check_mode_passes_against_the_checked_in_table():
    result = subprocess.run(
        [sys.executable, "tools/generate_methods.py", "--check"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr


def test_check_mode_fails_when_the_table_drifts(tmp_path, monkeypatch):
    # The failure that actually happens: someone refreshes the reference and
    # forgets to regenerate, leaving the table describing a different API.
    stale = tmp_path / "method_table.rs"
    stale.write_text(
        render().replace("ces.projects.locations.apps.list", "ces.gone"), encoding="utf-8"
    )
    monkeypatch.setattr(generate_methods, "TARGET", stale)
    monkeypatch.setattr(sys, "argv", ["generate_methods.py", "--check"])
    assert generate_methods.main() == 1


def test_a_missing_table_is_reported_as_stale_not_as_a_crash(tmp_path, monkeypatch):
    monkeypatch.setattr(generate_methods, "TARGET", tmp_path / "absent.rs")
    monkeypatch.setattr(sys, "argv", ["generate_methods.py", "--check"])
    assert generate_methods.main() == 1
