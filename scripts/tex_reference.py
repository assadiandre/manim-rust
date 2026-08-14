"""ManimCE reference renders for LaTeX formula parity.

One scene per formula; rendered with `-s` each scene's last frame lands in
media/images/tex_reference/{SceneName}.png. Keep FORMULAS in sync with
compare_tex.py.

    ../.venv-ref/bin/python -m manim -qh -s tex_reference.py
"""

from manim import MathTex, Scene

FORMULAS = [
    r"e^{i\pi} + 1 = 0",
    r"\frac{a}{b} + \sqrt{x}",
    r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}",
]


def _make(i, formula):
    class TexRef(Scene):
        def construct(self):
            self.add(MathTex(formula))
            self.wait(0.1)

    TexRef.__name__ = f"TexRef{i}"
    return TexRef


for _i, _f in enumerate(FORMULAS):
    globals()[f"TexRef{_i}"] = _make(_i, _f)
