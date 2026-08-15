# manim_rust

A blazingly fast, GPU-native 2D reimagining of [Manim](https://www.manim.community/)
(ManimCE), architected so 3D can be added later. Rust engine, Python
authoring API, [Typst](https://typst.app/) (as an in-process library) for all
math typesetting — no LaTeX install, no subprocesses, no SVG round-trips.

Read [DESIGN.md](DESIGN.md) first — it states the architectural invariants
every change must preserve.

## Why it's fast

| ManimCE bottleneck                     | manim_rust                                     |
| -------------------------------------- | ---------------------------------------------- |
| Python recomputes all points per frame | Rust timeline evaluator; animations are data   |
| Cairo software rasterization           | vello (GPU compute) on wgpu                    |
| Full-scene redraw every frame          | Dirty-tracked scene graph + frame reuse        |
| `latex` + `dvisvgm` subprocess (~1-3s) | `typst` crate in-process (~ms), memoized       |

## Layout

```
crates/
  manim-core    geometry kernel (kurbo), styles, dirty-tracked scene graph
  manim-anim    declarative animations + stateless timeline evaluator
  manim-render  headless wgpu+vello renderer, PNG, mp4 via ffmpeg pipe
  manim-typst   typst math -> glyph outline mobjects (in-process)
  manim-scenes  shared demo/probe scenes (CLI, goldens, visual checks)
  manim-cli     `manim` binary (demo scene, visual-check)
  manim-py      Python bindings (PyO3, abi3)
```

## Usage

### Rust / CLI

```bash
cargo run -p manim-cli --release -- demo --out media/demo.mp4
cargo run -p manim-cli --release -- png --time 1.6 --out media/frame.png
cargo run -p manim-cli --release -- demo --formula "sum_(n=1)^infinity 1/n^2 = pi^2/6"
cargo run -p manim-cli --release -- visual-check   # contact sheets for review
```

### Python

```bash
pip install maturin
maturin develop -m crates/manim-py/Cargo.toml   # or: maturin build + pip install
python examples/demo.py
```

```python
from manim_rust import Scene

scene = Scene(1920, 1080)
c = scene.add_circle(radius=1.5, fill="blue", stroke="white", stroke_width=4.0)
s = scene.add_square(side=3.0, stroke="white")
tex = scene.add_tex("e^{i pi} + 1 = 0", y=2.6)

scene.play_create(c, duration=1.0)
scene.play_morph(c, s, duration=1.2)
scene.play_fade_in(tex, duration=0.8)
scene.wait(0.5)

scene.render("media/demo.mp4", fps=60)
```

Layout, arrows, and axes follow ManimCE names (`next_to`, `arrange`,
`Write`, `Indicate`, `Circumscribe`, `Flash`, `MoveAlongPath`,
`FadeTransform`). Function plots, areas, and Riemann rectangles are
sampled once at authoring time — the per-frame path never calls back
into Python. Plain text uses in-process Typst (`add_text` / `add_title`
/ `add_code` / `add_matrix`); the camera is timeline data
(`play_camera_shift` / `play_camera_zoom`), not a scene-graph node.
Color names accept the Manim A–E palette (`blue_c`, `yellow_c`, …).

```python
axes = scene.add_axes(x_min=-3, x_max=3, y_min=-1, y_max=3)
plot = scene.add_function(lambda x: 0.35 * x * x, -2.2, 2.2, stroke="yellow")
label = scene.add_tex(r"y = 0.35 x^2")
scene.next_to(label, plot, "up")
scene.play_create(axes)
scene.play_write(label)
```

## Tests

```bash
cargo test                              # everything
UPDATE_GOLDEN=1 cargo test -p manim-render   # regenerate golden images
```

Golden-image tests live in `crates/manim-render/tests/golden.rs`; performance
budgets (frame reuse < 5ms, cached typst < 5ms) are enforced as tests.

## Visual verification

Two layers, both fed by the same `manim-scenes` probe definitions:

- **Human review**: `visual-check` renders every animation primitive, tex
  sample, and the demo at semantic timestamps into `media/visual_check/` with
  an `index.html` contact sheet. Review this after any rendering change.
- **CI**: `golden_demo_phases` pins the demo at mid-create / mid-morph /
  final frame, so a rendering regression fails `cargo test` even if every
  individual assertion still passes.
- **Ground truth**: `scripts/manimce_reference.py` renders the same scene
  with real ManimCE (LaTeX and all), and `scripts/compare.py` stacks
  [ours | ManimCE | amplified diff] per timestamp with numeric diff metrics
  in `media/visual_check/sidebyside/index.html`. Current numbers: mid-create
  0.16% px differ, mid-morph 5.4% (bezier-cp vs polyline morph), final 0.9%
  (Typst vs LaTeX glyph spacing). Setup:

```bash
uv venv .venv-ref && uv pip install -p .venv-ref/bin/python -e ../manim-fork
cd scripts && ../.venv-ref/bin/python -m manim -qh manimce_reference.py RustDemoReference
../.venv-ref/bin/python compare.py --ours ../media/demo.mp4 \
    --theirs media/videos/manimce_reference/1080p60/RustDemoReference.mp4 \
    --out ../media/visual_check/sidebyside
```

Conventions that differ from a naive reading of the code (learned from
visual review): `stroke_width` is in *device pixels* and does not scale with
transforms (Manim semantics); `play_morph` consumes its target reference
mobject; Typst's default formula size is 48pt (≈ Manim's MathTex scale).

## Requirements

- Rust stable (1.8x+)
- ffmpeg on PATH (for mp4 output)
- A GPU wgpu can use (Metal on macOS); CPU fallback adapter works but is slow

## Roadmap to 3D

The seams are designed in (see DESIGN.md): wgpu-only rendering, a `Camera`
trait, and a `Mobject` model that doesn't assume paths. A 3D pass would
render meshes to the same target and composite the 2D overlay on top.
