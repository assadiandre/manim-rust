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
- M13 CE object API: deferred `Circle()` / `Create(c)` / `c.animate.shift`,
  `Scene.construct()`, ArrowVectorField (baked) and Table (Typst text grid).
  Python still only *builds* timeline data; the per-frame path stays in Rust.
- M14 graphing + live decimals: area between two baked curves, DashedVMobject
  on groups, ComplexPlane `i` labels, ChangingDecimal from a Typst digit
  atlas (no Python / no Typst in the frame loop).
- M15 matching + everyday shapes: TransformMatchingShapes (normalized
  shape-hash leaf pairs, shift matches, fade leftovers), Arc / Sector /
  Annulus, and CE wrappers for NumberLine / ComplexPlane / Brace /
  SurroundingRectangle.
- M16 graph labels + LaggedStart: baked `get_graph_label` via point_at_x
  (no per-frame Python), LaggedStart compiled to staggered start offsets.
- M17 Axes.plot / get_area / get_riemann_rectangles bake a Python `f(x)`
  at authoring time; RoundedRectangle, AnnularSector, BackgroundRectangle.
- M18 LabeledDot / LabeledLine (Typst label + geometry group) and
  MoveToTarget from `generate_target()` shift/move_to records.
- M19 Elbow / CubicBezier wrappers and CE rate-func aliases
  (`rush_into` / `rush_from`).
- M20 MarkupText / Paragraph via in-process Typst (Pango subset rewritten
  at authoring time) and Table inner/outer grid lines as baked geometry.
- M21 lists, MathTable, BarChart, FunctionGraph / ParametricFunction:
  bullets and math cells via Typst, bars as baked rects, Python `f`
  sampled once at authoring time.
- M22 table row/col labels and highlighted cells; Triangle /
  ArcBetweenPoints / CurvedDoubleArrow / LabeledArrow; Unwrite, Blink,
  GrowArrow, and FocusOn as compiled timeline data.
- M23 MathTex part splits (`{{...}}` / multi-string) with `set_color_by_tex`;
  NetworkX-free `Graph` / `DiGraph` (circular / spring / tree layouts);
  TangentLine / get_vertical_line_to_graph; ShowIncreasingSubsets and
  CyclicReplace as compiled timeline data; TransformMatchingTex pairs
  `tex-part:` children by substring.
- M24 SVGMobject via in-process `usvg` (paths only, no Typst SVG
  round-trip) and Union / Intersection / Difference / Exclusion as
  authoring-time `i_overlay` path ops.
- M25 ImplicitFunction via marching squares (Python `f(x,y)` sampled once);
  FadeTransform stretch as a single Travel (shift+scale) animation;
  ApplyWave as compiled path data that rests at the endpoints.
- M26 ImageMobject: authoring-time RGBA decode onto the mobject; vello
  `draw_image` at encode time (no per-frame Python, no extra GPU API).
  ArcPolygon sides are circular arcs baked into a `BezPath`.
- M27 SVG `<image>` rasters (data-URI PNG/JPEG/GIF/WebP via usvg) and
  `save_state` / `Restore` as compiled transform+style+path data.
- M28 `become` copies path/style/raster/transform (CE default
  `match_center=False`; authoring-time, no per-frame Python).
- M29 TransformFromCopy (target interpolates from the source snapshot),
  Broadcast (lagged Restore copies), FadeToColor, and VDict.
- M8 (later) 3D spike: cube pass composited under 2D overlay
