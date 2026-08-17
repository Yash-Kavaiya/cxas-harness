"""Measure actual rendered widths of the demo's text at given font sizes."""
from manim import Text, config, WHITE, NORMAL, BOLD

SANS = "Segoe UI"
MONO = "Consolas"

config.background_color = WHITE

FRAME_W = 14.22  # manim default frame width


def w(text, size, font=SANS, weight=NORMAL):
    t = Text(text, font=font, font_size=size, weight=weight)
    return t.width


cases = [
    # (label, text, size, font, max_ok)
    ("title subtitle", "Machine-first CLI & library harness for CX Agent Studio (CES)", 36, SANS, FRAME_W - 1.5),
    ("pitch label1", "CES methods addressable", 25, SANS, 4.0),
    ("pitch label4", 'defaults: location never \u201cglobal\u201d', 25, SANS, 4.0),
    ("defect mono", "PENDING / SUCCEEDED / FAILED", 26, MONO, 5.0),
    ("arch ref", "reference/ces/", 24, SANS, 2.8),
    ("arch core sub", "REST table \u00b7 auth \u00b7 streaming", 21, SANS, 3.2),
    ("arch ces sub", "ADC token, cached + refreshed", 19, SANS, 3.2),
    ("coverage line1", "addressable  v1=66/66  v1beta=104/104", 30, MONO, FRAME_W - 1.5),
    ("coverage line2", "total=170/170  modelled=37/170", 30, MONO, FRAME_W - 1.5),
    ("install line", "$ python -m pytest tests gauntlet/tests  # 58 tests", 20, MONO, 7.8),
    ("cli json", '{ "ok": true, "count": 4, "methods": [...] }', 20, MONO, 7.8),
    ("auth cred", "GOOGLE_APPLICATION_CREDENTIALS", 22, MONO, 3.9),
    ("auth msg", '{ "message": "hi, how can I help?" }', 20, MONO, 5.4),
    ("gauntlet stops", "max_rounds \u00b7 max_agent_calls \u00b7 rc_coverage_min", 24, SANS, 7.4),
    ("gauntlet note1", "Blindness is enforced by a test:", 22, SANS, FRAME_W - 1.5),
    ("gauntlet note2", "source reaching a critic fails the suite.", 22, SANS, FRAME_W - 1.5),
    ("pitch note1", "Addressable: generated from Google's own discovery documents.", 24, SANS, FRAME_W - 1.5),
    ("cli cap1", "JSON envelope \u00b7 exit 0 = ok \u00b7 1 = runtime \u00b7 2 = usage", 22, SANS, FRAME_W - 1.5),
    ("title", "cxas-harness", 96, SANS, FRAME_W - 1.5),
    ("header gauntlet", "The Gauntlet Loop  (repo tooling, never shipped)", 30, SANS, FRAME_W - 3),
    ("header auth", "Auth & streaming", 30, SANS, FRAME_W - 3),
    ("defect note1", "The fix: Google's own discovery documents, pinned + sha256,", 30, SANS, FRAME_W - 1.5),
    ("defect num", "78 passing tests could not see it.", 38, SANS, FRAME_W - 1.5),
    ("arch note1", "Nothing in the workspace decides what CES is \u2014", 28, SANS, FRAME_W - 1.5),
    ("coverage note1", "A pass/fail threshold can be satisfied by deleting the metric.", 27, SANS, FRAME_W - 1.5),
    ("gauntlet stops3", "reaching a cap is a FAIL, never a pass", 22, SANS, 7.8),
    ("auth note1", "A stream that ends mid-message is an error \u2014 a dropped connection", 22, SANS, FRAME_W - 1.5),
    ("outro link3", "Apache-2.0 \u00b7 an independent rewrite, not an official Google product", 24, SANS, FRAME_W - 1.5),
    ("outro title", "Build for CX Agent Studio, in Rust", 54, SANS, FRAME_W - 1.5),
    ("auth cache", "token cached \u00b7 refreshed a minute before expiry", 22, SANS, FRAME_W - 1.5),
    ("pitch note2", "Modelled: this workspace has an opinion about the resource.", 24, SANS, FRAME_W - 1.5),
    ("coverage line mono30", "addressable  v1=66/66  v1beta=104/104", 30, MONO, FRAME_W - 1.5),
    ("cli line mono20", "{ \"ok\": true, \"count\": 4, \"methods\": [...] }", 20, MONO, 7.8),
    ("install cap1", "No GCP project, no credentials, no network", 22, SANS, FRAME_W - 1.5),
]


for label, text, size, font, max_ok in cases:
    wd = w(text, size, font)
    flag = "OK " if wd <= max_ok else "OVER"
    print(f"{flag} {label:22s} size={size:3d} width={wd:6.2f} (max {max_ok:.2f})")
