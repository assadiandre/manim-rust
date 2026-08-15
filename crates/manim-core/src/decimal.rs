//! Composed decimal outlines from a baked glyph atlas.
//!
//! Used by ChangingDecimal: Typst compiles `0-9`, `.`, `-`, `+` once at
//! authoring time; the per-frame path only concatenates those outlines.

use kurbo::{Affine, BezPath, Point, Rect, Shape};

#[derive(Clone, Debug, Default)]
pub struct DigitAtlas {
    glyphs: Vec<(char, BezPath, f64)>,
}

impl DigitAtlas {
    pub fn insert(&mut self, ch: char, path: BezPath, width: f64) {
        if let Some(slot) = self.glyphs.iter_mut().find(|(c, _, _)| *c == ch) {
            *slot = (ch, path, width);
        } else {
            self.glyphs.push((ch, path, width));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    fn lookup(&self, ch: char) -> Option<(&BezPath, f64)> {
        self.glyphs
            .iter()
            .find(|(c, _, _)| *c == ch)
            .map(|(_, p, w)| (p, *w))
    }

    /// Left-to-right decimal string, then recentered on the origin so a
    /// ChangingDecimal keeps its world center as digits change.
    pub fn compose(&self, value: f64, places: usize) -> BezPath {
        let text = format!("{value:.prec$}", prec = places);
        let mut out = BezPath::new();
        let mut x = 0.0;
        for ch in text.chars() {
            let Some((path, width)) = self.lookup(ch) else {
                continue;
            };
            let bb = path.bounding_box();
            let t = Affine::translate((x - bb.x0, 0.0));
            out.extend((t * path.clone()).iter());
            x += width;
        }
        if out.elements().is_empty() {
            return out;
        }
        let bb = bounding_box(&out);
        Affine::translate(-bb.center().to_vec2()) * out
    }
}

fn bounding_box(path: &BezPath) -> Rect {
    let mut acc: Option<Rect> = None;
    for seg in path.segments() {
        let b = seg.bounding_box();
        acc = Some(match acc {
            None => b,
            Some(a) => a.union(b),
        });
    }
    acc.unwrap_or_else(|| Rect::from_center_size(Point::ORIGIN, (0.0, 0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry;

    fn box_glyph(w: f64, h: f64) -> BezPath {
        geometry::rect(Point::new(w * 0.5, h * 0.5), w, h)
    }

    #[test]
    fn compose_recenters_and_grows_with_digits() {
        let mut atlas = DigitAtlas::default();
        for ch in ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '.', '-'] {
            atlas.insert(ch, box_glyph(0.4, 0.6), 0.45);
        }
        let a = atlas.compose(1.0, 0);
        let b = atlas.compose(12.0, 0);
        let wa = bounding_box(&a).width();
        let wb = bounding_box(&b).width();
        assert!(wb > wa, "one digit {wa} vs two {wb}");
        let ca = bounding_box(&a).center();
        let cb = bounding_box(&b).center();
        assert!(ca.x.abs() < 1e-9 && ca.y.abs() < 0.05, "{ca:?}");
        assert!(cb.x.abs() < 1e-9 && cb.y.abs() < 0.05, "{cb:?}");
    }
}
