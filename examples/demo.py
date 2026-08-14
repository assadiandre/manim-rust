"""North-star demo: circle draws itself, morphs into a square, and Euler's
identity (typeset in-process by typst) fades in above it.

Run after `maturin develop -m crates/manim-py/Cargo.toml`.
"""

from manim_rust import Scene

scene = Scene(1920, 1080)

circle = scene.add_circle(radius=1.5, fill="blue", stroke="white", stroke_width=4.0)
# Morph target reference — play_morph consumes it (never shown on screen).
square = scene.add_square(side=3.0, stroke="white")
tex = scene.add_tex(r"e^{i\pi} + 1 = 0", y=2.6)  # LaTeX by default; syntax="typst" for native

scene.play_create(circle, duration=1.0)
scene.play_morph(circle, square, duration=1.2)
scene.play_fade_in(tex, duration=0.8)
scene.wait(0.5)

scene.render("media/demo.mp4", fps=60)
print(f"wrote media/demo.mp4 ({scene.duration():.1f}s)")
