"""ManimCE references matching manim_rust M25–M29 probes (480x270).

Rendered by scripts/compare_side_by_side.py. Geometry and colors are pinned
to the Rust probes so the side-by-side is a real parity check, not a vibe check.
"""

from manim import (
    PI,
    UP,
    ApplyWave,
    ArcPolygon,
    Broadcast,
    Circle,
    Difference,
    FadeToColor,
    FadeTransform,
    ImageMobject,
    ImplicitFunction,
    Intersection,
    Line,
    Restore,
    Scene,
    Square,
    TransformFromCopy,
    Union,
    WHITE,
    YELLOW,
    config,
)
from manim.utils.color import ManimColor

config.pixel_width = 480
config.pixel_height = 270
config.frame_rate = 10
config.background_color = "#000000"

BLUE = ManimColor("#58C4DD")
TEAL = ManimColor("#5CD0B3")
RED = ManimColor("#FC6255")
GREEN = ManimColor("#83C167")


class ImplicitRef(Scene):
    def construct(self):
        c = ImplicitFunction(
            lambda x, y: x * x + y * y - 1.0,
            x_range=(-2.4, 2.4),
            y_range=(-1.6, 1.6),
            color=YELLOW,
            stroke_width=5,
        )
        h = ImplicitFunction(
            lambda x, y: x * x - y * y - 0.6,
            x_range=(-2.4, 2.4),
            y_range=(-1.6, 1.6),
            color=TEAL,
            stroke_width=4,
        )
        self.add(c, h)


class WaveRef(Scene):
    def construct(self):
        line = Line([-3.2, 0, 0], [3.2, 0, 0], color=ManimColor("#FFFF00"), stroke_width=6)
        line.insert_n_curves(48)
        self.add(line)
        self.play(
            ApplyWave(line, direction=UP, amplitude=0.25, ripples=2, run_time=1, rate_func=lambda t: t)
        )


class StretchRef(Scene):
    def construct(self):
        tiny = Circle(radius=0.35, fill_color=RED, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        tiny.move_to([-2.6, 0, 0])
        big = Square(side_length=2.2, fill_color=BLUE, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        big.move_to([2.4, 0, 0])
        self.add(tiny, big)
        self.play(FadeTransform(tiny, big, stretch=True, run_time=1.2, rate_func=lambda t: t))


class RasterRef(Scene):
    def construct(self):
        check = ImageMobject("ce_assets/checker.png").set_height(3.2).move_to([-2.6, 0, 0])
        grad = ImageMobject("ce_assets/gradient.png").set_height(1.6).move_to([2.4, 0, 0])
        self.add(check, grad)


class ArcpRef(Scene):
    def construct(self):
        p = ArcPolygon(
            [-1.4, -1.4, 0],
            [1.4, -1.4, 0],
            [1.4, 1.4, 0],
            [-1.4, 1.4, 0],
            angle=PI / 2,
            color=WHITE,
            fill_color=ManimColor("#FFFF00"),
            fill_opacity=1,
            stroke_width=5,
        )
        self.add(p)


class RestoreRef(Scene):
    def construct(self):
        c = Circle(radius=0.7, fill_color=YELLOW, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        c.move_to([-2.4, 0, 0])
        self.add(c)
        c.save_state()
        c.shift([4.8, 0, 0])
        c.set_opacity(0.25)
        self.play(Restore(c, run_time=1, rate_func=lambda t: t))


class BecomeRef(Scene):
    def construct(self):
        a = Circle(radius=0.45, fill_color=RED, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        a.move_to([-2.4, 0, 0])
        b = Square(side_length=1.8, fill_color=BLUE, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        b.move_to([2.4, 0, 0])
        self.add(a, b)
        a.become(b)
        self.remove(b)


class BooleanRef(Scene):
    def construct(self):
        def pair(x0, x1):
            a = Circle(radius=0.95, fill_color=BLUE, fill_opacity=1, stroke_color=WHITE, stroke_width=2)
            b = Circle(radius=0.95, fill_color=BLUE, fill_opacity=1, stroke_color=WHITE, stroke_width=2)
            a.move_to([x0, 0, 0])
            b.move_to([x1, 0, 0])
            return a, b

        a, b = pair(-3.4, -2.4)
        u = Union(a, b, fill_color=TEAL, fill_opacity=1, stroke_color=WHITE, stroke_width=3)
        c, d = pair(-0.4, 0.4)
        inter = Intersection(c, d, fill_color=YELLOW, fill_opacity=1, stroke_color=WHITE, stroke_width=3)
        e, f = pair(2.6, 3.4)
        diff = Difference(e, f, fill_color=RED, fill_opacity=1, stroke_color=WHITE, stroke_width=3)
        self.add(u, inter, diff)


class SvgRef(Scene):
    def construct(self):
        from manim import SVGMobject

        svg = SVGMobject("ce_assets/probe.svg").set_height(3.2)
        self.add(svg)


class TfcRef(Scene):
    def construct(self):
        src = Circle(
            radius=0.45,
            fill_color=ManimColor("#FFFF00"),
            fill_opacity=1,
            stroke_color=WHITE,
            stroke_width=4,
        )
        src.move_to([-2.4, 0, 0])
        dst = Square(side_length=1.6, fill_color=BLUE, fill_opacity=1, stroke_color=WHITE, stroke_width=4)
        dst.move_to([2.4, 0, 0])
        self.add(src, dst)
        self.play(TransformFromCopy(src, dst, run_time=1, rate_func=lambda t: t))


class BroadcastRef(Scene):
    def construct(self):
        ring = Circle(radius=2.2, color=TEAL, stroke_width=5, fill_opacity=0)
        self.play(Broadcast(ring, run_time=1.5, lag_ratio=0.2, n_mobs=5))


class FadeToRef(Scene):
    def construct(self):
        c = Circle(
            radius=1.1,
            fill_color=ManimColor("#FFFF00"),
            fill_opacity=1,
            stroke_color=WHITE,
            stroke_width=4,
        )
        self.add(c)
        self.play(FadeToColor(c, RED, run_time=1, rate_func=lambda t: t))
