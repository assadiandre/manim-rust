//! Golden-image tests and the frame-reuse performance budget.
//!
//! Goldens regenerate with `UPDATE_GOLDEN=1 cargo test -p manim-render`.
//! Tolerances absorb minor GPU AA differences; layout/shape bugs blow past
//! them by orders of magnitude.

use std::path::PathBuf;

use manim_anim::{Animation, Timeline};
use manim_core::kurbo::Point;
use manim_core::peniko::Color;
use manim_core::{geometry, palette, Mobject, SceneGraph, Style};
use manim_render::{render_video, Renderer};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn assert_golden(actual: &[u8], width: u32, height: u32, name: &str) {
    let path = golden_dir().join(name);
    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(golden_dir()).unwrap();
        image::save_buffer(
            &path,
            actual,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
        eprintln!("golden written: {}", path.display());
        return;
    }
    let expected = image::open(&path).unwrap().to_rgba8();
    assert_eq!(expected.width(), width, "{name}: width mismatch");
    assert_eq!(expected.height(), height, "{name}: height mismatch");

    let mut differing = 0usize;
    let mut max_diff = 0u8;
    for (a, b) in actual.iter().zip(expected.iter()) {
        let d = a.abs_diff(*b);
        if d > 8 {
            differing += 1;
            max_diff = max_diff.max(d);
        }
    }
    let fraction = differing as f64 / actual.len() as f64;
    assert!(
        fraction < 0.001,
        "{name}: {differing} channel values differ (max {max_diff}, fraction {fraction:.5})"
    );
}

fn black() -> Color {
    Color::from_rgba8(0, 0, 0, 255)
}

#[test]
fn golden_circle() {
    let mut scene = SceneGraph::new();
    scene.add(
        Mobject::new(geometry::circle(Point::ORIGIN, 1.5))
            .with_style(Style::filled(palette::blue()).with_stroke(palette::white(), 6.0)),
    );
    let mut r = Renderer::new(256, 256, black()).unwrap();
    let px = r.render_frame(&mut scene).unwrap().to_vec();
    assert_golden(&px, 256, 256, "circle.png");
}

#[test]
fn golden_shape_gallery() {
    let mut scene = SceneGraph::new();
    scene.add(
        Mobject::new(geometry::square(Point::new(-2.0, 1.0), 1.5))
            .with_style(Style::filled(palette::red())),
    );
    scene.add(
        Mobject::new(geometry::triangle(Point::new(0.5, 1.0), 1.0))
            .with_style(Style::filled(palette::green())),
    );
    scene.add(
        Mobject::new(geometry::line(
            Point::new(-3.0, -1.5),
            Point::new(3.0, -0.5),
        ))
        .with_style(Style::default().with_stroke(palette::yellow(), 8.0)),
    );
    let mut r = Renderer::new(256, 256, black()).unwrap();
    let px = r.render_frame(&mut scene).unwrap().to_vec();
    assert_golden(&px, 256, 256, "gallery.png");
}

#[test]
fn golden_authoring_gallery() {
    let scene = manim_scenes::probes()
        .into_iter()
        .find(|p| p.name == "geometry")
        .expect("geometry probe");
    let mut sim = scene.scene.graph.clone();
    scene.scene.timeline.apply(&mut sim, 0.0);
    let mut r = Renderer::new(480, 270, black()).unwrap();
    let px = r.render_frame(&mut sim).unwrap().to_vec();
    assert_golden(&px, 480, 270, "authoring_gallery.png");
}

#[test]
fn golden_layout_and_axes() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["layout", "axes"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m22_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["tlabels", "extras"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m27() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    let scene = manim_scenes::probes()
        .into_iter()
        .find(|p| p.name == "svgraster")
        .expect("svgraster probe");
    let mut sim = scene.scene.graph.clone();
    scene.scene.timeline.apply(&mut sim, 0.0);
    let px = r.render_frame(&mut sim).unwrap().to_vec();
    assert_golden(&px, 480, 270, "svgraster.png");

    let scene = manim_scenes::probes()
        .into_iter()
        .find(|p| p.name == "restore")
        .expect("restore probe");
    for &t in &[0.0, 0.5, 1.0] {
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, t);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("restore_{t:.1}.png"));
    }
}

#[test]
fn golden_m26_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["raster", "arcp"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m25() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    let scene = manim_scenes::probes()
        .into_iter()
        .find(|p| p.name == "implicit")
        .expect("implicit probe");
    let mut sim = scene.scene.graph.clone();
    scene.scene.timeline.apply(&mut sim, 0.0);
    let px = r.render_frame(&mut sim).unwrap().to_vec();
    assert_golden(&px, 480, 270, "implicit.png");

    for (name, times) in [("wave", [0.0, 0.5, 1.0].as_slice()), ("stretch", &[0.0, 0.6, 1.2])]
    {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        for &t in times {
            let mut sim = scene.scene.graph.clone();
            scene.scene.timeline.apply(&mut sim, t);
            let px = r.render_frame(&mut sim).unwrap().to_vec();
            assert_golden(&px, 480, 270, &format!("{name}_{t:.1}.png"));
        }
    }
}

#[test]
fn golden_m24_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["svg", "boolean"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m23_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["texparts", "graph", "digraph", "tangent"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m23_anims() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for (name, times) in [("subsets", [0.0, 0.9, 1.5].as_slice()), ("cyclic", &[0.0, 0.5, 1.0])]
    {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        for &t in times {
            let mut sim = scene.scene.graph.clone();
            scene.scene.timeline.apply(&mut sim, t);
            let px = r.render_frame(&mut sim).unwrap().to_vec();
            assert_golden(&px, 480, 270, &format!("{name}_{t:.1}.png"));
        }
    }
}

#[test]
fn golden_m22_unwrite() {
    let scene = manim_scenes::probes()
        .into_iter()
        .find(|p| p.name == "unwrite")
        .expect("unwrite probe");
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for &t in &[0.0, 1.0, 2.0] {
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, t);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("unwrite_{t:.1}.png"));
    }
}

#[test]
fn golden_m21_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["lists", "bars"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m20_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["markup", "tgrid"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m19_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["curves"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m18_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["labeled"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m16_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["glabel"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m15_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["rings"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m14_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["areax", "clabels"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m13_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["field", "table"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m12_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["riemann", "dashed", "complex", "matrix", "labels"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_m11_static() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["shapes2", "angle", "polar", "title"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn golden_text_annotate_plane() {
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for name in ["text", "annotate", "plane"] {
        let scene = manim_scenes::probes()
            .into_iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} probe"));
        let mut sim = scene.scene.graph.clone();
        scene.scene.timeline.apply(&mut sim, 0.0);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, &format!("{name}.png"));
    }
}

#[test]
fn frame_reuse_meets_budget() {
    let mut scene = SceneGraph::new();
    for i in 0..1000 {
        let x = (i % 40) as f64 * 0.35 - 7.0;
        let y = (i / 40) as f64 * 0.35 - 4.0;
        scene.add(Mobject::new(geometry::circle(Point::new(x, y), 0.12)));
    }
    let mut r = Renderer::new(1920, 1080, black()).unwrap();

    let first = r.render_frame(&mut scene).unwrap().to_vec();
    let start = std::time::Instant::now();
    let second = r.render_frame(&mut scene).unwrap();
    let elapsed = start.elapsed();

    assert_eq!(&first, second, "reused frame must be pixel-identical");
    assert!(
        elapsed.as_millis() < 5,
        "frame reuse took {elapsed:?}, budget is 5ms"
    );
}

#[test]
fn dirty_scene_rerenders() {
    let mut scene = SceneGraph::new();
    let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
    let mut r = Renderer::new(128, 128, black()).unwrap();
    let a = r.render_frame(&mut scene).unwrap().to_vec();
    scene.get_mut(c).style.opacity = 0.3;
    let b = r.render_frame(&mut scene).unwrap().to_vec();
    assert_ne!(a, b, "mutation must trigger a re-render");
}

/// The demo scene pinned at semantically meaningful instants. These are the
/// same frames the `visual-check` contact sheet shows a human reviewer, so
/// "passes CI" and "looks right" can never silently drift apart again.
#[test]
fn golden_demo_phases() {
    let scene = manim_scenes::demo("e^{i pi} + 1 = 0");
    let mut r = Renderer::new(480, 270, black()).unwrap();
    for (t, name) in [
        (0.5, "demo_create_mid.png"),
        (1.6, "demo_morph_mid.png"),
        (3.45, "demo_final.png"),
    ] {
        let mut sim = scene.graph.clone();
        scene.timeline.apply(&mut sim, t);
        let px = r.render_frame(&mut sim).unwrap().to_vec();
        assert_golden(&px, 480, 270, name);
    }
}

#[test]
fn video_smoke_test() {
    let mut scene = SceneGraph::new();
    let c = scene.add(Mobject::new(geometry::circle(Point::ORIGIN, 1.0)));
    let timeline = Timeline {
        animations: vec![Animation::create(&scene, c, 1.0)],
        duration: 1.0,
        ..Timeline::default()
    };

    let mut r = Renderer::new(160, 90, black()).unwrap();
    let out = std::env::temp_dir().join("manim_rust_smoke.mp4");
    let frames = render_video(&scene, &timeline, &mut r, 12, &out).unwrap();
    assert_eq!(frames, 13);
    let size = std::fs::metadata(&out).unwrap().len();
    assert!(size > 2_000, "mp4 suspiciously small: {size} bytes");
    std::fs::remove_file(&out).ok();
}
