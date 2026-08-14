"""Authoring-surface demo: layout, arrows, and a baked plot on axes.

Run after `maturin develop -m crates/manim-py/Cargo.toml`.
"""

from manim_rust import Scene

scene = Scene(1920, 1080)

axes = scene.add_axes(x_min=-3, x_max=3, y_min=-1, y_max=3, unit_size=1.0)
plot = scene.add_function(lambda x: 0.35 * x * x, -2.2, 2.2, stroke="yellow", stroke_width=5.0)

dot = scene.add_dot(1.5, 0.35 * 1.5 * 1.5, fill="red")
label = scene.add_tex(r"y = 0.35 x^2", x=0.0, y=0.0)
scene.next_to(label, dot, "ur", buff=0.2)

arrow = scene.add_arrow(-2.0, 2.2, -0.4, 0.6, buff=0.0, stroke="gold")

scene.play_create(axes, duration=1.0)
scene.play_create(plot, duration=1.2)
scene.play_grow(dot, duration=0.5)
scene.play_write(label, duration=1.0)
scene.play_create(arrow, duration=0.6)
scene.play_indicate(dot, duration=0.8)
scene.wait(0.4)

scene.render("media/layout_and_axes.mp4", fps=60)
print(f"wrote media/layout_and_axes.mp4 ({scene.duration():.1f}s)")
