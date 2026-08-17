"""
cxas-harness demo — Manim (community edition), light theme, Google four colors.

Render:
    manim -qh scenes.py TitleScene ... OutroScene
or all scenes then concatenate with ffmpeg.

YouTube format: 16:9, 30 fps, 1920x1080.
"""
from manim import *

# ---------------------------------------------------------------- palette
GOOGLE_BLUE = "#4285F4"
GOOGLE_RED = "#EA4335"
GOOGLE_YELLOW = "#FBBC05"
GOOGLE_GREEN = "#34A853"
INK = "#202124"
SUBTLE = "#5F6368"
PANEL = "#F8F9FA"
LINE = "#DADCE0"
ACCENT = GOOGLE_BLUE

SANS = "Segoe UI"
MONO = "Consolas"

FRAME_W = config.frame_width

config.background_color = WHITE
config.frame_rate = 30
config.pixel_width = 1920
config.pixel_height = 1080

# Global pacing multiplier: every animation run_time and wait is scaled by
# this, so the whole video gets slower without editing each call.
PACING = 1.8


class PacedScene(Scene):
    """Scene whose animation and hold times are scaled by PACING."""

    def play(self, *args, **kwargs):
        kwargs["run_time"] = kwargs.pop("run_time", 1.0) * PACING
        super().play(*args, **kwargs)

    def wait(self, duration=1.0, **kwargs):
        super().wait(duration * PACING, **kwargs)


def gtext(text, size=34, color=INK, font=SANS, weight=NORMAL):
    return Text(text, font=font, font_size=size, color=color, weight=weight)


def mono(text, size=30, color=INK, weight=NORMAL):
    return Text(text, font=MONO, font_size=size, color=color, weight=weight)


def fit(t, max_w=None, max_h=None):
    """Scale a text mobject down (uniformly, in place) so it fits max_w/max_h.

    Call BEFORE positioning: scaling moves the center, so move_to/next_to
    must come after. Guarantees text never spills out of a box or off-frame.
    """
    if max_w and t.width > max_w:
        t.scale_to_fit_width(max_w)
    if max_h and t.height > max_h:
        t.scale_to_fit_height(max_h)
    return t


def panel_box(width, height, color=GOOGLE_BLUE, fill=PANEL, stroke=LINE):
    return RoundedRectangle(
        corner_radius=0.08,
        width=width,
        height=height,
        stroke_color=stroke,
        stroke_width=2,
        fill_color=fill,
        fill_opacity=1.0,
    )


def footer(scene, note="github.com/Yash-Kavaiya/cxas-harness"):
    f = fit(gtext(note, size=22, color=SUBTLE), max_w=FRAME_W - 2)
    f.to_corner(DOWN + RIGHT, buff=0.45)
    scene.add(f)
    return f


def header(scene, title, color=GOOGLE_BLUE):
    bar = Rectangle(width=config.frame_width - 2.2, height=0.09, fill_color=color,
                    fill_opacity=1.0, stroke_width=0)
    bar.to_edge(UP, buff=0.55)
    t = fit(gtext(title, size=30, color=SUBTLE), max_w=FRAME_W - 3)
    t.next_to(bar, DOWN, buff=0.16)
    scene.add(bar, t)
    return bar, t


def google_bars(center_y):
    bars = VGroup()
    widths = [0.34, 0.22, 0.18, 0.26]
    colors = [GOOGLE_BLUE, GOOGLE_RED, GOOGLE_YELLOW, GOOGLE_GREEN]
    x = -1.15
    for w, c in zip(widths, colors):
        b = Rectangle(width=w, height=0.14, fill_color=c, fill_opacity=1.0, stroke_width=0)
        b.move_to([x, center_y, 0])
        bars.add(b)
        x += w + 0.06
    return bars


# ---------------------------------------------------------------- scenes
class TitleScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)

        bars = google_bars(-0.75)

        title = fit(gtext("cxas-harness", size=96, color=INK, weight=BOLD), max_w=FRAME_W - 2)
        subtitle = fit(gtext(
            "Machine-first CLI & library harness for CX Agent Studio (CES)",
            size=33, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        subtitle.next_to(title, DOWN, buff=0.3)
        bars.next_to(subtitle, DOWN, buff=0.5)

        badge1 = gtext("Rust  \u00b7  10 crates", size=26, color=GOOGLE_BLUE)
        badge2 = gtext("JSON by default", size=26, color=GOOGLE_GREEN)
        badge3 = gtext("exit 0 / 1 / 2", size=26, color=GOOGLE_RED)
        badges = VGroup(badge1, badge2, badge3).arrange(RIGHT, buff=0.7)
        badges.next_to(bars, DOWN, buff=0.55)

        self.play(FadeIn(title, shift=UP * 0.3), run_time=1.0)
        self.play(FadeIn(subtitle), run_time=0.8)
        self.play(*[GrowFromEdge(b, LEFT if i % 2 == 0 else RIGHT)
                    for i, b in enumerate(bars)], run_time=0.9)
        self.play(FadeIn(badges), run_time=0.8)
        self.wait(1.6)
        footer(self)
        self.wait(1.2)


class PitchScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "What it is")

        cards = VGroup()
        rows = [
            ("170 / 170", "CES methods addressable", GOOGLE_BLUE),
            ("37", "hand-modelled with real types", GOOGLE_RED),
            ("66 + 104", "v1 + v1beta surfaces", GOOGLE_YELLOW),
            ("0", "defaults: location never \u201cglobal\u201d", GOOGLE_GREEN),
        ]
        for num, label, color in rows:
            box = RoundedRectangle(corner_radius=0.12, width=5.0, height=1.35,
                                   stroke_color=color, stroke_width=3,
                                   fill_color=PANEL, fill_opacity=1.0)
            n = fit(gtext(num, size=42, color=color, weight=BOLD), max_w=4.4, max_h=0.6)
            n.move_to(box.get_center() + UP * 0.28)
            l = fit(gtext(label, size=22, color=INK), max_w=4.4, max_h=0.45)
            l.next_to(n, DOWN, buff=0.12)
            g = VGroup(box, n, l)
            cards.add(g)
        cards.arrange_in_grid(rows=2, cols=2, buff=0.55)
        cards.move_to(UP * 0.25)

        self.play(*[FadeIn(c, shift=UP * 0.2) for c in cards], run_time=1.1)
        self.wait(1.0)

        note = fit(gtext(
            "Addressable: generated from Google's own discovery documents.\n"
            "Modelled: this workspace has an opinion about the resource.",
            size=24, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        note.next_to(cards, DOWN, buff=0.7)
        self.play(FadeIn(note), run_time=0.9)
        self.wait(1.4)
        footer(self)
        self.wait(1.0)


class DefectScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Why the benchmark exists")

        title = fit(gtext("A green suite missed a real bug", size=44, color=INK, weight=BOLD),
                    max_w=FRAME_W - 2)
        title.move_to(UP * 2.3)
        self.play(FadeIn(title, shift=DOWN * 0.3), run_time=0.8)

        wrong = panel_box(5.8, 1.5, color=GOOGLE_RED)
        w_l = fit(gtext("declared", size=22, color=SUBTLE), max_w=5.2)
        w_l2 = fit(mono("PENDING / SUCCEEDED / FAILED", size=24, color=GOOGLE_RED, weight=BOLD),
                   max_w=5.2, max_h=0.55)
        w_l.move_to(wrong.get_center() + UP * 0.35)
        w_l2.next_to(w_l, DOWN, buff=0.12)
        wrong_g = VGroup(wrong, w_l, w_l2).move_to(LEFT * 3.4)

        right = panel_box(5.8, 1.5, color=GOOGLE_GREEN)
        r_l = fit(gtext("CES declares", size=22, color=SUBTLE), max_w=5.2)
        r_l2 = fit(mono("QUEUED / COMPLETED / ERROR", size=24, color=GOOGLE_GREEN, weight=BOLD),
                   max_w=5.2, max_h=0.55)
        r_l.move_to(right.get_center() + UP * 0.35)
        r_l2.next_to(r_l, DOWN, buff=0.12)
        right_g = VGroup(right, r_l, r_l2).move_to(RIGHT * 3.4)

        x = gtext("\u2715", size=64, color=GOOGLE_RED, weight=BOLD)
        x.move_to(UP * 0.0)

        self.play(FadeIn(wrong_g), run_time=0.6)
        self.play(FadeIn(x), run_time=0.3)
        self.play(FadeIn(right_g), run_time=0.6)
        self.wait(0.9)

        num = fit(gtext("78 passing tests could not see it.", size=38, color=INK, weight=BOLD),
                  max_w=FRAME_W - 2)
        num.move_to(DOWN * 0.7)
        self.play(FadeIn(num, shift=UP * 0.2), run_time=0.8)
        self.wait(0.8)

        note = fit(gtext(
            "The fix: Google's own discovery documents, pinned + sha256,\n"
            "as the one authority \u2014 and a parity contract that can fail.",
            size=30, color=GOOGLE_BLUE, weight=BOLD,
        ), max_w=FRAME_W - 2)
        note.next_to(num, DOWN, buff=0.5)
        self.play(FadeIn(note), run_time=0.9)
        self.wait(2.0)
        footer(self)
        self.wait(1.0)


class ArchitectureScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Ground truth in, requests out")

        # --- ground truth column (left)
        ref = panel_box(3.4, 1.15, color=GOOGLE_BLUE)
        ref_l = fit(gtext("reference/ces/", size=24, color=GOOGLE_BLUE, weight=BOLD), max_w=2.8)
        ref_l2 = fit(gtext("pinned + sha256", size=20, color=SUBTLE), max_w=2.8)
        ref_l.move_to(ref.get_center() + UP * 0.22)
        ref_l2.next_to(ref_l, DOWN, buff=0.08)
        ref_g = VGroup(ref, ref_l, ref_l2).move_to(LEFT * 4.6 + UP * 1.7)

        gen = panel_box(3.4, 1.05, color=GOOGLE_GREEN)
        gen_l = fit(gtext("generate_methods.py", size=22, color=GOOGLE_GREEN, weight=BOLD), max_w=2.8)
        gen_l.move_to(gen.get_center())
        gen_g = VGroup(gen, gen_l).move_to(LEFT * 4.6 + UP * 0.35)

        table = panel_box(3.4, 1.25, color=GOOGLE_RED)
        table_l = fit(gtext("METHODS table", size=24, color=GOOGLE_RED, weight=BOLD), max_w=2.8)
        table_l2 = fit(gtext("170 specs, generated", size=20, color=SUBTLE), max_w=2.8)
        table_l.move_to(table.get_center() + UP * 0.22)
        table_l2.next_to(table_l, DOWN, buff=0.08)
        table_g = VGroup(table, table_l, table_l2).move_to(LEFT * 4.6 + DOWN * 1.05)

        ar1 = Arrow(ref_g.get_bottom(), gen_g.get_top(), color=GOOGLE_BLUE, stroke_width=4)
        ar2 = Arrow(gen_g.get_bottom(), table_g.get_top(), color=GOOGLE_GREEN, stroke_width=4)

        # --- core column (middle)
        core = panel_box(3.9, 1.5, color=GOOGLE_BLUE)
        core_l = fit(gtext("cxas-core", size=26, color=GOOGLE_BLUE, weight=BOLD), max_w=3.3)
        core_l2 = fit(gtext("REST table \u00b7 auth \u00b7 streaming", size=20, color=INK), max_w=3.3)
        core_l.move_to(core.get_center() + UP * 0.3)
        core_l2.next_to(core_l, DOWN, buff=0.1)
        core_g = VGroup(core, core_l, core_l2).move_to(UP * 0.1)

        parity = panel_box(3.9, 1.05, color=GOOGLE_RED)
        parity_l = fit(gtext("cxas-parity", size=24, color=GOOGLE_RED, weight=BOLD), max_w=3.3)
        parity_l2 = fit(gtext("the contract that can fail", size=20, color=SUBTLE), max_w=3.3)
        parity_l.move_to(parity.get_center() + UP * 0.2)
        parity_l2.next_to(parity_l, DOWN, buff=0.08)
        parity_g = VGroup(parity, parity_l, parity_l2).move_to(DOWN * 1.6)

        ar3 = Arrow(table_g.get_right(), core_g.get_left(), color=GOOGLE_RED, stroke_width=4)
        ar4 = DoubleArrow(core_g.get_bottom(), parity_g.get_top(),
                          color=SUBTLE, stroke_width=3)
        ar5 = DoubleArrow(table_g.get_right(), parity_g.get_left(),
                          color=GOOGLE_GREEN, stroke_width=3)

        # --- CES column (right)
        ces = panel_box(3.9, 1.7, color=GOOGLE_GREEN)
        ces_l = fit(gtext("CES REST API", size=26, color=GOOGLE_GREEN, weight=BOLD), max_w=3.3)
        ces_l2 = fit(gtext("v1 + v1beta", size=22, color=INK), max_w=3.3)
        ces_l3 = fit(gtext("ADC token, cached + refreshed", size=19, color=SUBTLE), max_w=3.3)
        ces_l.move_to(ces.get_center() + UP * 0.42)
        ces_l2.next_to(ces_l, DOWN, buff=0.1)
        ces_l3.next_to(ces_l2, DOWN, buff=0.1)
        ces_g = VGroup(ces, ces_l, ces_l2, ces_l3).move_to(RIGHT * 4.7 + UP * 0.1)

        ar6 = Arrow(core_g.get_right(), ces_g.get_left(), color=GOOGLE_BLUE, stroke_width=4)
        ar7 = Arrow(parity_g.get_right(), ces_g.get_bottom(), color=GOOGLE_GREEN, stroke_width=3)

        # animate
        self.play(FadeIn(ref_g), run_time=0.6)
        self.play(FadeIn(gen_g), GrowArrow(ar1), run_time=0.6)
        self.play(FadeIn(table_g), GrowArrow(ar2), run_time=0.6)
        self.play(FadeIn(core_g), GrowArrow(ar3), run_time=0.6)
        self.play(FadeIn(parity_g), GrowArrow(ar4), GrowArrow(ar5), run_time=0.7)
        self.play(FadeIn(ces_g), GrowArrow(ar6), GrowArrow(ar7), run_time=0.7)
        self.wait(1.2)

        note = fit(gtext(
            "Nothing in the workspace decides what CES is \u2014\n"
            "it reads that from Google's machine-readable description.",
            size=28, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        note.next_to(parity_g, DOWN, buff=0.55)
        self.play(FadeIn(note), run_time=0.9)
        self.wait(1.6)
        footer(self)
        self.wait(1.0)


class CoverageScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Two numbers, honestly reported")

        big = fit(gtext("CES-COVERAGE", size=58, color=INK, weight=BOLD), max_w=FRAME_W - 2)
        big.move_to(UP * 2.4)
        line1 = fit(mono("addressable  v1=66/66  v1beta=104/104", size=30, color=GOOGLE_BLUE),
                    max_w=FRAME_W - 2)
        line2 = fit(mono("total=170/170  modelled=37/170", size=30, color=GOOGLE_BLUE),
                    max_w=FRAME_W - 2)
        lines = VGroup(line1, line2).arrange(DOWN, buff=0.15)
        lines.next_to(big, DOWN, buff=0.4)

        self.play(FadeIn(big, shift=DOWN * 0.3), run_time=0.8)
        self.play(Write(line1), run_time=1.0)
        self.play(Write(line2), run_time=1.0)
        self.wait(0.8)

        # per-version bars
        def bar_seg(label, num, color, width):
            b = RoundedRectangle(corner_radius=0.1, width=width, height=0.9,
                                 stroke_color=color, stroke_width=3,
                                 fill_color=color, fill_opacity=0.15)
            t = fit(gtext(label, size=24, color=INK), max_w=width - 0.5)
            n = fit(gtext(num, size=30, color=color, weight=BOLD), max_w=width - 0.5)
            t.move_to(b.get_center() + UP * 0.25)
            n.move_to(b.get_center() + DOWN * 0.3)
            return VGroup(b, t, n)

        v1 = bar_seg("v1", "66", GOOGLE_BLUE, 3.4)
        vb = bar_seg("v1beta", "104", GOOGLE_GREEN, 3.4)
        mod = bar_seg("modelled", "37", GOOGLE_RED, 3.4)
        group = VGroup(v1, vb, mod).arrange(RIGHT, buff=0.7)
        group.next_to(lines, DOWN, buff=0.7)

        self.play(*[FadeIn(g, shift=UP * 0.15) for g in group], run_time=0.9)
        self.wait(1.0)

        note = fit(gtext(
            "A pass/fail threshold can be satisfied by deleting the metric.\n"
            "A printed number cannot.",
            size=27, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        note.next_to(group, DOWN, buff=0.6)
        self.play(FadeIn(note), run_time=0.8)
        self.wait(1.6)
        footer(self)
        self.wait(1.0)


class InstallScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Install & verify")

        term = RoundedRectangle(corner_radius=0.12, width=8.6, height=4.6,
                                stroke_color=LINE, stroke_width=3,
                                fill_color="#F1F3F4", fill_opacity=1.0)
        term.move_to(UP * 0.2)

        lines = [
            ("$ cargo build --release -p cxas-cli", GOOGLE_GREEN),
            ("$ cargo test --workspace      # 221 tests", GOOGLE_BLUE),
            ("$ cargo clippy --all-targets  # clean", GOOGLE_BLUE),
            ("$ python -m pytest tests gauntlet/tests  # 58 tests", GOOGLE_BLUE),
            ("", None),
            ("$ cxas --help   # 36 commands, JSON out", GOOGLE_GREEN),
        ]
        text_lines = VGroup()
        y = term.get_top() + DOWN * 0.85
        for txt, color in lines:
            if txt == "":
                text_lines.add(VGroup())
                continue
            t = fit(mono(txt, size=20, color=color if color else INK), max_w=7.8)
            t.move_to([term.get_left()[0] + 0.4, y[1], 0], aligned_edge=LEFT)
            text_lines.add(t)
            y = y + DOWN * 0.55

        cap1 = fit(gtext("No GCP project, no credentials, no network", size=22, color=SUBTLE),
                   max_w=FRAME_W - 2)
        cap2 = fit(gtext("fully offline & deterministic", size=22, color=GOOGLE_GREEN, weight=BOLD),
                   max_w=FRAME_W - 2)
        cap = VGroup(cap1, cap2).arrange(DOWN, buff=0.2)
        cap.next_to(term.get_bottom(), DOWN, buff=0.7)

        self.play(FadeIn(term), run_time=0.6)
        for t in text_lines:
            if len(t) == 0:
                self.wait(0.15)
                continue
            self.play(Write(t), run_time=0.45)
            self.wait(0.1)
        self.play(FadeIn(cap), run_time=0.7)
        self.wait(1.4)
        footer(self)
        self.wait(1.0)


class CliScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Machine-first CLI")

        term = RoundedRectangle(corner_radius=0.12, width=8.6, height=5.0,
                                stroke_color=LINE, stroke_width=3,
                                fill_color="#F1F3F4", fill_opacity=1.0)
        term.move_to(UP * 0.15)

        title_dots = VGroup(
            Dot(radius=0.09, color=GOOGLE_RED), Dot(radius=0.09, color=GOOGLE_YELLOW),
            Dot(radius=0.09, color=GOOGLE_GREEN),
        ).arrange(RIGHT, buff=0.14)
        title_dots.move_to(term.get_top() + DOWN * 0.35 + LEFT * 3.6)

        lines = [
            ("$ cxas init --app-dir ./my-app", GOOGLE_GREEN),
            ('{ "ok": true, "command": "init" }', GOOGLE_BLUE),
            ("", None),
            ("$ cxas lint --app-dir ./my-app", GOOGLE_GREEN),
            ('{ "ok": true, "error_count": 0 }', GOOGLE_BLUE),
            ("", None),
            ("$ cxas api list --filter evaluationRuns", GOOGLE_GREEN),
            ('{ "ok": true, "count": 4, "methods": [...] }', GOOGLE_BLUE),
            ("", None),
            ("$ cxas api stream ...streamRunSession", GOOGLE_GREEN),
            ('{ "ok": true, "message": "..." }', GOOGLE_BLUE),
        ]
        text_lines = VGroup()
        y = term.get_top() + DOWN * 0.75
        for i, (txt, color) in enumerate(lines):
            if txt == "":
                text_lines.add(VGroup())  # spacer
                continue
            t = fit(mono(txt, size=20, color=color if color else INK), max_w=7.8)
            t.move_to([term.get_left()[0] + 0.4, y[1], 0], aligned_edge=LEFT)
            text_lines.add(t)
            y = y + DOWN * 0.42

        cap1 = fit(gtext("JSON envelope \u00b7 exit 0 = ok \u00b7 1 = runtime \u00b7 2 = usage",
                         size=22, color=SUBTLE), max_w=FRAME_W - 2)
        cap2 = fit(gtext("--no-input on \u00b7 --format human for people",
                         size=22, color=GOOGLE_BLUE, weight=BOLD), max_w=FRAME_W - 2)
        cap = VGroup(cap1, cap2).arrange(DOWN, buff=0.2)
        cap.next_to(term.get_bottom(), DOWN, buff=0.6)

        self.play(FadeIn(term), FadeIn(title_dots), run_time=0.7)
        for t in text_lines:
            if len(t) == 0:
                self.wait(0.15)
                continue
            self.play(Write(t), run_time=0.45)
            self.wait(0.12)
        self.play(FadeIn(cap), run_time=0.8)
        self.wait(1.4)
        footer(self)
        self.wait(1.0)


class AuthStreamingScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "Auth & streaming")

        # ---- left: credential precedence chain
        chain_title = gtext("credential precedence", size=26, color=SUBTLE, weight=BOLD)
        chain_title.move_to(LEFT * 4.55 + UP * 1.95)
        self.play(FadeIn(chain_title), run_time=0.5)

        creds = [
            ("--oauth-token", GOOGLE_BLUE),
            ("CXAS_ACCESS_TOKEN", GOOGLE_RED),
            ("GOOGLE_APPLICATION_CREDENTIALS", GOOGLE_YELLOW),
            ("ADC well-known file", GOOGLE_GREEN),
            ("metadata server", GOOGLE_BLUE),
            ("gcloud auth print-access-token", GOOGLE_RED),
        ]
        chain = VGroup()
        y = UP * 1.45
        for i, (label, color) in enumerate(creds):
            box = RoundedRectangle(corner_radius=0.1, width=5.2, height=0.5,
                                   stroke_color=color, stroke_width=2.5,
                                   fill_color=PANEL, fill_opacity=1.0)
            t = fit(gtext(label, size=22, color=color, weight=BOLD, font=MONO), max_w=4.6)
            box.move_to(LEFT * 4.55 + y)
            t.move_to(box.get_center())
            chain.add(VGroup(box, t))
            y = y + DOWN * 0.66
        chain_note = gtext("first usable wins", size=21, color=SUBTLE)
        chain_note.next_to(chain, DOWN, buff=0.12)

        # arrows between chain boxes
        arrows = VGroup()
        for a, b in zip(chain, chain[1:]):
            arr = Arrow(a.get_bottom(), b.get_top(), color=LINE, stroke_width=3, buff=0.02)
            arrows.add(arr)

        self.play(FadeIn(chain[0]), run_time=0.5)
        for i in range(1, len(chain)):
            self.play(FadeIn(chain[i]), GrowArrow(arrows[i - 1]), run_time=0.5)
        self.play(FadeIn(chain_note), run_time=0.6)

        cache = fit(gtext("token cached \u00b7 refreshed a minute before expiry", size=22,
                          color=GOOGLE_GREEN, weight=BOLD), max_w=FRAME_W - 2)
        cache.next_to(chain_note, DOWN, buff=0.18)
        self.play(FadeIn(cache), run_time=0.7)

        # ---- right: streaming terminal
        term = RoundedRectangle(corner_radius=0.12, width=6.0, height=4.9,
                                stroke_color=LINE, stroke_width=3,
                                fill_color="#F1F3F4", fill_opacity=1.0)
        term.move_to(RIGHT * 4.1 + DOWN * 0.15)
        term_title = gtext("streamRunSession", size=24, color=GOOGLE_BLUE, weight=BOLD)
        term_title.next_to(term.get_top(), DOWN, buff=0.22)

        term_lines = [
            ("$ cxas api stream ...streamRunSession", GOOGLE_GREEN),
            ("", None),
            ('{ "query": "hello" }', GOOGLE_BLUE),
            ('{ "message": "hi, how can I help?" }', GOOGLE_BLUE),
            ('{ "message": "..." }  <- one by one', GOOGLE_BLUE),
        ]
        tlines = VGroup()
        ty = term.get_top() + DOWN * 0.85
        for txt, color in term_lines:
            if txt == "":
                tlines.add(VGroup())
                continue
            t = fit(mono(txt, size=20, color=color if color else INK), max_w=5.4)
            t.move_to([term.get_left()[0] + 0.3, ty[1], 0], aligned_edge=LEFT)
            tlines.add(t)
            ty = ty + DOWN * 0.62

        self.play(FadeIn(term), FadeIn(term_title), run_time=0.6)
        for t in tlines:
            if len(t) == 0:
                self.wait(0.3)
                continue
            self.play(Write(t), run_time=0.5)
            self.wait(0.2)

        # ---- bottom note
        note = fit(gtext(
            "A stream that ends mid-message is an error \u2014 a dropped connection\n"
            "and a finished conversation are otherwise indistinguishable.",
            size=22, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        note.move_to(DOWN * 3.35)
        self.play(FadeIn(note, shift=UP * 0.2), run_time=0.8)
        self.wait(2.0)
        footer(self)
        self.wait(1.5)


class GauntletScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)
        header(self, "The Gauntlet Loop  (repo tooling, never shipped)")

        # three nodes in a loop
        builder = panel_box(3.2, 1.6, color=GOOGLE_BLUE)
        b_l = fit(gtext("Builder", size=28, color=GOOGLE_BLUE, weight=BOLD), max_w=2.6)
        b_l2 = fit(gtext("edits one crate", size=21, color=INK), max_w=2.6)
        b_l.move_to(builder.get_center() + UP * 0.3)
        b_l2.next_to(b_l, DOWN, buff=0.1)
        builder_g = VGroup(builder, b_l, b_l2).move_to(LEFT * 4.25 + UP * 1.35)

        evidence = panel_box(3.2, 1.6, color=GOOGLE_GREEN)
        e_l = fit(gtext("evidence.py", size=28, color=GOOGLE_GREEN, weight=BOLD), max_w=2.6)
        e_l2 = fit(gtext("deterministic code", size=21, color=INK), max_w=2.6)
        e_l.move_to(evidence.get_center() + UP * 0.3)
        e_l2.next_to(e_l, DOWN, buff=0.1)
        evidence_g = VGroup(evidence, e_l, e_l2).move_to(RIGHT * 0.0 + UP * 1.35)

        critic = panel_box(3.2, 1.6, color=GOOGLE_RED)
        c_l = fit(gtext("Blind critic", size=28, color=GOOGLE_RED, weight=BOLD), max_w=2.6)
        c_l2 = fit(gtext("sees evidence only", size=21, color=INK), max_w=2.6)
        c_l.move_to(critic.get_center() + UP * 0.3)
        c_l2.next_to(c_l, DOWN, buff=0.1)
        critic_g = VGroup(critic, c_l, c_l2).move_to(RIGHT * 4.25 + UP * 1.35)

        # loop arrows (straight)
        a1 = Arrow(builder_g.get_right(), evidence_g.get_left(),
                   color=GOOGLE_BLUE, stroke_width=4, buff=0.18)
        a2 = Arrow(evidence_g.get_right(), critic_g.get_left(),
                   color=GOOGLE_GREEN, stroke_width=4, buff=0.18)
        ret = Line(critic_g.get_bottom() + DOWN * 0.55, builder_g.get_bottom() + DOWN * 0.55,
                   color=GOOGLE_RED, stroke_width=4)
        tip = Triangle(color=GOOGLE_RED, fill_color=GOOGLE_RED, fill_opacity=1.0,
                       stroke_width=0).scale(0.16).rotate(PI / 2)
        tip.move_to(builder_g.get_bottom() + DOWN * 0.55 + RIGHT * 0.12)
        a3 = VGroup(ret, tip)

        # flow captions sit in the gap between the row and the return arrow,
        # not on top of the boxes
        l1 = fit(gtext("commits first", size=20, color=SUBTLE), max_w=2.2)
        l1.move_to([-2.2, 0.4, 0])
        l2 = fit(gtext("blind: evidence only", size=20, color=SUBTLE), max_w=2.6)
        l2.move_to([2.35, 0.4, 0])
        l3 = fit(gtext("names ONE gap", size=20, color=SUBTLE), max_w=3.0)
        l3.next_to(ret, DOWN, buff=0.1)

        stops = panel_box(8.4, 2.0, color=GOOGLE_YELLOW)
        stops_l = fit(gtext("Stop conditions, enforced in code", size=26, color="#B47D00", weight=BOLD),
                      max_w=7.8)
        stops_l2 = fit(gtext("max_rounds \u00b7 max_agent_calls \u00b7 rc_coverage_min", size=24, color=INK),
                       max_w=7.8)
        stops_l3 = fit(gtext("reaching a cap is a FAIL, never a pass", size=22, color=SUBTLE), max_w=7.8)
        stops_l.move_to(stops.get_center() + UP * 0.45)
        stops_l2.next_to(stops_l, DOWN, buff=0.12)
        stops_l3.next_to(stops_l2, DOWN, buff=0.12)
        stops_g = VGroup(stops, stops_l, stops_l2, stops_l3).move_to(DOWN * 1.5)

        self.play(FadeIn(builder_g), run_time=0.6)
        self.play(FadeIn(evidence_g), GrowArrow(a1), run_time=0.6)
        self.play(FadeIn(critic_g), GrowArrow(a2), run_time=0.6)
        self.play(Create(ret), FadeIn(tip), FadeIn(l1), FadeIn(l2), FadeIn(l3), run_time=0.6)
        self.play(FadeIn(stops_g, shift=UP * 0.2), run_time=0.7)
        self.wait(1.4)

        note = fit(gtext(
            "Blindness is enforced by a test:\n"
            "source reaching a critic fails the suite.",
            size=22, color=SUBTLE,
        ), max_w=FRAME_W - 2)
        note.next_to(stops_g, DOWN, buff=0.35)
        self.play(FadeIn(note), run_time=0.8)
        self.wait(1.6)
        footer(self)
        self.wait(1.0)


class OutroScene(PacedScene):
    def construct(self):
        bg = Rectangle(width=config.frame_width, height=config.frame_height,
                       fill_color=WHITE, fill_opacity=1.0, stroke_width=0)
        self.add(bg)

        bars = google_bars(0.4)

        title = fit(gtext("Build for CX Agent Studio, in Rust", size=54, color=INK, weight=BOLD),
                    max_w=FRAME_W - 2)
        title.move_to(UP * 1.4)
        bars.next_to(title, DOWN, buff=0.5)

        links = VGroup(
            fit(gtext("github.com/Yash-Kavaiya/cxas-harness", size=32, color=GOOGLE_BLUE, weight=BOLD),
                max_w=FRAME_W - 2),
            fit(gtext("docs: yash-kavaiya.github.io/cxas-harness", size=28, color=SUBTLE),
                max_w=FRAME_W - 2),
            fit(gtext("Apache-2.0 \u00b7 an independent rewrite, not an official Google product",
                      size=24, color=SUBTLE), max_w=FRAME_W - 2),
        ).arrange(DOWN, buff=0.35)
        links.next_to(bars, DOWN, buff=0.7)

        self.play(FadeIn(title, shift=UP * 0.3), run_time=0.9)
        self.play(*[GrowFromEdge(b, LEFT if i % 2 == 0 else RIGHT)
                    for i, b in enumerate(bars)], run_time=0.8)
        self.play(FadeIn(links), run_time=0.9)
        self.wait(2.6)
        footer(self, "thanks for watching")
        self.wait(1.5)


ALL_SCENES = [
    TitleScene,
    PitchScene,
    DefectScene,
    ArchitectureScene,
    CoverageScene,
    InstallScene,
    CliScene,
    AuthStreamingScene,
    GauntletScene,
    OutroScene,
]
