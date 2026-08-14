"""ManimCE reference for the manim_rust north-star demo, rendered with the
manim-fork checkout. Styles are pinned explicitly so the end state matches
manim_rust semantics (morph keeps the circle's blue fill).

Run:  ../.venv-ref/bin/python -m manim -qh manimce_reference.py RustDemoReference
"""

from manim import Scene, Create, Transform, FadeIn, Circle, Square, MathTex, UP, BLUE, WHITE


class RustDemoReference(Scene):
    def construct(self):
        circle = Circle(
            radius=1.5,
            fill_color=BLUE,
            fill_opacity=1.0,
            stroke_color=WHITE,
            stroke_width=4.0,
        )
        self.play(Create(circle, run_time=1.0))

        square = Square(
            side_length=3.0,
            fill_color=BLUE,
            fill_opacity=1.0,
            stroke_color=WHITE,
            stroke_width=4.0,
        )
        self.play(Transform(circle, square, run_time=1.2))

        tex = MathTex(r"e^{i\pi} + 1 = 0").move_to(UP * 2.6)
        self.play(FadeIn(tex, run_time=0.8))

        self.wait(0.5)
