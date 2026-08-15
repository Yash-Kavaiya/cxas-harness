import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

from refresh_reference import canonicalize, pinned_toml


def test_canonicalize_sorts_keys_and_ends_with_newline():
    out = canonicalize({"b": 1, "a": {"d": 2, "c": 3}})
    assert out.endswith("\n")
    assert out == '{\n  "a": {\n    "c": 3,\n    "d": 2\n  },\n  "b": 1\n}\n'


def test_canonicalize_is_idempotent():
    obj = {"z": [3, 1, 2], "a": "x"}
    once = canonicalize(obj)
    twice = canonicalize(json.loads(once))
    assert once == twice


def test_canonicalize_preserves_non_ascii():
    assert "é" in canonicalize({"k": "café"})


def test_pinned_toml_records_url_revision_and_sha():
    out = pinned_toml([
        {"version": "v1", "url": "https://example/v1", "revision": "20260730", "sha256": "abc"},
    ])
    assert '[[reference]]' in out
    assert 'version = "v1"' in out
    assert 'revision = "20260730"' in out
    assert 'sha256 = "abc"' in out
