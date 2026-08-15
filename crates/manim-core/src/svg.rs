//! Authoring-time SVG import: parse once with `usvg` into `kurbo::BezPath`
//! mobjects. `usvg` is a parser, not a render backend.
//!
//! This is user SVG import, not a Typst round-trip.

use kurbo::{Affine, BezPath, Point, Rect, Shape};
use peniko::{Color, ImageData};
use usvg::tiny_skia_path::PathSegment;

use crate::mobject::Mobject;
use crate::raster::raster_from_bytes;
use crate::scene::{NodeId, SceneGraph};
use crate::style::Style;

/// Failure while parsing an SVG or converting it to path mobjects.
#[derive(Debug)]
pub enum SvgError {
    Parse(String),
    Empty,
}

impl std::fmt::Display for SvgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SvgError::Parse(e) => write!(f, "svg parse error: {e}"),
            SvgError::Empty => write!(f, "svg contained no visible paths or images"),
        }
    }
}

impl std::error::Error for SvgError {}

struct Fragment {
    path: BezPath,
    style: Style,
    image: Option<ImageData>,
}

/// Parse `svg` once into centered, y-up path mobjects of logical height `height`.
///
/// If `height <= 0`, uses `2.0` (Manim `SVGMobject` default).
pub fn svg_mobjects(svg: &str, height: f64) -> Result<Vec<Mobject>, SvgError> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| SvgError::Parse(e.to_string()))?;

    let mut fragments = Vec::new();
    walk_group(tree.root(), &mut fragments);
    if fragments.is_empty() {
        return Err(SvgError::Empty);
    }

    let mut bbox: Option<Rect> = None;
    for frag in &fragments {
        let b = frag.path.bounding_box();
        bbox = Some(match bbox {
            None => b,
            Some(acc) => acc.union(b),
        });
    }
    let bbox = bbox.ok_or(SvgError::Empty)?;
    let center = bbox.center();
    let content_h = bbox.height();
    let target_h = if height <= 0.0 { 2.0 } else { height };
    let s = if content_h > 1e-12 {
        target_h / content_h
    } else {
        1.0
    };
    // A * B applies B first: center in SVG user space (y-down), then flip Y
    // and scale so the union height becomes `target_h`.
    let to_logical =
        Affine::scale_non_uniform(s, -s) * Affine::translate((-center.x, -center.y));

    let mobjects: Vec<Mobject> = fragments
        .into_iter()
        .filter_map(|frag| {
            let path = to_logical * frag.path;
            if path.elements().is_empty() {
                None
            } else {
                let mut m = Mobject::new(path).with_style(frag.style);
                if let Some(img) = frag.image {
                    m = m.with_image(img);
                }
                Some(m)
            }
        })
        .collect();
    if mobjects.is_empty() {
        return Err(SvgError::Empty);
    }
    Ok(mobjects)
}

/// Parse `svg` and add the paths as children of a group named `"svg"`.
pub fn add_svg(graph: &mut SceneGraph, svg: &str, height: f64) -> Result<NodeId, SvgError> {
    let parts = svg_mobjects(svg, height)?;
    let group = graph.add(Mobject::group().named("svg"));
    for part in parts {
        graph.add_child(group, part);
    }
    Ok(group)
}

fn walk_group(group: &usvg::Group, out: &mut Vec<Fragment>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => walk_group(child, out),
            usvg::Node::Path(path) => {
                if path.is_visible() {
                    if let Some(frag) = path_to_fragment(path) {
                        out.push(frag);
                    }
                }
            }
            usvg::Node::Image(image) => {
                if image.is_visible() {
                    if let Some(frag) = image_to_fragment(image) {
                        out.push(frag);
                    }
                }
            }
            usvg::Node::Text(_) => {}
        }
    }
}

fn path_to_fragment(path: &usvg::Path) -> Option<Fragment> {
    let mut bez = tiny_path_to_bez(path.data());
    if bez.elements().is_empty() {
        return None;
    }
    bez = usvg_transform_to_affine(path.abs_transform()) * bez;

    let mut style = Style::default().no_fill().no_stroke();
    if let Some(fill) = path.fill() {
        if let Some(color) = paint_to_color(fill.paint()) {
            style.fill = Some(color);
            style.fill_opacity = fill.opacity().get();
        }
    }
    if let Some(stroke) = path.stroke() {
        if let Some(color) = paint_to_color(stroke.paint()) {
            style.stroke = Some(color);
            style.stroke_width = f64::from(stroke.width().get());
            style.stroke_opacity = stroke.opacity().get();
        }
    }
    Some(Fragment {
        path: bez,
        style,
        image: None,
    })
}

fn image_to_fragment(image: &usvg::Image) -> Option<Fragment> {
    let bytes: &[u8] = match image.kind() {
        usvg::ImageKind::JPEG(b)
        | usvg::ImageKind::PNG(b)
        | usvg::ImageKind::GIF(b)
        | usvg::ImageKind::WEBP(b) => b.as_slice(),
        usvg::ImageKind::SVG(_) => return None,
    };
    let raster = raster_from_bytes(bytes).ok()?;
    let bb = image.abs_bounding_box();
    let path = Rect::new(
        f64::from(bb.x()),
        f64::from(bb.y()),
        f64::from(bb.x() + bb.width()),
        f64::from(bb.y() + bb.height()),
    )
    .to_path(0.1);
    Some(Fragment {
        path,
        style: Style::default().no_fill().no_stroke(),
        image: Some(raster),
    })
}

fn tiny_path_to_bez(data: &usvg::tiny_skia_path::Path) -> BezPath {
    let mut bez = BezPath::new();
    for seg in data.segments() {
        match seg {
            PathSegment::MoveTo(p) => bez.move_to(pt(p)),
            PathSegment::LineTo(p) => bez.line_to(pt(p)),
            PathSegment::QuadTo(p1, p) => bez.quad_to(pt(p1), pt(p)),
            PathSegment::CubicTo(p1, p2, p) => bez.curve_to(pt(p1), pt(p2), pt(p)),
            PathSegment::Close => bez.close_path(),
        }
    }
    bez
}

fn pt(p: usvg::tiny_skia_path::Point) -> Point {
    Point::new(f64::from(p.x), f64::from(p.y))
}

fn usvg_transform_to_affine(t: usvg::Transform) -> Affine {
    // usvg / SVG matrix(sx, ky, kx, sy, tx, ty) → kurbo column-major [a,b,c,d,e,f].
    Affine::new([
        f64::from(t.sx),
        f64::from(t.ky),
        f64::from(t.kx),
        f64::from(t.sy),
        f64::from(t.tx),
        f64::from(t.ty),
    ])
}

fn paint_to_color(paint: &usvg::Paint) -> Option<Color> {
    match paint {
        usvg::Paint::Color(c) => Some(Color::from_rgba8(c.red, c.green, c.blue, 255)),
        usvg::Paint::LinearGradient(g) => first_stop_color(g.stops()),
        usvg::Paint::RadialGradient(g) => first_stop_color(g.stops()),
        usvg::Paint::Pattern(_) => None,
    }
}

fn first_stop_color(stops: &[usvg::Stop]) -> Option<Color> {
    let stop = stops.first()?;
    let c = stop.color();
    let a = (stop.opacity().get() * 255.0).round() as u8;
    Some(Color::from_rgba8(c.red, c.green, c.blue, a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_circle_is_named_group() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="red"/></svg>"#;
        let mut graph = SceneGraph::new();
        let id = add_svg(&mut graph, svg, 2.0).expect("circle svg");
        assert_eq!(graph.get(id).name.as_deref(), Some("svg"));
        let kids = graph.children_of(id);
        assert!(
            kids.iter()
                .any(|&c| !graph.get(c).path.elements().is_empty()),
            "expected at least one path child"
        );
        assert!(
            kids.iter().any(|&c| graph.get(c).style.fill.is_some()),
            "expected a child with a fill"
        );
    }

    #[test]
    fn svg_two_rects_two_children() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10">
            <rect x="0" y="0" width="8" height="8" fill="blue"/>
            <rect x="10" y="0" width="8" height="8" fill="green"/>
        </svg>"#;
        let mut graph = SceneGraph::new();
        let id = add_svg(&mut graph, svg, 2.0).expect("two-rect svg");
        let n_paths = graph
            .children_of(id)
            .iter()
            .filter(|&&c| !graph.get(c).path.elements().is_empty())
            .count();
        assert!(n_paths >= 2, "expected at least 2 path children, got {n_paths}");
    }

    const EMBED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAYAAABytg0kAAAAFklEQVR4nGP4/5/hf8SRu/8ZQASIAwBqPQvrM5aq/wAAAABJRU5ErkJggg==";

    #[test]
    fn svg_embedded_png_has_image_child() {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
                <image href="data:image/png;base64,{EMBED_PNG}" x="0" y="0" width="10" height="10"/>
            </svg>"##
        );
        let mut graph = SceneGraph::new();
        let id = add_svg(&mut graph, &svg, 2.0).expect("embedded png svg");
        let kids = graph.children_of(id);
        assert!(
            kids.iter().any(|&c| graph.get(c).image.is_some()),
            "expected a raster child from <image>"
        );
    }

    #[test]
    fn empty_svg_errors() {
        let mut graph = SceneGraph::new();
        let err = add_svg(
            &mut graph,
            r#"<svg xmlns="http://www.w3.org/2000/svg"></svg>"#,
            2.0,
        )
        .expect_err("empty svg");
        assert!(
            matches!(err, SvgError::Empty | SvgError::Parse(_)),
            "unexpected error: {err:?}"
        );
    }
}
