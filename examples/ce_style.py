"""CE-style authoring: Circle / Create / animate.shift.

Run after `maturin develop -m crates/manim-py/Cargo.toml`.
"""

from manim_rust import BLUE, Create, Circle, RIGHT, Scene, Write, MathTex


class Demo(Scene):
    def construct(self):
        c = Circle(radius=1.5, color=BLUE, fill_opacity=1)
        formula = MathTex(r"e^{i\pi} + 1 = 0")
        formula.shift((0.0, 2.4))
        self.play(Create(c))
        self.play(c.animate.shift(RIGHT))
        self.play(Write(formula))
        self.wait(0.4)


if __name__ == "__main__":
    Demo().render("media/ce_style.mp4", fps=60)
    print("wrote media/ce_style.mp4")
