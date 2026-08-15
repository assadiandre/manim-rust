# manim_rust — DESIGN

A blazingly fast 2D reimagining of Manim (ManimCE), architected so 3D can be
added later without breaking the core. Rust engine, Python authoring API,
Typst (library, not subprocess) for all math/text layout.

## Architectural invariants (DO NOT VIOLATE)

1. **No Python callables in the per-frame path.** Animations are *data*
   (kind, target, time range, easing) evaluated by the Rust core. Python
   builds the timeline; Rust evaluates it.
2. **No full-scene redraws by default.** The scene graph is retained and
   dirty-tracked. Frames where nothing changed must reuse the previous
   render. Static subtrees may be cached.
3. **wgpu only.** No 2D-only graphics APIs (Skia, CoreGraphics). The 2D
   vector layer (vello) and a future 3D pass must share one wgpu device and
   composite into one target.
4. **Flat f32/f64 buffers for geometry.** Paths are `kurbo::BezPath`
   (contiguous), transforms are affine matrices applied at encode time.
   No per-point heap objects.
5. **Typst as a library.** Math/text is compiled in-process via the `typst`
   crate and the layout frame is walked directly to glyph outlines.
   No subprocess, no SVG round-trip.
6. **Every milestone is machine-verifiable.** Golden-image tests with
   tolerance, timing tests with generous budgets. No "looks done" claims.

## Crate layout

```
crates/
  manim-core    geometry (kurbo), styles, Mobject, SceneGraph + dirty flags
  manim-anim    declarative animations, easings, Timeline evaluator
  manim-render  headless wgpu context, vello encoding, PNG + mp4 (ffmpeg pipe)
  manim-typst   typst World impl, math source -> positioned glyph paths
  manim-cli     `manim` binary: demo scenes, render to png/mp4
  manim-py      PyO3 bindings: Scene / play / render from Python
```

## Frame pipeline

1. `Timeline::apply(&mut scene, t)` — evaluate active animations in parallel,
   mark affected nodes dirty. Finished/inactive animations cost nothing.
2. Dirty propagation through the scene graph.
3. Encode dirty paths into a `vello::Scene` (encoding is cheap; correctness
   first, subtree texture caching is a later optimization).
4. Vello renders to an offscreen texture; async readback.
5. Frame reuse: if no node is dirty and camera is unchanged, reuse the
   previous frame's pixels (the big win for mostly-static scenes).
6. Pixels go to PNG (`image` crate) or to ffmpeg stdin as rawvideo.

## Typst bridge

- Minimal `typst::World`: one in-memory source, embedded fonts from
  `typst-assets`, no file system, no package downloads.
- Compile `$...$` math, walk `FrameItem::Text` glyph runs, convert glyph
  outlines via `ttf-parser` into `kurbo::BezPath`, preserving fill color and
  position. Result is a normal `Mobject` group — animatable like anything else.

## 3D extensibility (designed-in, not built)

- `Camera` trait from day one; the 2D camera is orthographic over a logical
  14.22 x 8 unit frame (Manim-compatible).
- `Mobject` does not assume paths: `VectorMobject` is the 2D kind; a future
  `MeshMobject` renders in a separate wgpu pass composited under the 2D layer.
- Transforms are 2D affine now; the scene graph stores them opaquely so a
  4x4 upgrade touches one crate.

## Performance budgets (enforced by tests)

- Render 1000 static shapes: a repeated frame must reuse pixels (< 5 ms).
- Typst formula setup: < 500 ms cold, cached recompile < 5 ms.
- 1080p simple scene: encode+render well under 16 ms/frame on Apple Silicon.

## Milestones

- M0 workspace + vello circle -> PNG, golden test
- M1 geometry kernel + shape gallery goldens
- M2 scene graph + dirty tracking + frame reuse
- M3 declarative animation evaluator (Create/Transform/Fade/Shift/Scale)
- M4 mp4 via ffmpeg pipe, ffprobe-verified
- M5 typst math -> paths, golden render
- M6 PyO3 API: Scene, mobjects, play(), render()
- M7 north-star demo: circle morphs to square while $e^{i\pi}+1=0$ fades in
- M9 authoring surface (ManimCE-inspired): directions, bbox layout
  (`move_to`/`next_to`/`arrange`), richer geometry (arc/arrow/ellipse),
  Rotate/Uncreate/Write/Grow/Indicate, NumberLine/Axes + baked plots
- M10 everyday 2D CE: Typst `Text`, annotations (surrounding rect /
  underline / brace / cross), NumberPlane, z-index, Recolor /
  DrawBorderThenFill / Wiggle / Circumscribe, camera shift+zoom as
  timeline data (not a node), `to_corner` / `set_x`/`set_y` / `flip` /
  `stretch` / `arrange_in_grid` / `set_width`/`set_height`
- M11 more 2D CE: Star / RegularPolygon / AnnularSector / CurvedArrow /
  Angle / RightAngle, PolarPlane, Title / DecimalNumber / BraceLabel,
  full Manim A–E palette lookup, MoveAlongPath / ShowPassingFlash /
  Flash / GrowFromPoint / SpinInFromNothing / ShrinkToCenter
- M12 graphing + text extras: DashedVMobject, area-under-curve, Riemann
  rectangles, ComplexPlane, number-line/axes labels, Matrix, Code,
  FadeTransform (fade+travel, no stretch — Shift and Scale share one
  transform property)
- M8 (later) 3D spike: cube pass composited under 2D overlay
