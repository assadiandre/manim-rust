//! mp4 output: raw frames piped to ffmpeg, plus the frame-loop driver.
//!
//! The driver keeps one simulation scene and re-applies the (stateless)
//! timeline each frame, so the dirty-tracking fast path kicks in whenever a
//! span of frames has no active animation.

use std::io::Write;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};

use manim_anim::Timeline;
use manim_core::SceneGraph;

use crate::renderer::{RenderError, Renderer};

#[derive(Debug)]
pub enum VideoError {
    Io(std::io::Error),
    Render(RenderError),
    FfmpegFailed(String),
}

impl std::fmt::Display for VideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VideoError::Io(e) => write!(f, "io: {e}"),
            VideoError::Render(e) => write!(f, "render: {e}"),
            VideoError::FfmpegFailed(e) => write!(f, "ffmpeg failed: {e}"),
        }
    }
}

impl std::error::Error for VideoError {}

impl From<std::io::Error> for VideoError {
    fn from(e: std::io::Error) -> Self {
        VideoError::Io(e)
    }
}

impl From<RenderError> for VideoError {
    fn from(e: RenderError) -> Self {
        VideoError::Render(e)
    }
}

/// Pipes RGBA8 frames into an ffmpeg subprocess (libx264/yuv420p).
pub struct FfmpegEncoder {
    child: Child,
    stdin: Option<ChildStdin>,
    pub width: u32,
    pub height: u32,
}

impl FfmpegEncoder {
    pub fn new(out: &Path, width: u32, height: u32, fps: u32) -> Result<Self, VideoError> {
        let mut child = Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                &format!("{width}x{height}"),
                "-r",
                &fps.to_string(),
                "-i",
                "-", // read frames from stdin
                "-an",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-crf",
                "18",
                "-movflags",
                "+faststart",
            ])
            .arg(out)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("stdin was piped");
        Ok(Self {
            child,
            stdin: Some(stdin),
            width,
            height,
        })
    }

    pub fn write_frame(&mut self, rgba: &[u8]) -> Result<(), VideoError> {
        debug_assert_eq!(rgba.len(), (self.width * self.height * 4) as usize);
        self.stdin.as_mut().expect("encoder finished").write_all(rgba)?;
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), VideoError> {
        drop(self.stdin.take());
        let out = self.child.wait_with_output()?;
        if !out.status.success() {
            return Err(VideoError::FfmpegFailed(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ));
        }
        Ok(())
    }
}

/// Render a timeline to mp4. `graph` is the scene's *final* state (the
/// `Scene` graph after all `play()` calls) — the timeline's `from` snapshots
/// restore earlier states for earlier frames.
pub fn render_video(
    graph: &SceneGraph,
    timeline: &Timeline,
    renderer: &mut Renderer,
    fps: u32,
    out: &Path,
) -> Result<usize, VideoError> {
    let frames = (timeline.duration() * fps as f64).ceil() as usize + 1;
    let mut encoder = FfmpegEncoder::new(out, renderer.width, renderer.height, fps)?;
    let mut sim = graph.clone();
    sim.clear_dirty();
    for f in 0..frames {
        let t = f as f64 / fps as f64;
        timeline.apply(&mut sim, t);
        renderer.camera = timeline.camera_at(t);
        let px = renderer.render_frame(&mut sim)?;
        encoder.write_frame(px)?;
    }
    encoder.finish()?;
    Ok(frames)
}
