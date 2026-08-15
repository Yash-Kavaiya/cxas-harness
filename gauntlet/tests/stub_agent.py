#!/usr/bin/env python3
"""Canned-verdict agent so the loop is testable without invoking a real model.

Reads a prompt on stdin, ignores it, and emits a verdict controlled by
GAUNTLET_STUB_MODE: 'pass', 'fail', or 'garbage'.
"""
import os
import sys

sys.stdin.read()
mode = os.environ.get("GAUNTLET_STUB_MODE", "pass")

if mode == "pass":
    print('{"score": 95, "verdict": "PASS", "biggest_gap": "none"}')
elif mode == "fail":
    print('{"score": 40, "verdict": "FAIL", "biggest_gap": "enum drift on EvaluationRunState"}')
else:
    print("this is not json at all")
