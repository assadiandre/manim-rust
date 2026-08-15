//! manim-typst: Typst math/text -> vector mobjects, fully in-process.
//!
//! Invariant #5 (DESIGN.md): no subprocess, no SVG round-trip. We compile
//! with the `typst` crate, walk the layout `Frame` directly, and convert
//! glyph outlines (via the font's own `ttf-parser` face) into `BezPath`s.
//!
//! Two input syntaxes:
//! - **Typst math** (native): [`math_mobjects`] / [`add_math`].
//! - **LaTeX math** (compatibility): [`tex_mobjects`] / [`add_tex`], converted
//!   by [mitex](https://crates.io/crates/mitex) and shimmed into scope by
//!   [`MITEX_PRELUDE`]. Caveats versus real LaTeX: no `\newcommand`/macro
//!   expansion, and errors are mitex's, not LaTeX's.
//!
//! Coordinate conventions (mirroring typst-svg):
//! - typst frames are y-down, in pt; glyph outlines are y-up in font units.
//! - glyph origin = pen + (x_offset, y_offset).at(size); outline mapped by
//!   scale(s, -s) with s = size_pt / units_per_em.
//! - we collect everything in pt space, then flip y and scale pt -> logical
//!   units (1 unit = `PT_PER_UNIT` pt), centered on the content's bbox.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use kurbo::{Affine, BezPath, Point, Shape, Size as KSize};
use manim_core::kurbo;
use manim_core::peniko::Color;
use manim_core::{Mobject, Style};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Frame, FrameItem};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::visualize::{CurveItem, Geometry, Paint};
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;

/// Logical units per typst pt. 48 pt per unit makes the default 48 pt font
/// render ~1 unit tall — close to Manim's default MathTex scale.
pub const PT_PER_UNIT: f64 = 48.0;

/// Reference device scale (px per logical unit at 1080p). Typst-internal
/// strokes (fraction bars, radical overlines) are converted to device px
/// with this; they won't track other resolutions — acceptable for now.
const PX_PER_UNIT_REF: f64 = 135.0;

/// Typst-scope shim for mitex output, ported from the official
/// `@preview/mitex` 0.2.4 package (`specs/latex/standard.typ`). mitex emits
/// identifiers like `mitexsqrt`, `pmatrix`, `mitexdisplay` that are normally
/// defined by that package's `mitex-scope`; since we compile in-process
/// without the package, we define the same surface at the document root.
///
/// Deliberately unsupported (mirroring mitex's own caveats): `\newcommand`
/// and friends (ignored by mitex), the `array` env's column spec, and the
/// `\color[model]{...}` form with an explicit color model.
const MITEX_PRELUDE: &str = r#"
#let mitexdisplay = math.display
#let mitexinline = math.inline
#let mitexscript = math.script
#let mitexsscript = math.sscript
#let mitexbold(it) = math.bold(math.upright(it))
#let mitexupright = math.upright
#let mitexitalic = math.italic
#let mitexmathbf(it) = math.bold(math.upright(it))
#let mitexoverbrace(it) = math.limits(math.overbrace(it))
#let mitexunderbrace(it) = math.limits(math.underbrace(it))
#let mitexnot(it) = math.cancel(angle: 20deg, it)
#let mitexset(it) = $\{ #it \}$
#let textmath(it) = it
#let textmd(it) = it
#let textnormal(it) = it
#let textbf(it) = math.bold(it)
#let textit(it) = math.italic(it)
#let textrm(it) = math.upright(it)
#let textup(it) = math.upright(it)
#let textsf(it) = math.sans(it)
#let texttt(it) = math.mono(it)
#let matrix = math.mat.with(delim: none)
#let pmatrix = math.mat.with(delim: "(")
#let bmatrix = math.mat.with(delim: "[")
#let Bmatrix = math.mat.with(delim: "{")
#let vmatrix = math.mat.with(delim: "|")
#let Vmatrix = math.mat.with(delim: "||")
#let smallmatrix(..args) = math.inline(math.mat(delim: none, ..args))
#let rcases = math.cases.with(reverse: true)
#let aligned(..args) = if args.pos().len() > 0 {
  math.op(math.display(args.pos().sum()))
} else { math.zws }
#let alignedat(n, ..args) = math.op(args.pos().sum())
#let operatornamewithlimits(it) = math.op(limits: true, math.upright(it))
#let boxed(it) = box(stroke: 0.5pt, inset: 6pt, it)
#let mitexsqrt(..args) = {
  let pos = args.pos()
  if pos.len() == 1 {
    math.sqrt(pos.at(0))
  } else {
    let idx = pos.at(0)
    let cleaned = if idx.has("children") {
      idx.children.filter(it => it != [\[] and it != [\]]).sum()
    } else { idx }
    math.root(cleaned, pos.at(1))
  }
}
#let mitex-color-map = (
  "red": rgb(255, 0, 0), "green": rgb(0, 255, 0), "blue": rgb(0, 0, 255),
  "cyan": rgb(0, 255, 255), "magenta": rgb(255, 0, 255), "yellow": rgb(255, 255, 0),
  "black": rgb(0, 0, 0), "white": rgb(255, 255, 255), "gray": rgb(128, 128, 128),
  "lightgray": rgb(192, 192, 192), "darkgray": rgb(64, 64, 64), "brown": rgb(165, 42, 42),
  "orange": rgb(255, 165, 0), "pink": rgb(255, 182, 193), "purple": rgb(128, 0, 128),
  "teal": rgb(0, 128, 128), "olive": rgb(128, 128, 0),
)
#let mitex-tex-str(it) = if it.has("children") {
  it.children.filter(c => c != [ ] and c != [#math.zws]).map(c => c.text).sum()
} else { it.text }
#let mitexcolor(spec, ..rest) = {
  let c = mitex-color-map.at(lower(mitex-tex-str(spec)), default: none)
  if c != none { text(fill: c, rest.pos().sum()) } else { rest.pos().sum() }
}
#let colortext(spec, body) = {
  let c = mitex-color-map.at(lower(mitex-tex-str(spec)), default: none)
  if c != none { text(fill: c, body) } else { body }
}
#let mitexlabel(..args) = { }
"#;

#[derive(Debug)]
pub enum TypstError {
    Compile(String),
    EmptyDocument,
    TexConvert(String),
}

impl std::fmt::Display for TypstError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypstError::Compile(e) => write!(f, "typst compile failed: {e}"),
            TypstError::EmptyDocument => write!(f, "typst produced no pages"),
            TypstError::TexConvert(e) => write!(f, "mitex conversion failed: {e}"),
        }
    }
}

impl std::error::Error for TypstError {}

// ---------------------------------------------------------------------------
// Minimal in-memory World

struct Fonts {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
}

static FONTS: LazyLock<Fonts> = LazyLock::new(|| {
    let fonts: Vec<Font> = typst_assets::fonts()
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();
    let book = LazyHash::new(FontBook::from_fonts(&fonts));
    Fonts { fonts, book }
});

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| LazyHash::new(Library::default()));

struct MathWorld {
    source: Source,
    main: FileId,
}

impl MathWorld {
    fn new(markup: &str) -> Self {
        let source = Source::detached(markup.to_owned());
        let main = FileId::new(RootedPath::new(
            VirtualRoot::Project,
            VirtualPath::new("main.typ").expect("valid virtual path"),
        ));
        Self { source, main }
    }
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound(std::path::PathBuf::from(
                "not-in-memory",
            )))
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        if id == self.main {
            Ok(Bytes::from_string(self.source.text().to_owned()))
        } else {
            Err(typst::diag::FileError::NotFound(std::path::PathBuf::from(
                "not-in-memory",
            )))
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

// ---------------------------------------------------------------------------
// Frame walking

/// A filled or stroked vector fragment in typst pt space (y-down).
struct Fragment {
    path: BezPath,
    fill: Option<Color>,
    stroke: Option<(Color, f64)>,
}

fn paint_to_color(paint: &Paint) -> Color {
    match paint {
        Paint::Solid(c) => {
            let [r, g, b, a] = c.to_vec4_u8();
            Color::from_rgba8(r, g, b, a)
        }
        // Gradients/tilings: approximate with white for now (rare in math).
        _ => Color::from_rgba8(255, 255, 255, 255),
    }
}

fn typst_transform_to_affine(t: &typst::layout::Transform) -> Affine {
    Affine::new([
        t.sx.get(),
        t.ky.get(),
        t.kx.get(),
        t.sy.get(),
        t.tx.to_pt(),
        t.ty.to_pt(),
    ])
}

/// Converts ttf font units to a y-down pt-space path at the glyph origin.
struct OutlineSink {
    path: BezPath,
    scale: f64,
}

impl ttf_parser::OutlineBuilder for OutlineSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to((x as f64 * self.scale, -y as f64 * self.scale));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((x as f64 * self.scale, -y as f64 * self.scale));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.quad_to(
            (x1 as f64 * self.scale, -y1 as f64 * self.scale),
            (x as f64 * self.scale, -y as f64 * self.scale),
        );
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.curve_to(
            (x1 as f64 * self.scale, -y1 as f64 * self.scale),
            (x2 as f64 * self.scale, -y2 as f64 * self.scale),
            (x as f64 * self.scale, -y as f64 * self.scale),
        );
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

fn walk_frame(frame: &Frame, transform: Affine, out: &mut Vec<Fragment>) {
    for (pos, item) in frame.items() {
        let at = transform * Affine::translate((pos.x.to_pt(), pos.y.to_pt()));
        match item {
            FrameItem::Group(group) => {
                walk_frame(
                    &group.frame,
                    at * typst_transform_to_affine(&group.transform),
                    out,
                );
            }
            FrameItem::Text(text) => {
                let fill = paint_to_color(&text.fill);
                let scale = text.size.to_pt() / text.font.units_per_em();
                let mut pen = Point::ORIGIN;
                for glyph in &text.glyphs {
                    let origin = pen
                        + kurbo::Vec2::new(
                            glyph.x_offset.at(text.size).to_pt(),
                            glyph.y_offset.at(text.size).to_pt(),
                        );
                    let mut sink = OutlineSink {
                        path: BezPath::new(),
                        scale,
                    };
                    let face = text.font.ttf();
                    if face
                        .outline_glyph(ttf_parser::GlyphId(glyph.id), &mut sink)
                        .is_some()
                    {
                        out.push(Fragment {
                            path: at * Affine::translate(origin.to_vec2()) * sink.path,
                            fill: Some(fill),
                            stroke: None,
                        });
                    }
                    pen.x += glyph.x_advance.at(text.size).to_pt();
                    pen.y += glyph.y_advance.at(text.size).to_pt();
                }
            }
            FrameItem::Shape(shape, _span) => {
                let path = match &shape.geometry {
                    Geometry::Line(to) => {
                        let mut p = BezPath::new();
                        p.move_to((0.0, 0.0));
                        p.line_to((to.x.to_pt(), to.y.to_pt()));
                        p
                    }
                    Geometry::Rect(size) => {
                        kurbo::Rect::from_origin_size(
                            Point::ORIGIN,
                            KSize::new(size.x.to_pt(), size.y.to_pt()),
                        )
                        .to_path(0.1)
                    }
                    Geometry::Curve(curve) => {
                        let mut p = BezPath::new();
                        for item in &curve.0 {
                            match item {
                                CurveItem::Move(p0) => {
                                    p.move_to((p0.x.to_pt(), p0.y.to_pt()))
                                }
                                CurveItem::Line(p1) => {
                                    p.line_to((p1.x.to_pt(), p1.y.to_pt()))
                                }
                                CurveItem::Cubic(c1, c2, p1) => p.curve_to(
                                    (c1.x.to_pt(), c1.y.to_pt()),
                                    (c2.x.to_pt(), c2.y.to_pt()),
                                    (p1.x.to_pt(), p1.y.to_pt()),
                                ),
                                CurveItem::Close => p.close_path(),
                            }
                        }
                        p
                    }
                };
                out.push(Fragment {
                    path: at * path,
                    fill: shape.fill.as_ref().map(paint_to_color),
                    stroke: shape.stroke.as_ref().map(|s| {
                        (paint_to_color(&s.paint), s.thickness.to_pt())
                    }),
                });
            }
            // Images/links/tags: not relevant for math layout.
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Public API

/// Options for `math_mobject`.
#[derive(Clone, Debug)]
pub struct MathOptions {
    /// Typst font size in pt (before scaling to logical units).
    pub font_size_pt: f64,
    /// Override fill color; `None` keeps typst's (black -> we map to white,
    /// since manim scenes default to dark backgrounds).
    pub color: Option<Color>,
}

impl Default for MathOptions {
    fn default() -> Self {
        Self {
            font_size_pt: 48.0,
            color: None,
        }
    }
}

/// Input syntax for the shared compile path.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Syntax {
    /// Native typst math syntax.
    Typst,
    /// LaTeX math syntax, converted to typst via mitex.
    Latex,
    /// Typst markup / plain text (not wrapped in `$...$`).
    Text,
}

/// Compile a math expression into mobjects in logical units, centered at the
/// origin. `source` is keyed into the memo cache together with `syntax`, so
/// the cache never conflates a LaTeX string with its converted typst form
/// (two different LaTeX strings may map to the same typst markup).
fn compile_mobjects(
    source: &str,
    syntax: Syntax,
    options: &MathOptions,
) -> Result<Vec<Mobject>, TypstError> {
    let key = (syntax as u8, source.to_owned(), options.font_size_pt.to_bits());
    static CACHE: LazyLock<Mutex<HashMap<(u8, String, u64), Vec<Mobject>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    if let Some(hit) = CACHE.lock().unwrap().get(&key) {
        return Ok(hit.clone());
    }

    let converted;
    let (body, prelude, math) = match syntax {
        Syntax::Typst => (source, "", true),
        Syntax::Text => (source, "", false),
        Syntax::Latex => {
            converted = mitex::convert_math(source, None).map_err(TypstError::TexConvert)?;
            (converted.as_str(), MITEX_PRELUDE, true)
        }
    };
    let markup = if math {
        format!(
            "#set page(width: auto, height: auto, margin: 0pt)\n#set text(size: {}pt)\n{prelude}$ {body} $",
            options.font_size_pt,
        )
    } else {
        format!(
            "#set page(width: auto, height: auto, margin: 0pt)\n#set text(size: {}pt)\n{body}",
            options.font_size_pt,
        )
    };
    let world = MathWorld::new(&markup);
    let warned = typst::compile::<PagedDocument>(&world);
    let doc = warned.output.map_err(|diags| {
        TypstError::Compile(
            diags
                .iter()
                .map(|d| d.message.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    let page = doc.pages().first().ok_or(TypstError::EmptyDocument)?;

    let mut fragments = Vec::new();
    walk_frame(&page.frame, Affine::IDENTITY, &mut fragments);

    // pt (y-down) -> logical units (y-up), centered on content bbox.
    let mut bbox: Option<kurbo::Rect> = None;
    for f in &fragments {
        let b = f.path.bounding_box();
        bbox = Some(match bbox {
            None => b,
            Some(acc) => acc.union(b),
        });
    }
    let center = bbox.map(|b| b.center()).unwrap_or(Point::ORIGIN);
    // A * B applies B first: move content center to origin (pt space), then
    // flip y and scale pt -> logical units.
    let to_logical = Affine::scale_non_uniform(1.0 / PT_PER_UNIT, -1.0 / PT_PER_UNIT)
        * Affine::translate((-center.x, -center.y));

    let default_fill = palette_white();
    let mobjects: Vec<Mobject> = fragments
        .into_iter()
        .filter_map(|f| {
            let mut style = Style::default().no_fill().no_stroke();
            if let Some(c) = options.color.or(f.fill) {
                // Typst text defaults to black; on manim-style dark scenes we
                // map pure black to white unless an explicit color was given.
                let c = match options.color {
                    Some(explicit) => explicit,
                    None => {
                        let rgba = c.to_rgba8();
                        if rgba.r == 0 && rgba.g == 0 && rgba.b == 0 {
                            default_fill
                        } else {
                            c
                        }
                    }
                };
                style = style.with_fill(c);
            }
            if let Some((c, w)) = f.stroke {
                style = style.with_stroke(c, w * PX_PER_UNIT_REF / PT_PER_UNIT);
            }
            let path = to_logical * f.path;
            if path.elements().is_empty() {
                None
            } else {
                Some(Mobject::new(path).with_style(style))
            }
        })
        .collect();

    CACHE.lock().unwrap().insert(key, mobjects.clone());
    Ok(mobjects)
}

/// Compile a math expression in **typst math syntax** (without the
/// surrounding `$...$`) into mobjects. This is the native path.
///
/// Results are memoized: identical sources compile once.
pub fn math_mobjects(source: &str, options: &MathOptions) -> Result<Vec<Mobject>, TypstError> {
    compile_mobjects(source, Syntax::Typst, options)
}

/// Compile a math expression in **LaTeX math syntax** into mobjects, via a
/// mitex conversion to typst. Symbol-level caveats versus real LaTeX:
/// no `\newcommand`/macro definitions, and conversion errors come from mitex,
/// not LaTeX. See `MITEX_PRELUDE` for the supported command surface.
///
/// Results are memoized on the *original LaTeX string*.
pub fn tex_mobjects(source: &str, options: &MathOptions) -> Result<Vec<Mobject>, TypstError> {
    compile_mobjects(source, Syntax::Latex, options)
}

fn palette_white() -> Color {
    Color::from_rgba8(255, 255, 255, 255)
}

fn add_group(
    scene: &mut manim_core::SceneGraph,
    parts: Vec<Mobject>,
    source: &str,
) -> manim_core::NodeId {
    let group = scene.add(Mobject::group().named(format!("tex:{source}")));
    for part in parts {
        scene.add_child(group, part);
    }
    group
}

/// Convenience: compile typst math and add to a scene graph as a group.
/// Returns the group node id.
pub fn add_math(
    scene: &mut manim_core::SceneGraph,
    source: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let parts = math_mobjects(source, options)?;
    Ok(add_group(scene, parts, source))
}

/// Convenience: compile LaTeX math (via mitex) and add to a scene graph as a
/// group. Returns the group node id.
pub fn add_tex(
    scene: &mut manim_core::SceneGraph,
    source: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let parts = tex_mobjects(source, options)?;
    Ok(add_group(scene, parts, source))
}

/// Compile Typst markup / plain text (Manim `Text`) into a glyph group.
pub fn text_mobjects(source: &str, options: &MathOptions) -> Result<Vec<Mobject>, TypstError> {
    compile_mobjects(source, Syntax::Text, options)
}

pub fn add_text(
    scene: &mut manim_core::SceneGraph,
    source: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let parts = text_mobjects(source, options)?;
    Ok(add_group(scene, parts, source))
}

/// Plain text parked on the top edge (Manim `Title`).
pub fn add_title(
    scene: &mut manim_core::SceneGraph,
    source: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let id = add_text(scene, source, options)?;
    scene.to_edge(id, manim_core::constants::UP, 0.4);
    Ok(id)
}

/// Static decimal as Typst text (Manim `DecimalNumber`, not live-updating).
pub fn add_decimal(
    scene: &mut manim_core::SceneGraph,
    value: f64,
    num_decimal_places: usize,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let src = format!("{value:.prec$}", prec = num_decimal_places);
    add_text(scene, &src, options)
}

/// Brace plus a text label on the brace's outer side (Manim `BraceLabel`).
pub fn add_brace_label(
    scene: &mut manim_core::SceneGraph,
    target: manim_core::NodeId,
    direction: kurbo::Vec2,
    label: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let brace = manim_core::add_brace(scene, target, direction, 0.15, Style::default());
    let text = add_text(scene, label, options)?;
    scene.next_to(text, brace, direction, 0.12);
    Ok(scene.group_nodes(&[brace, text]))
}

fn format_matrix_entry(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// Matrix as Typst math `mat(...)` (Manim `Matrix`).
pub fn add_matrix(
    scene: &mut manim_core::SceneGraph,
    rows: &[Vec<f64>],
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    if rows.is_empty() || rows.iter().any(|r| r.is_empty()) {
        return Err(TypstError::Compile("empty matrix".into()));
    }
    let body = rows
        .iter()
        .map(|row| {
            row.iter()
                .copied()
                .map(format_matrix_entry)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect::<Vec<_>>()
        .join("; ");
    add_math(scene, &format!("mat({body})"), options)
}

fn escape_typst_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

/// Static code listing as a Typst raw block (Manim `Code`).
///
/// DejaVu Sans Mono is bundled in `typst-assets` (same fonts `MathWorld`
/// already loads), so we set it explicitly. No extra font download.
pub fn add_code(
    scene: &mut manim_core::SceneGraph,
    source: &str,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let escaped = escape_typst_string(source);
    let markup = format!(
        "#set par(leading: 0.65em)\n#set text(font: \"DejaVu Sans Mono\", size: {}pt)\n#raw(block: true, \"{escaped}\")",
        options.font_size_pt,
    );
    match add_text(scene, &markup, options) {
        Ok(id) => Ok(id),
        Err(_) => add_text(scene, source, options),
    }
}

/// Tick values matching `construct.rs` number-line ticks: `x_min..=x_max`
/// by `x_step`, skipping `x_max` when `include_tip` (the tip occupies that end).
fn number_line_ticks(x_min: f64, x_max: f64, x_step: f64, include_tip: bool) -> Vec<f64> {
    if x_step <= 0.0 {
        return Vec::new();
    }
    let n = ((x_max - x_min) / x_step).round() as i32;
    let mut out = Vec::new();
    for i in 0..=n {
        let x = x_min + i as f64 * x_step;
        if include_tip && (x - x_max).abs() < 1e-9 {
            continue;
        }
        out.push(x);
    }
    out
}

fn add_tick_label(
    scene: &mut manim_core::SceneGraph,
    value: f64,
    pos: Point,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let places = if value.fract().abs() < 1e-9 { 0 } else { 1 };
    let id = add_decimal(scene, value, places, options)?;
    scene.move_to(id, pos);
    Ok(id)
}

/// Decimal labels under a number line's ticks (Manim `NumberLine` labels).
pub fn add_number_line_labels(
    scene: &mut manim_core::SceneGraph,
    opts_x_min: f64,
    opts_x_max: f64,
    opts_x_step: f64,
    unit_size: f64,
    include_tip: bool,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let mut ids = Vec::new();
    for x in number_line_ticks(opts_x_min, opts_x_max, opts_x_step, include_tip) {
        ids.push(add_tick_label(
            scene,
            x,
            Point::new(x * unit_size, -0.35),
            options,
        )?);
    }
    Ok(scene.group_nodes(&ids))
}

/// Axis tick labels: x below the axis, y to the left. Origin is labeled
/// only on x so "0" is not drawn twice.
pub fn add_axes_labels(
    scene: &mut manim_core::SceneGraph,
    x_min: f64,
    x_max: f64,
    x_step: f64,
    y_min: f64,
    y_max: f64,
    y_step: f64,
    unit_size: f64,
    include_tip: bool,
    options: &MathOptions,
) -> Result<manim_core::NodeId, TypstError> {
    let mut ids = Vec::new();
    for x in number_line_ticks(x_min, x_max, x_step, include_tip) {
        ids.push(add_tick_label(
            scene,
            x,
            Point::new(x * unit_size, -0.35),
            options,
        )?);
    }
    for y in number_line_ticks(y_min, y_max, y_step, include_tip) {
        if y.abs() < 1e-9 {
            continue;
        }
        ids.push(add_tick_label(
            scene,
            y,
            Point::new(-0.4, y * unit_size),
            options,
        )?);
    }
    Ok(scene.group_nodes(&ids))
}
