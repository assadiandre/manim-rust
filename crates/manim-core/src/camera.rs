//! Camera abstraction. 2D now; the trait is the seam a 3D camera plugs into
//! later (a 3D pass renders with its own projection and composites under the
//! 2D overlay — see DESIGN.md).

use kurbo::{Affine, Point};

pub trait Camera {
    /// Transform mapping logical scene units to pixel coordinates.
    fn logical_to_pixels(&self, px_width: u32, px_height: u32) -> Affine;
}

/// Orthographic 2D camera with a Manim-compatible frame: `frame_height`
/// logical units span the image vertically, y-axis points up.
#[derive(Clone, Debug)]
pub struct OrthoCamera2D {
    pub center: Point,
    pub frame_height: f64,
}

impl Default for OrthoCamera2D {
    fn default() -> Self {
        Self {
            center: Point::ORIGIN,
            frame_height: 8.0, // Manim's default frame height
        }
    }
}

impl Camera for OrthoCamera2D {
    fn logical_to_pixels(&self, px_width: u32, px_height: u32) -> Affine {
        let scale = px_height as f64 / self.frame_height;
        let (w, h) = (px_width as f64, px_height as f64);
        // x' =  s*x + e ; y' = -s*y + f  (y flip: logical up = pixel up)
        Affine::new([
            scale,
            0.0,
            0.0,
            -scale,
            w / 2.0 - self.center.x * scale,
            h / 2.0 + self.center.y * scale,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_maps_to_image_center() {
        let cam = OrthoCamera2D::default();
        let t = cam.logical_to_pixels(1920, 1080);
        let p = t * Point::ORIGIN;
        assert!((p.x - 960.0).abs() < 1e-9);
        assert!((p.y - 540.0).abs() < 1e-9);
    }

    #[test]
    fn logical_up_is_pixel_up() {
        let cam = OrthoCamera2D::default();
        let t = cam.logical_to_pixels(1920, 1080);
        let up = t * Point::new(0.0, 1.0);
        assert!(up.y < 540.0, "positive logical y must decrease pixel y");
    }
}
