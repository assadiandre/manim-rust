use kurbo::{Affine, BezPath, Vec2};
use peniko::ImageData;

use crate::style::Style;

/// A vector mobject: one path + style + local transform.
///
/// Hierarchies live in `SceneGraph` (arena); a `Mobject` with an empty path
/// acts as a group when given children there. Kept deliberately plain-data so
/// animations can snapshot and restore state cheaply.
///
/// Optional `image` is authoring-time raster (Manim `ImageMobject`). The path
/// is then the layout rectangle; vello samples the pixels at encode time.
#[derive(Clone, Debug)]
pub struct Mobject {
    pub name: Option<String>,
    pub path: BezPath,
    pub style: Style,
    pub transform: Affine,
    pub visible: bool,
    /// Higher draws later (on top). Manim `z_index`.
    pub z_index: i32,
    pub image: Option<ImageData>,
}

impl Mobject {
    pub fn new(path: BezPath) -> Self {
        Self {
            name: None,
            path,
            style: Style::default(),
            transform: Affine::IDENTITY,
            visible: true,
            z_index: 0,
            image: None,
        }
    }

    pub fn with_image(mut self, image: ImageData) -> Self {
        self.image = Some(image);
        self
    }

    /// Empty-path node used purely as a transform/style parent.
    pub fn group() -> Self {
        Self::new(BezPath::new())
    }

    pub fn named(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn with_transform(mut self, transform: Affine) -> Self {
        self.transform = transform;
        self
    }

    pub fn shifted(self, delta: Vec2) -> Self {
        let t = Affine::translate(delta) * self.transform;
        self.with_transform(t)
    }

    pub fn scaled(self, factor: f64) -> Self {
        let t = Affine::scale(factor) * self.transform;
        self.with_transform(t)
    }

    pub fn with_z_index(mut self, z: i32) -> Self {
        self.z_index = z;
        self
    }
}
