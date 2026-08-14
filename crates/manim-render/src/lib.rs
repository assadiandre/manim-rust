//! manim-render: headless GPU rendering for manim_rust.
//!
//! - `gpu`: wgpu device/queue without a surface
//! - `renderer`: vello scene encoding, PNG output, frame reuse
//! - `video`: ffmpeg pipe encoder + timeline render loop

pub mod gpu;
pub mod renderer;
pub mod video;

pub use renderer::{RenderError, Renderer};
pub use video::{render_video, FfmpegEncoder, VideoError};

// Downstream crates should use these re-exports to stay version-consistent
// with the renderer instead of depending on wgpu/vello directly.
pub use vello;
