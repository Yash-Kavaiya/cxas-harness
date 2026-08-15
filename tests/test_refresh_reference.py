import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import pytest

from refresh_reference import canonicalize, newest, pinned_toml


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


def test_newest_picks_highest_revision_regardless_of_fetch_order():
    # The discovery endpoint is served from replicas that disagree: fetching
    # ?version=v1 repeatedly returns 20260730 and 20260806 in no stable order.
    docs = [
        {"revision": "20260730", "marker": "old"},
        {"revision": "20260806", "marker": "new"},
        {"revision": "20260730", "marker": "old"},
    ]
    assert newest(docs)["marker"] == "new"
    assert newest(list(reversed(docs)))["marker"] == "new"


def test_newest_handles_a_single_document():
    assert newest([{"revision": "20260101"}])["revision"] == "20260101"


def test_newest_rejects_an_empty_fetch_set():
    with pytest.raises(ValueError):
        newest([])


def test_newest_tolerates_a_document_missing_revision():
    docs = [{"no_revision": True}, {"revision": "20260806"}]
    assert newest(docs)["revision"] == "20260806"


def test_read_pinned_revisions_parses_both_entries(tmp_path):
    from refresh_reference import read_pinned_revisions

    p = tmp_path / "PINNED.toml"
    p.write_text(pinned_toml([
        {"version": "v1", "url": "u1", "revision": "20260806", "sha256": "a"},
        {"version": "v1beta", "url": "u2", "revision": "20260806", "sha256": "b"},
    ]), encoding="utf-8")
    assert read_pinned_revisions(p) == {"v1": "20260806", "v1beta": "20260806"}


def test_read_pinned_revisions_of_missing_file_is_empty(tmp_path):
    from refresh_reference import read_pinned_revisions

    assert read_pinned_revisions(tmp_path / "nope.toml") == {}
