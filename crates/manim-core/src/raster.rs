//! Authoring-time raster images (Manim `ImageMobject`).
//!
//! Pixels are decoded once into a flat RGBA8 buffer and stored on the
//! mobject. The per-frame path only samples that buffer through vello.

use kurbo::Point;
use peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};

use crate::geometry;
use crate::mobject::Mobject;
use crate::scene::{NodeId, SceneGraph};
use crate::style::Style;

/// Failure while decoding or validating raster bytes.
#[derive(Debug)]
pub enum RasterError {
    Decode(String),
    Empty,
}

impl std::fmt::Display for RasterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RasterError::Decode(e) => write!(f, "image decode error: {e}"),
            RasterError::Empty => write!(f, "image is empty"),
        }
    }
}

impl std::error::Error for RasterError {}

/// Build an RGBA8 `ImageData` from tightly packed pixels.
pub fn raster_from_rgba(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<ImageData, RasterError> {
    if width == 0 || height == 0 {
        return Err(RasterError::Empty);
    }
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(RasterError::Decode(format!(
            "expected {expected} RGBA bytes for {width}x{height}, got {}",
            rgba.len()
        )));
    }
    Ok(ImageData {
        data: Blob::from(rgba),
        format: ImageFormat::Rgba8,
        alpha_type: ImageAlphaType::Alpha,
        width,
        height,
    })
}

/// Decode PNG/JPEG/etc. bytes once at authoring time.
pub fn raster_from_bytes(bytes: &[u8]) -> Result<ImageData, RasterError> {
    let img = image::load_from_memory(bytes).map_err(|e| RasterError::Decode(e.to_string()))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    raster_from_rgba(w, h, rgba.into_raw())
}

/// Decode an image file once at authoring time.
pub fn raster_from_path(path: impl AsRef<std::path::Path>) -> Result<ImageData, RasterError> {
    let bytes = std::fs::read(path.as_ref()).map_err(|e| RasterError::Decode(e.to_string()))?;
    raster_from_bytes(&bytes)
}

/// Place a raster so its logical height is `height` (default 2, like SVGMobject).
/// Width follows the pixel aspect ratio. Named `"image"`.
pub fn add_raster(graph: &mut SceneGraph, image: ImageData, height: f64) -> NodeId {
    let h = if height > 0.0 { height } else { 2.0 };
    let aspect = image.width as f64 / image.height.max(1) as f64;
    let w = h * aspect;
    graph.add(
        Mobject::new(geometry::rect(Point::ORIGIN, w, h))
            .with_image(image)
            .with_style(Style::default().no_fill().no_stroke())
            .named("image"),
    )
}

/// `nx` by `ny` checkerboard (one pixel per cell). Useful for goldens.
pub fn checkerboard(nx: u32, ny: u32, a: [u8; 4], b: [u8; 4]) -> ImageData {
    let mut px = Vec::with_capacity(nx as usize * ny as usize * 4);
    for y in 0..ny {
        for x in 0..nx {
            let c = if (x + y) % 2 == 0 { a } else { b };
            px.extend_from_slice(&c);
        }
    }
    raster_from_rgba(nx, ny, px).expect("checkerboard size is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SceneGraph;

    #[test]
    fn add_raster_is_named_and_uses_aspect() {
        let img = checkerboard(
            8,
            4,
            [255, 255, 0, 255],
            [88, 196, 221, 255],
        );
        let mut g = SceneGraph::new();
        let id = add_raster(&mut g, img, 2.0);
        assert_eq!(g.get(id).name.as_deref(), Some("image"));
        assert!(g.get(id).image.is_some());
        let bb = g.bounding_box(id);
        assert!((bb.height() - 2.0).abs() < 1e-6, "{bb:?}");
        assert!((bb.width() - 4.0).abs() < 1e-6, "{bb:?}");
    }

    #[test]
    fn empty_rgba_errors() {
        assert!(raster_from_rgba(0, 4, vec![]).is_err());
        assert!(raster_from_rgba(2, 2, vec![1, 2, 3]).is_err());
    }

    #[test]
    fn png_roundtrip_decodes() {
        let mut png = Vec::new();
        {
            let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
            img.write_to(
                &mut std::io::Cursor::new(&mut png),
                image::ImageFormat::Png,
            )
            .unwrap();
        }
        let data = raster_from_bytes(&png).unwrap();
        assert_eq!((data.width, data.height), (2, 2));
        assert_eq!(data.data.as_ref()[0], 10);
    }
}
