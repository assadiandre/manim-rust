//! Vello-backed renderer with frame reuse.
//!
//! Frame reuse is the invariant-2 win: when the scene graph reports no dirty
//! nodes, `render_frame` returns the previous frame's pixels without touching
//! the GPU.

use manim_core::{geometry, Camera, OrthoCamera2D, SceneGraph};
use vello::kurbo::{Affine, Stroke};
use vello::peniko::{Color, Fill, ImageBrush, ImageQuality};
use vello::wgpu;
use vello::{AaConfig, RenderParams, RendererOptions, Scene as VelloScene};

use crate::gpu::{Gpu, GpuError};

/// Effective uniform scale of an affine: sqrt(|det|), robust to rotation.
/// Used to convert device-pixel stroke widths into local space.
fn affine_scale(t: &vello::kurbo::Affine) -> f64 {
    let [a, b, c, d, _, _] = t.as_coeffs();
    (a * d - b * c).abs().sqrt().max(1e-9)
}

#[derive(Debug)]
pub enum RenderError {
    Gpu(GpuError),
    Vello(vello::Error),
    Image(image::ImageError),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Gpu(e) => write!(f, "{e}"),
            RenderError::Vello(e) => write!(f, "vello render failed: {e}"),
            RenderError::Image(e) => write!(f, "image encode failed: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<GpuError> for RenderError {
    fn from(e: GpuError) -> Self {
        RenderError::Gpu(e)
    }
}

pub struct Renderer {
    gpu: Gpu,
    vello: vello::Renderer,
    pub width: u32,
    pub height: u32,
    pub camera: OrthoCamera2D,
    pub background: Color,
    target: wgpu::Texture,
    view: wgpu::TextureView,
    staging: wgpu::Buffer,
    padded_bytes_per_row: u32,
    last_frame: Option<Vec<u8>>,
    last_camera: Option<OrthoCamera2D>,
}

impl Renderer {
    pub fn new(width: u32, height: u32, background: Color) -> Result<Self, RenderError> {
        let gpu = Gpu::headless()?;
        let vello_renderer = vello::Renderer::new(&gpu.device, RendererOptions::default())
            .map_err(RenderError::Vello)?;

        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("manim-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            // vello renders via compute (STORAGE_BINDING); we read via COPY_SRC.
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());

        // wgpu requires copy rows aligned to 256 bytes.
        let padded_bytes_per_row = (width * 4).div_ceil(256) * 256;
        let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("manim-readback"),
            size: padded_bytes_per_row as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Ok(Self {
            gpu,
            vello: vello_renderer,
            width,
            height,
            camera: OrthoCamera2D::default(),
            background,
            target,
            view,
            staging,
            padded_bytes_per_row,
            last_frame: None,
            last_camera: None,
        })
    }

    /// Render one frame, returning RGBA8 pixels (width * height * 4 bytes).
    /// Reuses the previous frame's pixels when nothing in the scene changed.
    pub fn render_frame(&mut self, scene: &mut SceneGraph) -> Result<&[u8], RenderError> {
        // Structured this way (single borrow at the end) to keep NLL happy.
        let camera_changed = self.last_camera.as_ref() != Some(&self.camera);
        let reuse = !scene.any_dirty() && !camera_changed && self.last_frame.is_some();
        if !reuse {
            let px = self.render_frame_gpu(scene)?;
            scene.clear_dirty();
            self.last_frame = Some(px);
            self.last_camera = Some(self.camera.clone());
        }
        Ok(self.last_frame.as_deref().unwrap())
    }

    pub fn save_png(
        &mut self,
        scene: &mut SceneGraph,
        path: &std::path::Path,
    ) -> Result<(), RenderError> {
        let px = self.render_frame(scene)?.to_vec();
        image::save_buffer(
            path,
            &px,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(RenderError::Image)
    }

    fn render_frame_gpu(&mut self, scene: &SceneGraph) -> Result<Vec<u8>, RenderError> {
        let camera_t = self.camera.logical_to_pixels(self.width, self.height);
        let mut vs = VelloScene::new();
        for (id, world, opacity) in scene.traverse() {
            let m = scene.get(id);
            if !m.visible || opacity <= 0.0 {
                continue;
            }
            let t = camera_t * world;
            if let Some(img) = &m.image {
                if img.width > 0 && img.height > 0 {
                    let bb = geometry::bounding_box(&m.path);
                    if bb.width() > 0.0 && bb.height() > 0.0 {
                        // Image space is y-down from the top-left; logical is y-up.
                        let local = Affine::translate((bb.x0, bb.y1))
                            * Affine::scale_non_uniform(
                                bb.width() / img.width as f64,
                                -bb.height() / img.height as f64,
                            );
                        let brush = ImageBrush::new(img.clone())
                            .with_quality(ImageQuality::Medium)
                            .with_alpha(opacity);
                        vs.draw_image(&brush, t * local);
                    }
                }
            }
            // `opacity` already includes this node's own style.opacity, so
            // use the per-paint opacities directly rather than effective_*.
            if let Some(fill) = m.style.fill {
                let c = manim_core::style::with_opacity(fill, m.style.fill_opacity * opacity);
                vs.fill(Fill::NonZero, t, c, None, &m.path);
            }
            if let Some(stroke) = m.style.stroke {
                let c =
                    manim_core::style::with_opacity(stroke, m.style.stroke_opacity * opacity);
                // Manim semantics: stroke width is in *device pixels* and
                // does not scale with mobject/camera transforms. Vello
                // strokes in local space, so divide out the transform scale.
                let device_width = m.style.stroke_width / affine_scale(&t);
                vs.stroke(&Stroke::new(device_width), t, c, None, &m.path);
            }
        }
        self.vello
            .render_to_texture(
                &self.gpu.device,
                &self.gpu.queue,
                &vs,
                &self.view,
                &RenderParams {
                    base_color: self.background,
                    width: self.width,
                    height: self.height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .map_err(RenderError::Vello)?;
        Ok(self.readback())
    }

    /// Copy the target texture to the staging buffer and block on the map.
    fn readback(&self) -> Vec<u8> {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.gpu.queue.submit([encoder.finish()]);

        let slice = self.staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = self.gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        rx.recv()
            .expect("map_async callback dropped")
            .expect("buffer map failed");

        let padded = slice.get_mapped_range();
        // Strip row padding.
        let row = (self.width * 4) as usize;
        let mut out = Vec::with_capacity(row * self.height as usize);
        for y in 0..self.height as usize {
            let start = y * self.padded_bytes_per_row as usize;
            out.extend_from_slice(&padded[start..start + row]);
        }
        drop(padded);
        self.staging.unmap();
        out
    }
}
