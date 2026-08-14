//! manim-cli: render demo scenes to PNG/MP4, and dump visual-check contact
//! sheets for human review of rendering correctness.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use manim_core::peniko::Color;
use manim_render::{render_video, Renderer};

#[derive(Parser)]
#[command(name = "manim", about = "manim_rust: GPU-native math animations")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render the north-star demo scene to mp4.
    Demo {
        #[arg(long, default_value = "media/demo.mp4")]
        out: PathBuf,
        #[arg(long, default_value_t = 60)]
        fps: u32,
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        /// The formula to typeset (typst math syntax).
        #[arg(long, default_value = "e^{i pi} + 1 = 0")]
        formula: String,
    },
    /// Render a single frame of the demo scene (at `time`) to PNG.
    Png {
        #[arg(long, default_value = "media/frame.png")]
        out: PathBuf,
        #[arg(long, default_value_t = 1.6)]
        time: f64,
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        #[arg(long, default_value = "e^{i pi} + 1 = 0")]
        formula: String,
    },
    /// Render every probe scene at semantic timestamps into a directory of
    /// PNGs plus an index.html contact sheet, for human visual review.
    VisualCheck {
        #[arg(long, default_value = "media/visual_check")]
        out: PathBuf,
        #[arg(long, default_value_t = 960)]
        width: u32,
        #[arg(long, default_value_t = 540)]
        height: u32,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Demo {
            out,
            fps,
            width,
            height,
            formula,
        } => {
            let scene = manim_scenes::demo(&formula);
            let mut renderer = Renderer::new(width, height, Color::from_rgba8(0, 0, 0, 255))
                .expect("GPU init failed");
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).ok();
            }
            let start = std::time::Instant::now();
            let frames = render_video(&scene.graph, &scene.timeline, &mut renderer, fps, &out)
                .expect("render failed");
            let elapsed = start.elapsed();
            println!(
                "wrote {} ({} frames, {:.2}s scene) in {:.2}s = {:.1} fps",
                out.display(),
                frames,
                scene.duration(),
                elapsed.as_secs_f64(),
                frames as f64 / elapsed.as_secs_f64()
            );
        }
        Command::Png {
            out,
            time,
            width,
            height,
            formula,
        } => {
            let scene = manim_scenes::demo(&formula);
            let mut renderer = Renderer::new(width, height, Color::from_rgba8(0, 0, 0, 255))
                .expect("GPU init failed");
            let mut sim = scene.graph.clone();
            scene.timeline.apply(&mut sim, time);
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).ok();
            }
            renderer.save_png(&mut sim, &out).expect("png render failed");
            println!("wrote {}", out.display());
        }
        Command::VisualCheck { out, width, height } => {
            visual_check(&out, width, height);
        }
    }
}

/// Render every probe at every semantic timestamp, then write an index.html
/// that presents them as labeled contact sheets for review.
fn visual_check(out: &std::path::Path, width: u32, height: u32) {
    std::fs::create_dir_all(out).expect("create output dir");
    let mut renderer =
        Renderer::new(width, height, Color::from_rgba8(0, 0, 0, 255)).expect("GPU init failed");

    let mut html = String::from(
        "<!doctype html><meta charset=utf-8><title>manim_rust visual check</title>\
         <style>body{background:#111;color:#eee;font:14px/1.4 system-ui;margin:2em}\
         h1{font-size:1.4em}h2{font-size:1.1em;margin-top:2em}\
         figure{display:inline-block;margin:.4em}img{display:block;width:320px}\
         figcaption{text-align:center;color:#aaa}</style><h1>manim_rust visual check</h1>",
    );

    for probe in manim_scenes::probes() {
        html.push_str(&format!("<h2>{}</h2><div>", probe.name));
        for &t in &probe.times {
            let mut sim = probe.scene.graph.clone();
            probe.scene.timeline.apply(&mut sim, t);
            let file = format!("{}_t{:05.2}.png", probe.name, t);
            renderer
                .save_png(&mut sim, &out.join(&file))
                .expect("png render failed");
            html.push_str(&format!(
                "<figure><img src=\"{file}\"><figcaption>t = {t:.2}s</figcaption></figure>"
            ));
        }
        html.push_str("</div>");
    }

    std::fs::write(out.join("index.html"), html).expect("write index.html");
    println!(
        "wrote visual check to {} — open {}",
        out.display(),
        out.join("index.html").display()
    );
}
